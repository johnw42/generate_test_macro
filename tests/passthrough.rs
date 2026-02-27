// Test 3 – non-test helper methods are copied as-is
//
// The README states: "Other than a `new` method and methods annotated with
// `#[test]` or `#[quickcheck]`, all methods are copied as-is to the result."
// Verify that a plain helper method is accessible and works correctly.

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
passthrough_suite!(run_passthrough_suite: Suite = Suite::new(42));
