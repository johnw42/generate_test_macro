//! A proc macro for writing a suite of unit tests as methods or associated
//! functions on a type.  This is useful for running the same suite of tests on
//! multiple instances of the type, and for writing `proptest` tests without
//! putting all test code inside a macro call.
//!
//! # Motivation and Examples
//!
//! Consider trying to test the following function, which uses polymorphism to
//! be generic over different number types.  You'd like to test it with various
//! integer types.
//!
//! ```rust
//! # use std::ops::{Add, Div, Mul};
//! /// Computes the sum of integers from `0..=n` using the Gaussian formula.
//! fn gauss_sum<N>(n: N) -> N
//! where
//!     N: From<u8> + Add<Output = N> + Div<Output = N> + Mul<Output = N> + Copy,
//! {
//!     n * (n + 1.into()) / 2.into()
//! }
//! ```
//!
//! ## Naive Macro
//!
//! A naive approach to generating multiple tests is to define a macro, like
//! this:
//!
//! ```rust,no_run
//! macro_rules! gauss_test {
//!     ($name:ident, $ty:ty, $n:expr) => {
//!         #[test]
//!         fn $name() {
//!             let expected: $ty = (0..=$n).into_iter().sum();
//!             let actual: $ty = gauss_sum($n);
//!             assert_eq!(expected, actual);
//!         }
//!     };
//! }
//!
//! gauss_test!(for_u8, u8, 10);
//! gauss_test!(for_i32, i32, 1024);
//! // ...repeat for more integer types
//! ```
//!
//! This works, but it has a severe drawback: all of your test logic lives
//! inside a `macro_rules!` definition, meaning you'll have to format it
//! manually and you can't get help from your IDE for it.  This can be very
//! annoying when writing large, complicated tests!  Another less obvious
//! drawback is that when errors are reported in the test, they are reported at
//! the location of the macro invocation rather than the location of the test
//! code, making it hard to track down the source of the error.
//!
//! ## Moving Test Logic Out Of Macros
//!
//! Using `generate_test_macro` allows you to move your test logic out of the
//! macro and into regular Rust functions, while still generating the necessary
//! test wrappers automatically.
//!
//! ```rust,no_run
//! # use std::ops::{Add, Div, Mul};
//! # fn gauss_sum<N>(n: N) -> N { todo!() }
//! # use generate_test_macro::generate_test_macro;
//! # use std::fmt::Debug;
//! # use std::iter::Sum;
//! # use std::ops::RangeInclusive;
//! struct GaussTest<N>(N);
//!
//! #[generate_test_macro(basic_tests)]
//! impl<N> GaussTest<N>
//! where
//!     N: From<u8> + Add<Output = N> + Div<Output = N> + Mul<Output = N> + Copy,
//!     N: Debug + Eq + Sum,
//!     RangeInclusive<N>: IntoIterator<Item = N>,
//! {
//!     #[test]
//!     fn test_sum(&self) {
//!         // Test logic in non-macro code!
//!         let expected: N = (0.into()..=self.0).into_iter().sum();
//!         let actual: N = gauss_sum(self.0);
//!         assert_eq!(expected, actual);
//!     }
//! }
//!
//! basic_tests!(for_u8 = GaussTest::<u8>(10));
//! basic_tests!(for_i32 = GaussTest::<i32>(1024));
//! ```
//!
//! ## More Thorough Testing
//!
//! Testing single input values doesn't test our code very thoroughly.  To have
//! more confidence in our implementation, we should test a wider range of
//! inputs, for which we can use a property-testing framework like `quickcheck`.
//! Support for `quickcheck` is build into `generate_test_macro!`, so you can
//! write a test like this.
//!
//! ```rust,no_run
//! # use std::ops::{Add, Div, Mul};
//! # fn gauss_sum<N>(n: N) -> N { todo!() }
//! # use generate_test_macro::generate_test_macro;
//! # use quickcheck::TestResult;
//! # use std::fmt::Debug;
//! # use std::iter::Sum;
//! # use std::marker::PhantomData;
//! # use std::ops::RangeInclusive;
//! struct GaussTest<N, const LIMIT: usize>(PhantomData<N>);
//!
//! #[generate_test_macro(quickcheck_tests)]
//! impl<N, const LIMIT: usize> GaussTest<N, LIMIT>
//! where
//!     N: From<u8> + Add<Output = N> + Div<Output = N> + Mul<Output = N> + Copy,
//!     N: Debug + Eq + Sum + TryFrom<usize>,
//!     usize: From<N>,
//!     <N as TryFrom<usize>>::Error: Debug,
//!     RangeInclusive<N>: IntoIterator<Item = N>,
//! {
//!     #[quickcheck]
//!     fn test_sum(arb: N) -> TestResult {
//!         // Discard the test if the conversion to usize fails
//!         let Ok(arb_usize) = usize::try_from(arb) else {
//!             return TestResult::discard();
//!         };
//!         // Generate a number in the range 0..=LIMIT
//!         let n: N = (arb_usize % LIMIT).try_into().unwrap();
//!         let expected: N = (0.into()..=n).into_iter().sum();
//!         let actual: N = gauss_sum(n);
//!         assert_eq!(expected, actual);
//!         TestResult::passed()
//!     }
//! }
//!
//! quickcheck_tests!(for_u8: GaussTest<u8, 10>);
//! quickcheck_tests!(for_i32: GaussTest<i32, 1024>);
//! ```
//!
//! Some things are worth noting in this example.  First, the test method no
//! longer takes `&self` as a parameter, which is necessary because of
//! limitations in `quickcheck`.  Second, the invocations of `quickcheck_tests!`
//! specify a type rather than a value of the `GaussTest` struct; this works
//! because no test methods in `GaussTest` take `&self` as a parameter.  Instead
//! of a `self` parameter, the test method makes an arbitrary value provided by
//! the testing framework, and in general, tests built with `quickcheck` or
//! `proptest` *must* take parameters other than `&self`.
//!
//! Going more deeply, this test has some clear drawbacks.  The test logic has
//! become more complicated because it has to handle negative numbers generated
//! by `quickcheck`, and we have to jump through some hoops to specify the limit
//! of the numbers we'll pass to `gauss_sum`, and this logic leads to a lot of
//! boilerplate.
//!
//! ## Gaining Flexibility and Simplicity with Proptest
//!
//! Rather than using `quickcheck` with its limitations and boilerplate, we can
//! use `proptest` to generate test cases in a more flexible and concise manner.
//! Unlike `quickcheck`, which relies on its `Arbitrary` trait to generate test
//! cases, `proptest` uses strategies that give you more control over the
//! generated values.  Although `proptest` has its own `Arbitrary` trait,
//! `generate_test_macro` makes it possible to specify a `proptest` strategy for
//! each test parameter, only falling back to `Arbitrary` when no strategy is
//! specified.
//!
//! Here is a version of the tests using `propcheck`.
//!
//! ```rust
//! # use std::ops::{Add, Div, Mul};
//! # fn gauss_sum<N>(n: N) -> N { todo!() }
//! # use generate_test_macro::generate_test_macro;
//! # use std::fmt::Debug;
//! # use std::iter::Sum;
//! # use std::ops::RangeInclusive;
//! # use proptest::prelude::*;
//! struct GaussTest<N>(RangeInclusive<N>);
//!
//! #[generate_test_macro(propcheck_tests)]
//! impl<N> GaussTest<N>
//! where
//!     N: From<u8> + Add<Output = N> + Div<Output = N> + Mul<Output = N> + Copy,
//!     N: Debug + Eq + Sum,
//!     RangeInclusive<N>: IntoIterator<Item = N> + Strategy<Value = N>,
//! {
//!     #[proptest(arb in self.0.clone())]
//!     fn test_sum(&self, arb: N) {
//!         let expected: N = (0.into()..=arb).into_iter().sum();
//!         let actual: N = gauss_sum(arb);
//!         prop_assert_eq!(expected, actual);
//!     }
//! }
//!
//! propcheck_tests!(for_u8 = GaussTest::<u8>(0..=10));
//! propcheck_tests!(for_i32 = GaussTest::<i32>(0..=1024));
//! ```
//!
//! In this example, we're back to using a `&self` parameter in the test method,
//! and the `GaussTest` struct encapsulates the range for generating test cases,
//! which `proptest` uses as a strategy for generating test inputs.  Each call
//! to `propcheck_tests!` specifies a range of values to test so we don't have
//! to worry about negative inputs anymore.  Notice that the strategy specified
//! for the `arb` parameter refers to `self`, which is allowed in `proptest`
//! tests that accept a `&self` parameter.
//!
//! # Reference
//!
//! The `generate_test_macro` is called on an `impl` block of the type
//! containing the test methods.  Its only parameter is the name of the macro to
//! generate.  Normal declarations inside the impl block are left unchanged, but
//! function definitions annotated with `#[test]`, `#[proptest]`, or
//! `#quickcheck` are treated as test cases.
//!
//! The generated macro needs three things when it is invoked: the name of a
//! module to generate, the name of the test suite type, the instance of the
//! test suite.  Often the name or value of the test type can be omitted, but
//! the full syntax is
//!
//!   `$macro_name!($module_name: $test_type = $test_instance);`
//!
//! ## Types of Test Functions
//!
//! ### #\[test\]
//!
//! Functions annotated with `#[test]` act like standard Rust test functions.
//! They can make `self` as a receiver, either by value or by reference, but
//! they can't have any other parameters and must return `()`.  The
//! `#[should_panic]` attribute is recognized and preserved.
//!
//! ### #\[proptest\]
//!
//! Function annotated with `#[proptest]` become `proptest` test cases.  They
//! can take `self`as a receiver and they must have additional argument
//! representing the test inputs to be generated by `proptest`.  By default, the
//! argument types must implement `proptest::arbitrary::Arbitrary`, but an
//! extended form of the attribute allows specifying custom strategies for
//! generating the test inputs, such as `#[proptest(arg in 0..100)]`, which
//! specifies that the parameter `arg` will have values generated from the
//! strategy `0..100`.
//!
//! Function with the `#[proptest]` attribute must not have a return type
//! specified; their return type is changed to
//! `proptest::test_runner::TestCaseResult`, just as when a function is called
//! from the `proptest!` macro.  This enables the test framework to properly
//! handle test failures and report them using various macro provided by
//! `proptest`.
//!
//! ### #\[quickcheck\]
//!
//! Functions annotated with `#[quickcheck]` become `quickcheck` test cases.
//! They cannot take `self` as a receiver and they must have additional
//! arguments, all of whose types implement `quickcheck::Arbitrary`.  The return
//! type of the function can be any type allowed by `quickcheck`, typically
//! `()`, `bool`, or `TestResult`.
//!
//! ## Transformation of Test Functions
//!
//! All test functions are made `pub` and `#[doc(hidden)]`.
//!
//! ## Creation of Test Instances
//!
//! If none of the test functions accept a `self` parameter, no instances of the
//! test type are created, and the `= $test_instance` syntax used to specify the
//! test instance must be omitted from the test macro invocation.
//!
//! For `#[test]` functions that have a `self` parameter, the test type instance
//! is created once for each test.  It can be mutated without affecting other
//! tests.
//!
//! For `#[proptest]` functions that have a `self` parameter, the test type
//! instance is created once once for each invocation of the test method, which
//! happens 256 times per test case by default.  For parameters with strategies,
//! an additional instance is created to provide the `self` context of the
//! strategy expressions.
//!
//! ### Abbreviated Signatures
//!
//! The `: $test_type` part of the test macro invocation may be omitted when it
//! is obvious from the `$test_instance` expression.  The type is considered
//! obvious when the expression starts with the unqualified name of the test
//! type, possibly followed by type arguments.
//!
//! The `= $test_instance` syntax may be omitted if the test suite type
//! implments `Default`, and must be omitted if no test function takes a `self`
//! parameter.
use std::collections::HashMap;

