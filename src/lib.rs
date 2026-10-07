#![doc = include_str!("../README.md")]
use std::collections::HashMap;

use proc_macro::TokenStream;
use proc_macro2::{Ident, Punct, Spacing, Span, TokenStream as TokenStream2, TokenTree};
use quote::quote;
use syn::{
    parse_macro_input, parse_quote, punctuated::Punctuated, FnArg, ImplItem, ItemImpl, Pat,
    Visibility,
};

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
            .expect("generate_test_macro: expected at least one path segment in self type")
            .ident
            .clone(),
        _ => panic!("generate_test_macro: self type must be a path (e.g. `Struct<T>`)"),
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

struct ProptestMethod {
    name: Ident,
    args: Vec<syn::FnArg>,
    param_exprs: HashMap<String, syn::Expr>,
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
/// #[generate_test_macro(my_suite_tests)]
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
pub fn generate_test_macro(attr: TokenStream, item: TokenStream) -> TokenStream {
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
    let mut proptest_methods: Vec<ProptestMethod> = Vec::new();

    for impl_item in &mut impl_block.items {
        let ImplItem::Fn(method) = impl_item else {
            continue;
        };

        let method_name = method.sig.ident.clone();

        // Detect and strip special attributes.
        let mut is_test = false;
        let mut is_quickcheck = false;
        let mut proptest_attr = None;
        let mut extra_attrs: Vec<syn::Attribute> = Vec::new();
        method.attrs.retain(|attr| {
            if attr.path().is_ident("test") {
                is_test = true;
                false
            } else if attr.path().is_ident("quickcheck") {
                is_quickcheck = true;
                false
            } else if attr.path().is_ident("proptest") {
                proptest_attr = Some(attr.clone());
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

        if is_test || is_quickcheck || proptest_attr.is_some() {
            method.vis = Visibility::Public(Default::default());
            method.attrs.push(parse_quote!(#[doc(hidden)]));
            method.attrs.push(parse_quote!(#[inline(never)]));
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
            let qm = build_quickcheck_method(method);
            quickcheck_methods.push(qm);
        } else if let Some(attr) = proptest_attr {
            match build_proptest_method(method, attr) {
                Ok(pm) => proptest_methods.push(pm),
                Err(error) => return error,
            }
        }
    }

    // ------------------------------------------------------------------
    // If there is nothing to generate, just return the transformed impl.
    // ------------------------------------------------------------------
    let has_test_methods = !test_methods.is_empty();
    let has_self_test_methods = test_methods.iter().any(|tm| tm.has_self)
        || proptest_methods.iter().any(|pm| {
            pm.args
                .first()
                .is_some_and(|arg| matches!(arg, FnArg::Receiver(_)))
        });
    let has_quickcheck_methods = !quickcheck_methods.is_empty();
    let has_proptest_methods = !proptest_methods.is_empty();

    if !has_test_methods && !has_quickcheck_methods && !has_proptest_methods {
        return quote! {
            compile_error!("generate_test_macro: the impl block must contain at least one #[test], #[quickcheck], or #[proptest] method");
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

    let proptest_fn_items: TokenStream2 = proptest_methods
        .iter()
        .map(|pm| {
            let name = &pm.name;
            let cfg_attrs = &pm.cfg_attrs;
            let has_self = pm
                .args
                .first()
                .is_some_and(|arg| matches!(arg, syn::FnArg::Receiver(_)));
            let call_args: Punctuated<Ident, syn::Token![,]> = pm
                .args
                .iter()
                .filter(|arg| matches!(arg, FnArg::Typed(_)))
                .enumerate()
                .map(|(i, _)| Ident::new(&format!("a{i}"), Span::call_site()))
                .collect();
            let mut strategy_args: Punctuated<TokenStream2, syn::Token![,]> = pm
                .args
                .iter()
                .filter_map(|arg| match arg {
                    FnArg::Typed(arg) => Some(arg),
                    FnArg::Receiver(_) => None,
                })
                .zip(call_args.iter())
                .map(|(arg, name)| {
                    let strategy = match &*arg.pat {
                        Pat::Ident(pat) => pm.param_exprs.get(&pat.ident.to_string()),
                        _ => None,
                    };
                    if let Some(strategy) = strategy {
                        quote!(#name in #strategy)
                    } else {
                        let ty = &arg.ty;
                        quote!(#name: #ty)
                    }
                })
                .collect();
            // proptest! requires at least one strategy, even for receiver-only methods.
            if strategy_args.is_empty() {
                strategy_args.push(quote!(_ in ::proptest::strategy::Just(())));
            }
            let call = if has_self {
                quote! {
                    #[allow(unused_mut)]
                    let mut instance : #dollar_type = #dollar_expr;
                    instance . #name (#call_args)?;
                }
            } else {
                quote! {
                    < #dollar_type > :: #name (#call_args)?;
                }
            };
            quote! {
                ::proptest::proptest! {
                    #(#cfg_attrs)*
                    #[test]
                    fn #name(#strategy_args) {
                        #call
                    }
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
        let has_static_test_methods = test_methods.iter().any(|tm| !tm.has_self)
            || !quickcheck_methods.is_empty()
            || proptest_methods.iter().any(|pm| {
                !pm.args
                    .first()
                    .is_some_and(|arg| matches!(arg, FnArg::Receiver(_)))
            });
        let plain_arm: TokenStream2 = if type_params.is_empty() || !has_static_test_methods {
            let wildcard_params = if type_params.is_empty() {
                quote! {}
            } else {
                let wildcards = type_params
                    .iter()
                    .fold(quote! {}, |acc, _| quote! { #acc _, });
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
                    #proptest_fn_items
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

#[deprecated(
    since = "0.1.2",
    note = "The `#[test_suite_macro]` attribute is now named `#[generate_test_macro]`"
)]
#[proc_macro_attribute]
pub fn test_suite_macro(attr: TokenStream, item: TokenStream) -> TokenStream {
    generate_test_macro(attr, item)
}

// ---------------------------------------------------------------------------
// Quickcheck method builder
// ---------------------------------------------------------------------------

fn build_quickcheck_method(method: &syn::ImplItemFn) -> QuickcheckMethod {
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

fn build_proptest_method(
    method: &mut syn::ImplItemFn,
    proptest_attr: syn::Attribute,
) -> Result<ProptestMethod, TokenStream> {
    if let syn::ReturnType::Type(..) = &method.sig.output {
        return Err(syn::Error::new_spanned(
            &method.sig.output,
            "generate_test_macro: #[proptest] methods must not specify a return type",
        )
        .to_compile_error()
        .into());
    }
    let name = method.sig.ident.clone();
    let param_exprs = parse_proptest_attr(proptest_attr)?;
    method.sig.output = parse_quote!(-> ::proptest::test_runner::TestCaseResult);
    if let Some(syn::Stmt::Expr(_, semi)) = method.block.stmts.last_mut() {
        *semi = Some(Default::default());
    }
    method
        .block
        .stmts
        .push(syn::Stmt::Expr(parse_quote!(Ok(())), None));
    let cfg_attrs = method
        .attrs
        .iter()
        .filter(|a| a.path().is_ident("cfg"))
        .cloned()
        .collect();
    Ok(ProptestMethod {
        name,
        args: method.sig.inputs.iter().cloned().collect(),
        param_exprs,
        cfg_attrs,
    })
}

// The attribute should have the form
//   #[proptest(param1 in expr1, param2 in expr2, ...)]
// where the params are identifiers and the expressions are instances of `syn::Expr`
//
// Alternatively, simply #[proptest] is acceptable and indicates no parameters.
//
// Produces a TokenStream emitting a compilation error if the attribute is malformed.
fn parse_proptest_attr(attr: syn::Attribute) -> Result<HashMap<String, syn::Expr>, TokenStream> {
    if matches!(attr.meta, syn::Meta::Path(_)) {
        return Ok(HashMap::new());
    }

    let parsed = attr.parse_args_with(|input: syn::parse::ParseStream| {
        let mut param_exprs = HashMap::new();

        while !input.is_empty() {
            let param: Ident = input.parse()?;
            input.parse::<syn::Token![in]>()?;
            let expr: syn::Expr = input.parse()?;

            let name = param.to_string();
            if param_exprs.insert(name.clone(), expr).is_some() {
                return Err(syn::Error::new(
                    param.span(),
                    format!("duplicate proptest parameter `{name}`"),
                ));
            }

            if !input.is_empty() {
                input.parse::<syn::Token![,]>()?;
            }
        }

        Ok(param_exprs)
    });

    parsed.map_err(|error| error.to_compile_error().into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bare_proptest_attr_has_no_parameters() {
        let attr = parse_quote!(#[proptest]);
        assert!(parse_proptest_attr(attr).unwrap().is_empty());
    }

    #[test]
    fn empty_proptest_attr_has_no_parameters() {
        let attr = parse_quote!(#[proptest()]);
        assert!(parse_proptest_attr(attr).unwrap().is_empty());
    }

    #[test]
    fn proptest_attr_parses_expressions_with_trailing_comma() {
        let attr = parse_quote!(#[proptest(a in 0..10, b in strategy(1, 2),)]);
        let params = parse_proptest_attr(attr).unwrap();

        assert_eq!(params.len(), 2);
        assert!(matches!(params["a"], syn::Expr::Range(_)));
        assert!(matches!(params["b"], syn::Expr::Call(_)));
    }
}
