// Test 5 – #[quickcheck] methods (requires `--features quickcheck`)
//
// Mirrors the README quickcheck example.  Covers:
//   • A `bool`-returning property (all outcomes accepted)
//   • A `TestResult`-returning property (with discard)
//   • A generic impl<T> suite (type param threaded through the macro call)

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
quickcheck_suite!(run_quickcheck_suite: MathSuite);

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
generic_quickcheck_suite!(run_generic_quickcheck_suite: GenericSuite<ConcreteType>);
