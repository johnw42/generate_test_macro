// Test 1 – basic (non-generic) test suite
//
// Matches the first README example, minus the generic parameter.  Verifies:
//   • `new` is made pub (implicitly: the generated macro can call it)
//   • `#[test]` methods are made pub / callable from the generated module
//   • Multiple `#[test]` methods all appear in the generated module

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
basic_suite!(run_basic_suite: TestSuite = TestSuite::new(10, "hello".to_string()));