use proc_macro::TokenStream;
use proc_macro2::{Ident, Punct, Spacing, Span, TokenStream as TokenStream2, TokenTree};
use quote::quote;
use syn::{
    parse_macro_input, parse_quote, punctuated::Punctuated, FnArg, ImplItem, ItemImpl, LitStr, Pat,
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

fn proptest_strategy_method_name(method: &Ident, param: &Ident) -> Ident {
    let method_name = method.to_string();
    let param_name = param.to_string();
    Ident::new(
        &format!(
            "{}_{}_strategy",
            method_name.strip_prefix("r#").unwrap_or(&method_name),
            param_name.strip_prefix("r#").unwrap_or(&param_name)
        ),
        method.span(),
    )
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
    has_self: bool,
    param_names: Vec<Ident>,
    param_exprs: HashMap<String, syn::Expr>,
    /// Any `#[cfg(...)]` attributes on the original method, propagated verbatim
    /// onto the generated wrapper function.
    cfg_attrs: Vec<syn::Attribute>,
}

#[derive(Default)]
struct TestMethods {
    tests: Vec<TestMethod>,
    quickchecks: Vec<QuickcheckMethod>,
    proptests: Vec<ProptestMethod>,
}

impl TestMethods {
    fn is_empty(&self) -> bool {
        self.tests.is_empty() && self.quickchecks.is_empty() && self.proptests.is_empty()
    }

    fn has_self(&self) -> bool {
        self.tests.iter().any(|method| method.has_self)
            || self.proptests.iter().any(|method| method.has_self)
    }

    fn has_static(&self) -> bool {
        self.tests.iter().any(|method| !method.has_self)
            || !self.quickchecks.is_empty()
            || self.proptests.iter().any(|method| !method.has_self)
    }
}

// ---------------------------------------------------------------------------
// Proc-macro entry point
// ---------------------------------------------------------------------------

/// Generate a `macro_rules!` test harness from an `impl` block.
///
/// # Example
///
/// ```rust,ignore
/// struct MySuite<T>(T);
///
/// #[generate_test_macro(my_suite_tests)]
/// impl<T: MyTrait> MySuite<T> {
///     #[test]
///     fn it_works(&self) { /* … */ }
/// }
///
/// // In a consumer crate (MySuite must be in scope at the invocation site):
/// use my_crate::my_suite::MySuite;
/// my_suite_tests!(for_concrete_impl: MySuite<ConcreteType> = MySuite(42));
/// ```
#[proc_macro_attribute]
pub fn generate_test_macro(attr: TokenStream, item: TokenStream) -> TokenStream {
    let macro_name = parse_macro_input!(attr as Ident);
    let mut impl_block = parse_macro_input!(item as ItemImpl);

    let struct_name = extract_struct_name(&impl_block.self_ty);
    let methods = match collect_test_methods(&mut impl_block) {
        Ok(methods) => methods,
        Err(error) => return error,
    };

    if methods.is_empty() {
        return quote! {
            compile_error!("generate_test_macro: the impl block must contain at least one #[test], #[quickcheck], or #[proptest] method");
            #impl_block
        }
        .into();
    }

    let macro_rules_def = generate_macro_rules(&macro_name, &struct_name, &impl_block, &methods);
    quote! {
        #impl_block
        #macro_rules_def
    }
    .into()
}

fn collect_test_methods(impl_block: &mut ItemImpl) -> Result<TestMethods, TokenStream> {
    let mut methods = TestMethods::default();
    let mut strategy_methods = Vec::new();

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
            methods.tests.push(TestMethod {
                name: method_name,
                has_self,
                extra_attrs,
            });
        } else if is_quickcheck {
            let qm = build_quickcheck_method(method);
            methods.quickchecks.push(qm);
        } else if let Some(attr) = proptest_attr {
            let (proptest_method, generated_strategy_methods) =
                build_proptest_method(method, attr)?;
            methods.proptests.push(proptest_method);
            strategy_methods.extend(generated_strategy_methods);
        }
    }

    impl_block
        .items
        .extend(strategy_methods.into_iter().map(ImplItem::Fn));
    Ok(methods)
}

