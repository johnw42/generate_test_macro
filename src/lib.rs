//! A proc macro for writing a suite of unit tests as methods on a struct.
//!
//! Attach `#[test_suite_macro(name)]` to an `impl` block to produce:
//!
//! 1. The same `impl` block with `new`, `#[test]`, and `#[quickcheck]` methods
//!    made `pub` / `#[doc(hidden)]` (and their special attributes stripped).
//! 2. A `macro_rules! name { … }` that, when invoked with a module name,
//!    concrete type arguments, and (if needed) constructor arguments, creates
//!    an isolated test module.
//!
//! See the project README for full examples.

use proc_macro::TokenStream;
use proc_macro2::{Ident, Punct, Spacing, Span, TokenStream as TokenStream2, TokenTree};
use quote::quote;
use syn::{FnArg, ImplItem, ItemImpl, Visibility, parse_macro_input, parse_quote};

// ---------------------------------------------------------------------------
// Token-stream helpers
// ---------------------------------------------------------------------------

/// Produces `$name` as two tokens (a literal `$` punct followed by an ident),
/// suitable for embedding inside a generated `macro_rules!` definition.
fn dollar_ident(name: &str) -> TokenStream2 {
    let mut ts = TokenStream2::new();
    ts.extend([
        TokenTree::Punct(Punct::new('$', Spacing::Alone)),
        TokenTree::Ident(Ident::new(name, Span::call_site())),
    ]);
    ts
}

/// Extract the last path segment ident from a `syn::Type`.
/// E.g. `TestSuite<T>` → `TestSuite`.
fn extract_struct_name(ty: &syn::Type) -> Ident {
    match ty {
        syn::Type::Path(type_path) => type_path
            .path
            .segments
            .last()
            .expect("test_suite_macro: expected at least one path segment in self type")
            .ident
            .clone(),
        _ => panic!("test_suite_macro: self type must be a path (e.g. `Struct<T>`)"),
    }
}

// ---------------------------------------------------------------------------
// Collected information from analysing the impl block
// ---------------------------------------------------------------------------

struct TestMethod {
    name: Ident,
    /// Whether the method takes a `self` / `&self` / `&mut self` receiver.
    /// When `true` the generated wrapper calls `Struct::new(...).method()`;
    /// when `false` it calls `Struct::method()` directly.
    has_self: bool,
    /// Attributes from the original method that should be propagated verbatim
    /// onto the generated wrapper function: `#[cfg(...)]`, `#[should_panic]`,
    /// and `#[ignore]`.
    extra_attrs: Vec<syn::Attribute>,
}

struct QuickcheckMethod {
    name: Ident,
    /// Number of non-self parameters; used to build the `fn(_, ...) -> _` cast.
    arity: usize,
    /// Any `#[cfg(...)]` attributes on the original method, propagated verbatim
    /// onto the generated wrapper function.
    cfg_attrs: Vec<syn::Attribute>,
}

// ---------------------------------------------------------------------------
// Proc-macro entry point
// ---------------------------------------------------------------------------

