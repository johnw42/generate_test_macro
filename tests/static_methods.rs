// Test 7 – #[test] methods without a self parameter
//
// A `#[test]` method that does not take `self` is a pure static test.  The
// generated wrapper calls `Suite::method()` directly instead of
// `Suite::new(...).method()`.  Because there are no self-taking tests in this
// suite, no `new` method or constructor args appear in the macro pattern.

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
static_test_suite!(run_static_test_suite: StaticSuite);