fn generate_test_wrappers(test_methods: &[TestMethod]) -> TokenStream2 {
    let dollar_type = dollar_ident("type");
    let dollar_expr = dollar_ident("expr");
    test_methods
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
        .collect()
}

fn generate_quickcheck_wrappers(quickcheck_methods: &[QuickcheckMethod]) -> TokenStream2 {
    let dollar_type = dollar_ident("type");
    quickcheck_methods
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
        .collect()
}

fn generate_proptest_wrappers(proptest_methods: &[ProptestMethod]) -> TokenStream2 {
    let dollar_type = dollar_ident("type");
    let dollar_expr = dollar_ident("expr");
    let instance = syn::Ident::new("instance", Span::mixed_site());

    proptest_methods
        .iter()
        .map(|pm| {
            let name = &pm.name;
            let cfg_attrs = &pm.cfg_attrs;
            let call_args: Punctuated<Ident, syn::Token![,]> =
                pm.param_names.iter().cloned().collect();
            let arg_name_strings: Punctuated<LitStr, syn::Token![,]> = pm
                .param_names
                .iter()
                .map(|name| LitStr::new(&name.to_string(), name.span()))
                .collect();
            let mut needs_instance = false;
            let mut strategies: Punctuated<TokenStream2, syn::Token![,]> = pm
                .args
                .iter()
                .filter_map(|arg| match arg {
                    FnArg::Typed(arg) => Some(arg),
                    FnArg::Receiver(_) => None,
                })
                .zip(call_args.iter())
                .map(|(arg, name)| {
                    if pm.param_exprs.contains_key(&name.to_string()) {
                        let strategy_method = proptest_strategy_method_name(&pm.name, name);
                        if pm.has_self {
                            needs_instance = true;
                            quote!(#instance.#strategy_method())
                        } else {
                            quote!(<#dollar_type>::#strategy_method())
                        }
                    } else {
                        let ty = &arg.ty;
                        quote!(::proptest::arbitrary::any::<#ty>())
                    }
                })
                .collect();
            // proptest! requires at least one strategy, even for receiver-only methods.
            if strategies.is_empty() {
                strategies.push(quote!(::proptest::strategy::Just(())));
            }
            let instance_decl = if needs_instance {
                quote! {
                    #[allow(unused_mut)]
                    let #instance : #dollar_type = #dollar_expr;
                }
            } else {
                quote! {}
            };
            let call = if pm.has_self {
                quote! {
                    #[allow(unused_mut)]
                    let mut #instance : #dollar_type = #dollar_expr;
                    #instance.#name(#call_args)?
                }
            } else {
                quote! {
                    <#dollar_type>::#name(#call_args)?
                }
            };
            quote! {
                #(#cfg_attrs)*
                #[test]
                #[allow(unused_parens)]
                fn #name() {
                    let mut config =
                        ::proptest::test_runner::contextualize_config(
                            ::proptest::test_runner::Config::default());
                    config.source_file = Some(file!());
                    let mut runner = ::proptest::test_runner::TestRunner::new(config);
                    #instance_decl
                    let strategies = (#strategies);
                    match runner.run(
                        &::proptest::strategy::Strategy::prop_map(
                            strategies,
                            |values| ::proptest::sugar::NamedArguments((#arg_name_strings), values)),
                        |::proptest::sugar::NamedArguments(_, (#call_args))| {
                            #call;
                            Ok(())
                        },
                    ) {
                        Ok(_) => (),
                        Err(e) => panic!("{}\n{}", e, runner),
                    }
                }
            }
        })
        .collect()
}

fn generate_invocation_arms(
    macro_name: &Ident,
    struct_name: &Ident,
    type_param_count: usize,
    methods: &TestMethods,
) -> (TokenStream2, TokenStream2) {
    let dollar_mod_name = dollar_ident("mod_name");
    let dollar_type = dollar_ident("type");
    let dollar_expr = dollar_ident("expr");
    if methods.has_self() {
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
        let plain_arm: TokenStream2 = if type_param_count == 0 || !methods.has_static() {
            let wildcard_params = if type_param_count == 0 {
                quote! {}
            } else {
                let wildcards = (0..type_param_count).fold(quote! {}, |acc, _| quote! { #acc _, });
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
    }
}

fn generate_macro_rules(
    macro_name: &Ident,
    struct_name: &Ident,
    impl_block: &ItemImpl,
    methods: &TestMethods,
) -> TokenStream2 {
    let dollar_mod_name = dollar_ident("mod_name");
    let test_fn_items = generate_test_wrappers(&methods.tests);
    let quickcheck_fn_items = generate_quickcheck_wrappers(&methods.quickchecks);
    let proptest_fn_items = generate_proptest_wrappers(&methods.proptests);
    let (main_pat, supporting_arms) = generate_invocation_arms(
        macro_name,
        struct_name,
        impl_block.generics.type_params().count(),
        methods,
    );
    quote! {
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
    }
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
) -> Result<(ProptestMethod, Vec<syn::ImplItemFn>), TokenStream> {
    if let syn::ReturnType::Type(..) = &method.sig.output {
        return Err(syn::Error::new_spanned(
            &method.sig.output,
            "generate_test_macro: #[proptest] methods must not specify a return type",
        )
        .to_compile_error()
        .into());
    }
    let mut param_names = Vec::new();
    for arg in &method.sig.inputs {
        if let FnArg::Typed(arg) = arg {
            match &*arg.pat {
                Pat::Ident(pat) => {
                    param_names.push(pat.ident.clone());
                }
                _ => {
                    return Err(syn::Error::new_spanned(
                        &arg.pat,
                        "generate_test_macro: #[proptest] method parameters must have an outer identifier binding",
                    )
                    .to_compile_error()
                    .into());
                }
            }
        }
    }
    let name = method.sig.ident.clone();
    let has_self = method
        .sig
        .inputs
        .first()
        .is_some_and(|arg| matches!(arg, FnArg::Receiver(_)));
    let param_exprs = parse_proptest_attr(proptest_attr)?;
    let cfg_attrs: Vec<_> = method
        .attrs
        .iter()
        .filter(|a| a.path().is_ident("cfg"))
        .cloned()
        .collect();
    let strategy_methods = method
        .sig
        .inputs
        .iter()
        .filter_map(|arg| {
            let FnArg::Typed(arg) = arg else {
                return None;
            };
            let Pat::Ident(param) = &*arg.pat else {
                return None;
            };
            let strategy = param_exprs.get(&param.ident.to_string())?;
            let strategy_method_name = proptest_strategy_method_name(&name, &param.ident);
            let arg_ty = &arg.ty;
            let receiver = has_self.then_some(quote!(&self));
            Some(parse_quote! {
                #(#cfg_attrs)*
                #[doc(hidden)]
                #[allow(non_snake_case)]
                pub fn #strategy_method_name(#receiver)
                    -> impl ::proptest::strategy::Strategy<Value = #arg_ty>
                {
                    #strategy
                }
            })
        })
        .collect();
    method.sig.output = parse_quote!(-> ::proptest::test_runner::TestCaseResult);
    if let Some(syn::Stmt::Expr(_, semi)) = method.block.stmts.last_mut() {
        *semi = Some(Default::default());
    }
    method
        .block
        .stmts
        .push(syn::Stmt::Expr(parse_quote!(Ok(())), None));
    Ok((
        ProptestMethod {
            name,
            args: method.sig.inputs.iter().cloned().collect(),
            has_self,
            param_names,
            param_exprs,
            cfg_attrs,
        },
        strategy_methods,
    ))
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
