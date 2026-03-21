// Test 1 – basic (non-generic) test suite
//
// Matches the first README example, minus the generic parameter.  Verifies:
//   • `new` is made pub (implicitly: the generated macro can call it)
//   • `#[test]` methods are made pub / callable from the generated module
//   • Multiple `#[test]` methods all appear in the generated module

mod basic_suite {
    use test_suite_macro::test_suite_macro;

    pub struct TestSuite<T: ToString> {
        pub param1: usize,
        pub param2: T,
    }

    #[test_suite_macro(basic_suite)]
    impl<T: ToString> TestSuite<T> {
        pub fn new(param1: usize, param2: T) -> Self {
            Self { param1, param2 }
        }

        #[test]
        pub fn param1_is_correct(&self) {
            assert_eq!(self.param1, 10);
        }

        #[test]
        pub fn param2_is_correct(&self) {
            assert_eq!(self.param2.to_string(), "hello");
        }
    }
}

// Invoke the generated macro to spin up a test module.  TestSuite must be
// in scope at the invocation site so the unqualified reference resolves.
use basic_suite::TestSuite;
basic_suite!(run_basic_suite_method: TestSuite<String> = TestSuite::new(10, "hello".to_string()));
basic_suite!(run_basic_suite_literal: TestSuite<String> = TestSuite { param1: 10, param2: "hello".to_string() });
basic_suite!(run_basic_suite_method_abbreviated = TestSuite::new(10, "hello".to_string()));
basic_suite!(
    run_basic_suite_literal_abbreviated = TestSuite {
        param1: 10,
        param2: "hello".to_string()
    }
);
