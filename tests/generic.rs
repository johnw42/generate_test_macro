// Test 2 – generic test suite (mirrors the README example directly)
//
// Uses a generic type parameter `T: MyTrait`, just like `TestSuite<T>` in the
// README.  Verifies that the generated macro correctly threads the concrete
// type through.

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
        pub fn new(param1: usize, param2: String) -> Self {
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

use generic_suite::GenericTestSuite;
generic_suite!(run_for_concrete_type: GenericTestSuite<ConcreteType> = GenericTestSuite::new(42, "world".to_string()));
generic_suite!(
    run_for_concrete_type_abbreviated =
        GenericTestSuite::<ConcreteType>::new(42, "world".to_string())
);