/// Generate a `macro_rules!` test harness from an `impl` block.
///
/// # Example
///
/// ```rust,ignore
/// #[test_suite_macro(my_suite_tests)]
/// impl<T: MyTrait> MySuite<T> {
///     fn new(arg: usize) -> Self { Self { arg } }
///
///     #[test]
///     fn it_works(&self) { /* … */ }
/// }
///
/// // In a consumer crate (MySuite must be in scope at the invocation site):
/// use my_crate::my_suite::MySuite;
/// my_suite_tests!(for_concrete_impl, ConcreteType, 42);
/// ```
#[proc_macro_attribute]
pub fn test_suite_macro(attr: TokenStream, item: TokenStream) -> TokenStream {
    let macro_name = parse_macro_input!(attr as Ident);
    let mut impl_block = parse_macro_input!(item as ItemImpl);

    // ------------------------------------------------------------------
    // Collect generic type-parameter names (e.g. `T` in `impl<T: Trait>`).
    // ------------------------------------------------------------------
    let type_params: Vec<Ident> = impl_block
        .generics
        .type_params()
        .map(|tp| tp.ident.clone())
        .collect();

    let struct_name = extract_struct_name(&impl_block.self_ty);

    // ------------------------------------------------------------------
    // Walk the impl items, categorising and transforming each method.
    // ------------------------------------------------------------------
    let mut test_methods: Vec<TestMethod> = Vec::new();
    let mut quickcheck_methods: Vec<QuickcheckMethod> = Vec::new();

    for impl_item in &mut impl_block.items {
        let ImplItem::Fn(method) = impl_item else {
            continue;
        };

        let method_name = method.sig.ident.clone();

        // Detect and strip special attributes.
        let mut is_test = false;
        let mut is_quickcheck = false;
        let mut extra_attrs: Vec<syn::Attribute> = Vec::new();
        method.attrs.retain(|attr| {
            if attr.path().is_ident("test") {
                is_test = true;
                false
            } else if attr.path().is_ident("quickcheck") {
                is_quickcheck = true;
                false
            } else if attr.path().is_ident("should_panic") || attr.path().is_ident("ignore") {
                // Collect for propagation onto the generated #[test] wrapper,
                // but strip from the inherent method to avoid an "unused
                // attribute" compiler warning on the impl block.
                extra_attrs.push(attr.clone());
                false
            } else if attr.path().is_ident("cfg") {
                extra_attrs.push(attr.clone());
                true
            } else {
                true
            }
        });

        if is_test || is_quickcheck {
            method.vis = Visibility::Public(Default::default());
            method.attrs.insert(0, parse_quote!(#[doc(hidden)]));
        }

        if is_test {
            let has_self = method
                .sig
                .inputs
                .first()
                .map(|arg| matches!(arg, FnArg::Receiver(_)))
                .unwrap_or(false);
            test_methods.push(TestMethod {
                name: method_name,
                has_self,
                extra_attrs,
            });
        } else if is_quickcheck {
            let qm = build_quickcheck_method(method, &type_params);
            quickcheck_methods.push(qm);
            let _ = (&is_quickcheck, &method_name, &type_params);
        }
    }

    // ------------------------------------------------------------------
    // If there is nothing to generate, just return the transformed impl.
    // ------------------------------------------------------------------
    let has_test_methods = !test_methods.is_empty();
    let has_self_test_methods = test_methods.iter().any(|tm| tm.has_self);
    let has_quickcheck_methods = !quickcheck_methods.is_empty();

    if !has_test_methods && !has_quickcheck_methods {
        return quote! {
            compile_error!("test_suite_macro: the impl block must contain at least one #[test] or #[quickcheck] method");
            #impl_block
        }
        .into();
    }

    // ------------------------------------------------------------------
    // Dollar-metavariable helpers used inside the generated macro body.
    // ------------------------------------------------------------------
    let dollar_mod_name = dollar_ident("mod_name");
    let dollar_type = dollar_ident("type");
    let dollar_expr = dollar_ident("expr");

    // ------------------------------------------------------------------
    // Emit `#[test]` wrappers.
    //
    // Each wrapper references $type (and $expr for instance methods), which
    // are macro metavariables supplied by the caller at invocation time.
    // ------------------------------------------------------------------
    let test_fn_items: TokenStream2 = test_methods
        .iter()
        .map(|tm| {
            let name = &tm.name;
            let extra_attrs = &tm.extra_attrs;
            let call = if tm.has_self {
                quote! {
                    #[allow(unused_mut)]
                    let mut instance : #dollar_type = #dollar_expr;
                    instance . #name ();
                }
            } else {
                quote! {
                    < #dollar_type > :: #name ();
                }
            };
            quote! {
                #(#extra_attrs)*
                #[test]
                fn #name () {
                    #call
                }
            }
        })
        .collect();

    // ------------------------------------------------------------------
    // Emit `#[quickcheck]` wrappers (only populated when feature is active).
    // Each wrapper is a plain `#[test]` that calls `quickcheck::quickcheck`
    // with a function-pointer cast to drive shrinking and randomisation.
    // ------------------------------------------------------------------
    let quickcheck_fn_items: TokenStream2 = quickcheck_methods
        .iter()
        .map(|qm| {
            let name = &qm.name;
            let cfg_attrs = &qm.cfg_attrs;
            // Build `_, _, …` with qm.arity underscores for the fn-ptr cast.
            let underscores: TokenStream2 = (0..qm.arity)
                .enumerate()
                .map(|(i, _)| {
                    if i == 0 {
                        quote! { _ }
                    } else {
                        quote! { , _ }
                    }
                })
                .collect();
            quote! {
                #(#cfg_attrs)*
                #[test]
                pub fn #name() {
                    quickcheck::quickcheck(
                        < #dollar_type > :: #name as fn( #underscores ) -> _
                    );
                }
            }
        })
        .collect();

    // ------------------------------------------------------------------
    // Build the macro_rules! arms.
    //
    // Calling convention (README):
    //   macro_name!($mod_name: $type = $instance_expr)
    //
    // The primary arm pattern depends on whether any test method takes self:
    //   • with instance methods:  ($mod_name:ident : $type:ty = $expr:expr)
    //   • static / quickcheck only: ($mod_name:ident : $type:ty)
    //
    // Supporting shorthand arms (generated via string parsing, since
    // macro repetition syntax cannot be expressed with quote!):
    //   • default arm:    ($mod_name:ident : $type:ty)
    //                       → delegates using Default::default()
    //   • turbofish arm:  ($mod_name:ident = StructName::<T,...> rest...)
    //                       → infers type from expression
    //   • plain abbrev:   ($mod_name:ident = StructName rest...)
    //                       → infers type from expression
    // ------------------------------------------------------------------
    let (main_pat, supporting_arms) = if has_self_test_methods {
        let pat = quote! {
            #dollar_mod_name : ident : #dollar_type : ty = #dollar_expr : expr
        };

        // Default arm: omit expr, construct via Default::default().
        let default_arm = quote! {
            ($mod_name:ident : $type:ty) => {
                #macro_name!($mod_name : $type = <$type as ::core::default::Default>::default());
            };
        };

        // Abbreviated turbofish arm: infer type from StructName::<T, …> expr.
        let turbofish_arm = quote! {
            ($mod_name:ident = #struct_name :: <$($tparam:ty),* $(,)?> $($rest:tt)*) => {
                #macro_name!($mod_name : #struct_name<$($tparam),*> = #struct_name::<$($tparam),*> $($rest)*);
            };
        };

        // Abbreviated plain arm: infer type from StructName expr.
        let has_static_test_methods =
            test_methods.iter().any(|tm| !tm.has_self) || !quickcheck_methods.is_empty();
        let plain_arm: TokenStream2 = if type_params.is_empty() || !has_static_test_methods {
            let wildcard_params = if type_params.is_empty() {
                quote! {}
            } else {
                let wildcards = type_params
                    .iter()
                    .map(|_| quote! { _ })
                    .collect::<Vec<_>>()
                    .into_iter()
                    .fold(quote! {}, |acc, wc| quote! { #acc #wc });
                quote! { < #wildcards > }
            };
            quote! {
                ($mod_name:ident = #struct_name $($rest:tt)*) => {
                    #macro_name!($mod_name : #struct_name #wildcard_params = #struct_name $($rest)*);
                };
            }
        } else {
            quote! {
                ($mod_name:ident = #struct_name $($rest:tt)*) => {
                    compile_error!(concat!(stringify!(#macro_name), ": type parameters for ",
                     stringify!(#struct_name), " cannot be inferred; use the turbofish form instead"));
                };
            }
        };

        let arms = quote! {
            #default_arm
            #turbofish_arm
            #plain_arm
        };
        (pat, arms)
    } else {
        // Static / quickcheck-only: caller supplies only the type.
        let pat = quote! {
            #dollar_mod_name : ident : #dollar_type : ty
        };
        (pat, TokenStream2::new())
    };

    // ------------------------------------------------------------------
    // Assemble the final output.
    // ------------------------------------------------------------------
    let macro_rules_def = quote! {
        #[macro_export]
        macro_rules! #macro_name {
            #supporting_arms
            ( #main_pat ) => {
                mod #dollar_mod_name {
                    #[allow(unused_imports)]
                    use super::*;
                    #test_fn_items
                    #quickcheck_fn_items
                }
            }
        }
    };

    quote! {
        #impl_block
        #macro_rules_def
    }
    .into()
}

// ---------------------------------------------------------------------------
// Quickcheck method builder
// ---------------------------------------------------------------------------

fn build_quickcheck_method(method: &syn::ImplItemFn, _type_params: &[Ident]) -> QuickcheckMethod {
    let name = method.sig.ident.clone();
    let arity = method
        .sig
        .inputs
        .iter()
        .filter(|arg| matches!(arg, FnArg::Typed(_)))
        .count();
    let cfg_attrs = method
        .attrs
        .iter()
        .filter(|a| a.path().is_ident("cfg"))
        .cloned()
        .collect();
    QuickcheckMethod {
        name,
        arity,
        cfg_attrs,
    }
}
