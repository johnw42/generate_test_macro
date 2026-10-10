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

mod proptest_suite {
    use generate_test_macro::generate_test_macro;
    use proptest::prelude::*;

    #[derive(Default)]
    pub struct MathSuite(pub u32);

    #[generate_test_macro(proptest_suite)]
    impl MathSuite {
        fn private_method(&self) -> impl Strategy<Value = u32> {
            0..=self.0
        }

        fn private_function() -> impl Strategy<Value = u32> {
            0..10_u32
        }

        /// Addition is commutative for all u32 pairs.
        #[proptest(a in self.private_method(), b in self.private_method())]
        fn addition_is_commutative(self, a: u32, b: u32) {
            let _ = self.0;
            prop_assert_eq!(a.wrapping_add(b), b.wrapping_add(a));
        }

        /// Subtraction result is at most `a` when `a >= b`; discard otherwise.
        #[proptest(b in Self::private_function())]
        fn subtraction_is_bounded(a: u32, b: u32) {
            prop_assume!(a >= b);
            prop_assert!(a - b <= a);
        }
    }
}

use proptest_suite::MathSuite;
proptest_suite!(run_proptest_suite = MathSuite(0));

// Generic quickcheck suite – mirrors the README's `TestSuite<T: MyTrait>` but
// the property only exercises the type-param threading, not the trait itself.
mod generic_proptest_suite {
    use generate_test_macro::generate_test_macro;
    use proptest::prelude::*;

    use super::MyTrait;

    pub struct GenericSuite<T> {
        _marker: std::marker::PhantomData<T>,
    }

    fn private_method() -> impl Strategy<Value = u32> {
        0u32..10
    }

    #[generate_test_macro(generic_proptest_suite)]
    impl<T: MyTrait + 'static> GenericSuite<T> {
        /// Multiplying any u32 by 1 is an identity operation.
        #[proptest(n in private_method())]
        fn multiply_by_one_is_identity(n: u32) {
            proptest::prop_assert_eq!(n.wrapping_mul(1), n);
        }
    }
}

use generic_proptest_suite::GenericSuite;
generic_proptest_suite!(run_generic_proptest_suite: GenericSuite<ConcreteType>);
