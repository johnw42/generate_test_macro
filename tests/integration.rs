// Test 1 – basic (non-generic) test suite
//
// Matches the first README example, minus the generic parameter.  Verifies:
//   • `new` is made pub (implicitly: the generated macro can call it)
//   • `#[test]` methods are made pub / callable from the generated module
//   • Multiple `#[test]` methods all appear in the generated module
// ============================================================================

mod basic_suite {
    use test_suite_macro::test_suite_macro;

    pub struct TestSuite {
        param1: usize,
        param2: String,
    }

    #[test_suite_macro(basic_suite)]
    impl TestSuite {
        fn new(param1: usize, param2: String) -> Self {
            Self { param1, param2 }
        }

        #[test]
        fn param1_is_correct(&self) {
            assert_eq!(self.param1, 10);
        }

        #[test]
        fn param2_is_correct(&self) {
            assert_eq!(self.param2, "hello");
        }
    }
}

// Invoke the generated macro to spin up a test module.  TestSuite must be
// in scope at the invocation site so the unqualified reference resolves.
use basic_suite::TestSuite;
basic_suite!(run_basic_suite, 10, "hello".to_string());

// ============================================================================
// Test 2 – generic test suite (mirrors the README example directly)
//
// Uses a generic type parameter `T: MyTrait`, just like `TestSuite<T>` in the
// README.  Verifies that the generated macro correctly threads the concrete
// type through.
// ============================================================================

// Using `name()` via the trait is tested indirectly; keep the method but
// suppress the dead-code lint so that the API mirrors the README.
#[allow(dead_code)]
trait MyTrait: Send + Sync {
    fn name(&self) -> &str;
}

struct ConcreteType;

impl MyTrait for ConcreteType {
    fn name(&self) -> &str {
        "concrete"
    }
}

mod generic_suite {
    use test_suite_macro::test_suite_macro;

    use super::MyTrait;

    pub struct GenericTestSuite<T> {
        param1: usize,
        param2: String,
        _marker: std::marker::PhantomData<T>,
    }

    #[test_suite_macro(generic_suite)]
    impl<T: MyTrait> GenericTestSuite<T> {
        fn new(param1: usize, param2: String) -> Self {
            Self {
                param1,
                param2,
                _marker: std::marker::PhantomData,
            }
        }

        #[test]
        fn param1_is_correct(&self) {
            assert_eq!(self.param1, 42);
        }

        #[test]
        fn param2_is_correct(&self) {
            assert_eq!(self.param2, "world");
        }
    }
}

// Invoke the generated macro with a concrete type.  The generic TestSuite
// must be in scope (re-exported from generic_suite) so the wrapper can call
// TestSuite::<ConcreteType>::new(...).
use generic_suite::GenericTestSuite;
generic_suite!(run_for_concrete_type, ConcreteType, 42, "world".to_string());

// ============================================================================
// Test 3 – non-test helper methods are copied as-is
//
// The README states: "Other than a `new` method and methods annotated with
// `#[test]` or `#[quickcheck]`, all methods are copied as-is to the result."
// Verify that a plain helper method is accessible and works correctly.
// ============================================================================

mod passthrough_suite {
    use test_suite_macro::test_suite_macro;

    pub struct Suite {
        value: usize,
    }

    #[test_suite_macro(passthrough_suite)]
    impl Suite {
        fn new(value: usize) -> Self {
            Self { value }
        }

        // Plain helper – should be copied without modification (not made pub).
        fn doubled(&self) -> usize {
            self.value * 2
        }

        #[test]
        fn doubled_is_correct(&self) {
            assert_eq!(self.doubled(), 84);
        }
    }
}

use passthrough_suite::Suite;
passthrough_suite!(run_passthrough_suite, 42);

// ============================================================================
// Test 4 – impl block with no `#[test]` or `#[quickcheck]` methods
//
// When there are no special methods the macro should be a no-op (it just
// returns the transformed impl block).  Verify the struct is still usable.
// ============================================================================

mod no_test_methods {
    use test_suite_macro::test_suite_macro;

    pub struct Plain {
        pub x: usize,
    }

    // No #[test] or #[quickcheck] methods – the macro should leave the impl
    // block unchanged and emit no macro_rules!.
    #[test_suite_macro(no_test_methods)]
    impl Plain {
        #[allow(dead_code)]
        pub fn helper(&self) -> usize {
            self.x + 1
        }
    }
}

#[test]
fn no_test_methods_impl_is_intact() {
    let p = no_test_methods::Plain { x: 5 };
    assert_eq!(p.x, 5);
    assert_eq!(p.helper(), 6);
}

// ============================================================================
// Test 5 – #[quickcheck] methods (requires `--features quickcheck`)
//
// Mirrors the README quickcheck example.  Covers:
//   • A `bool`-returning property (all outcomes accepted)
//   • A `TestResult`-returning property (with discard)
//   • A generic impl<T> suite (type param threaded via $T:ty in the macro call)
// ============================================================================

#[cfg(feature = "quickcheck")]
mod quickcheck_suite {
    use quickcheck::TestResult;
    use test_suite_macro::test_suite_macro;

    pub struct MathSuite;

    #[test_suite_macro(quickcheck_suite)]
    impl MathSuite {
        /// Addition is commutative for all u32 pairs.
        #[quickcheck]
        fn addition_is_commutative(a: u32, b: u32) -> bool {
            a.wrapping_add(b) == b.wrapping_add(a)
        }

