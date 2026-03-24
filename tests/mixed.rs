// Test 8 – mixed suite: some #[test] methods take self, some do not
//
// When at least one test takes self, the constructor args must appear in the
// macro pattern.  Tests without self should still compile and run correctly.

mod mixed_suite {
    use test_suite_macro::test_suite_macro;

    pub struct MixedSuite {
        value: usize,
    }

    #[test_suite_macro(mixed_suite)]
    impl MixedSuite {
        pub fn new(value: usize) -> Self {
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
mixed_suite!(run_mixed_suite: MixedSuite = MixedSuite::new(99));
