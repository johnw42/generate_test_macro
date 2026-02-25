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
use syn::{FnArg, ImplItem, ItemImpl, Pat, Visibility, parse_macro_input, parse_quote};

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
    /// Any `#[cfg(...)]` attributes on the original method, propagated verbatim
    /// onto the generated wrapper function.
    cfg_attrs: Vec<syn::Attribute>,
}

#[cfg(feature = "quickcheck")]
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
    let mut new_param_names: Vec<Ident> = Vec::new();
    let mut test_methods: Vec<TestMethod> = Vec::new();
    #[cfg(feature = "quickcheck")]
    let mut quickcheck_methods: Vec<QuickcheckMethod> = Vec::new();

    for impl_item in &mut impl_block.items {
        let ImplItem::Fn(method) = impl_item else {
            continue;
        };

        let method_name = method.sig.ident.clone();
        let is_new = method_name == "new";

        // Detect and strip special attributes.
        let mut is_test = false;
        let mut is_quickcheck = false;
        method.attrs.retain(|attr| {
            if attr.path().is_ident("test") {
                is_test = true;
                false
            } else if attr.path().is_ident("quickcheck") {
                is_quickcheck = true;
                false
            } else {
                true
            }
        });

        if is_new || is_test || is_quickcheck {
            method.vis = Visibility::Public(Default::default());
            method.attrs.insert(0, parse_quote!(#[doc(hidden)]));
        }

        if is_new {
            for input in &method.sig.inputs {
                if let FnArg::Typed(pt) = input {
                    if let Pat::Ident(pi) = pt.pat.as_ref() {
                        new_param_names.push(pi.ident.clone());
                    }
                }
            }
        } else if is_test {
            let cfg_attrs = method
                .attrs
                .iter()
                .filter(|a| a.path().is_ident("cfg"))
                .cloned()
                .collect();
            let has_self = method
                .sig
                .inputs
                .first()
                .map(|arg| matches!(arg, FnArg::Receiver(_)))
                .unwrap_or(false);
            test_methods.push(TestMethod {
                name: method_name,
                has_self,
                cfg_attrs,
            });
        } else if is_quickcheck {
            #[cfg(feature = "quickcheck")]
            {
                let qm = build_quickcheck_method(method, &type_params);
                quickcheck_methods.push(qm);
            }
            let _ = (&is_quickcheck, &method_name, &type_params);
        }
    }

    // ------------------------------------------------------------------
    // If there is nothing to generate, just return the transformed impl.
    // ------------------------------------------------------------------
    let has_test_methods = !test_methods.is_empty();
    // `new` params are only needed in the macro pattern when at least one
    // #[test] method takes a `self` receiver.
    let has_self_test_methods = test_methods.iter().any(|tm| tm.has_self);
    #[cfg(feature = "quickcheck")]
    let has_quickcheck_methods = !quickcheck_methods.is_empty();
    #[cfg(not(feature = "quickcheck"))]
    let has_quickcheck_methods = false;

    if !has_test_methods && !has_quickcheck_methods {
        return quote! { #impl_block }.into();
    }

    // ------------------------------------------------------------------
    // Build the macro_rules! pattern:
    //
    //   ($mod_name:ident $(, $T:ty)* $(, $param:expr)*)
    //
    // The `, $param:expr` pieces are only present when #[test] methods exist
    // (they need `Self::new(…)` to be called).
    // ------------------------------------------------------------------

    let pat_mod_name = {
        let dv = dollar_ident("mod_name");
        quote! { #dv : ident }
    };

    let pat_type_params: TokenStream2 = type_params
        .iter()
        .map(|tp| {
            let dv = dollar_ident(&tp.to_string());
            quote! { , #dv : ty }
        })
        .collect();

    let pat_new_params: TokenStream2 = if has_self_test_methods {
        new_param_names
            .iter()
            .map(|p| {
                let dv = dollar_ident(&p.to_string());
                quote! { , #dv : expr }
            })
            .collect()
    } else {
        TokenStream2::new()
    };

    // ------------------------------------------------------------------
    // Build re-usable snippets for the macro expansion body.
    // ------------------------------------------------------------------

    let dollar_mod_name = dollar_ident("mod_name");

    // `StructName::<$T, …>` – unqualified; the struct must be in scope at the
    // macro invocation site (e.g. via `use super::*` or an explicit `use`).
    let type_path_args: TokenStream2 = if type_params.is_empty() {
        TokenStream2::new()
    } else {
        let args: TokenStream2 = type_params
            .iter()
            .enumerate()
            .map(|(i, tp)| {
                let dv = dollar_ident(&tp.to_string());
                if i == 0 {
                    quote! { #dv }
                } else {
                    quote! { , #dv }
                }
            })
            .collect();
        quote! { :: < #args > }
    };

    // `( $param1 , $param2 , … )` for the `new` call
    let new_call_args: TokenStream2 = {
        let args: TokenStream2 = new_param_names
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let dv = dollar_ident(&p.to_string());
                if i == 0 {
                    quote! { #dv }
                } else {
                    quote! { , #dv }
                }
            })
            .collect();
        quote! { ( #args ) }
    };

    // ------------------------------------------------------------------
    // Emit `#[test]` wrappers.
    // ------------------------------------------------------------------
    let test_fn_items: TokenStream2 = test_methods
        .iter()
        .map(|tm| {
            let name = &tm.name;
            let cfg_attrs = &tm.cfg_attrs;
            let call = if tm.has_self {
                quote! {
                    #struct_name #type_path_args
                        :: new #new_call_args . #name ();
                }
            } else {
                quote! {
                    #struct_name #type_path_args :: #name ();
                }
            };
            quote! {
                #(#cfg_attrs)*
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
    #[cfg(feature = "quickcheck")]
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
                        #struct_name #type_path_args
                            :: #name as fn( #underscores ) -> _
                    );
                }
            }
        })
        .collect();
    #[cfg(not(feature = "quickcheck"))]
    let quickcheck_fn_items: TokenStream2 = TokenStream2::new();

    // ------------------------------------------------------------------
    // Assemble the final output.
    // ------------------------------------------------------------------
    let macro_rules_def = quote! {
        #[macro_export]
        macro_rules! #macro_name {
            ( #pat_mod_name #pat_type_params #pat_new_params ) => {
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
// Quickcheck method builder (compiled only with the "quickcheck" feature)
// ---------------------------------------------------------------------------

#[cfg(feature = "quickcheck")]
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