        /// Subtraction result is at most `a` when `a >= b`; discard otherwise.
        #[quickcheck]
        fn subtraction_is_bounded(a: u32, b: u32) -> TestResult {
            if a < b {
                return TestResult::discard();
            }
            TestResult::from_bool(a - b <= a)
        }
    }
}

#[cfg(feature = "quickcheck")]
use quickcheck_suite::MathSuite;
#[cfg(feature = "quickcheck")]
quickcheck_suite!(run_quickcheck_suite);

// Generic quickcheck suite – mirrors the README's `TestSuite<T: MyTrait>` but
// the property only exercises the type-param threading, not the trait itself.
#[cfg(feature = "quickcheck")]
mod generic_quickcheck_suite {
    use test_suite_macro::test_suite_macro;

    use super::MyTrait;

    pub struct GenericSuite<T> {
        _marker: std::marker::PhantomData<T>,
    }

    #[test_suite_macro(generic_quickcheck_suite)]
    impl<T: MyTrait + 'static> GenericSuite<T> {
        /// Multiplying any u32 by 1 is an identity operation.
        #[quickcheck]
        fn multiply_by_one_is_identity(n: u32) -> bool {
            n.wrapping_mul(1) == n
        }
    }
}

#[cfg(feature = "quickcheck")]
use generic_quickcheck_suite::GenericSuite;
#[cfg(feature = "quickcheck")]
generic_quickcheck_suite!(run_generic_quickcheck_suite, ConcreteType);

// ============================================================================
// Test 6 – #[cfg(...)] attributes are propagated to generated wrapper fns
//
// Verifies that a cfg attribute on a #[test] (or #[quickcheck]) method is
// copied onto the generated wrapper, so the wrapper is only compiled/run when
// the condition holds.  Uses platform-width attributes as a reliable cfg
// condition available in every build.
// ============================================================================

mod cfg_suite {
    use test_suite_macro::test_suite_macro;

    pub struct CfgSuite;

    #[test_suite_macro(cfg_suite)]
    impl CfgSuite {
        fn new() -> Self {
            Self
        }

        // This wrapper must only exist (and run) on 64-bit targets.
        // If the #[cfg] were not propagated to the wrapper, this would fail to
        // compile on 32-bit targets because `CfgSuite::only_on_64bit` wouldn't
        // exist there.
        #[test]
        #[cfg(target_pointer_width = "64")]
        fn only_on_64bit(&self) {
            assert_eq!(std::mem::size_of::<usize>(), 8);
        }

        // Symmetrically compiled only on non-64-bit targets.
        #[test]
        #[cfg(not(target_pointer_width = "64"))]
        fn only_on_non_64bit(&self) {
            assert!(std::mem::size_of::<usize>() < 8);
        }

        // Always present – ensures at least one wrapper is always generated.
        #[test]
        fn always_runs(&self) {}
    }
}

use cfg_suite::CfgSuite;
cfg_suite!(run_cfg_suite);

// Quickcheck variant: cfg on a #[quickcheck] method.
#[cfg(feature = "quickcheck")]
mod cfg_quickcheck_suite {
    use test_suite_macro::test_suite_macro;

    pub struct CfgQcSuite;

    #[test_suite_macro(cfg_quickcheck_suite)]
    impl CfgQcSuite {
        #[quickcheck]
        #[cfg(target_pointer_width = "64")]
        fn usize_fits_u64(n: u64) -> bool {
            // On 64-bit usize == u64, so casting back must be lossless.
            n as usize as u64 == n
        }
    }
}

#[cfg(feature = "quickcheck")]
use cfg_quickcheck_suite::CfgQcSuite;
#[cfg(feature = "quickcheck")]
cfg_quickcheck_suite!(run_cfg_quickcheck_suite);

// ============================================================================
// Test 7 – #[test] methods without a self parameter
//
// A `#[test]` method that does not take `self` is a pure static test.  The
// generated wrapper calls `Suite::method()` directly instead of
// `Suite::new(...).method()`.  Because there are no self-taking tests in this
// suite, no `new` method or constructor args appear in the macro pattern.
// ============================================================================

mod static_test_suite {
    use test_suite_macro::test_suite_macro;

    pub struct StaticSuite;

    #[test_suite_macro(static_test_suite)]
    impl StaticSuite {
        // No `new` needed – none of the tests take self.

        #[test]
        fn always_passes() {}

        #[test]
        fn string_len() {
            assert_eq!("hello".len(), 5);
        }
    }
}

use static_test_suite::StaticSuite;
static_test_suite!(run_static_test_suite);

// ============================================================================
// Test 8 – mixed suite: some #[test] methods take self, some do not
//
// When at least one test takes self, the constructor args must appear in the
// macro pattern.  Tests without self should still compile and run correctly.
// ============================================================================

mod mixed_suite {
    use test_suite_macro::test_suite_macro;

    pub struct MixedSuite {
        value: usize,
    }

    #[test_suite_macro(mixed_suite)]
    impl MixedSuite {
        fn new(value: usize) -> Self {
            Self { value }
        }

        // Static – does not need an instance.
        #[test]
        fn static_always_passes() {}

        // Instance – needs `new`.
        #[test]
        fn instance_value_correct(self) {
            assert_eq!(self.value, 99);
        }
    }
}

use mixed_suite::MixedSuite;
mixed_suite!(run_mixed_suite, 99);
