// Test 7 – #[test] methods without a self parameter
//
// A `#[test]` method that does not take `self` is a pure static test.  The
// generated wrapper calls `Suite::method()` directly instead of
// `Suite::new(...).method()`.  Because there are no self-taking tests in this
// suite, no `new` method or constructor args appear in the macro pattern.

mod static_test_suite {
    use std::marker::PhantomData;

    use generate_test_macro::generate_test_macro;

    #[derive(Default)]
    pub struct StaticSuite<T>(pub PhantomData<T>);

    #[generate_test_macro(static_test_suite)]
    impl<T> StaticSuite<T> {
        // No `new` needed – none of the tests take self.

        #[test]
        fn always_passes() {}

        #[test]
        fn string_len() {
            assert_eq!("hello".len(), 5);
        }

        #[test]
        fn not_static(&self) {}
    }
}

use std::marker::PhantomData;

use static_test_suite::StaticSuite;
static_test_suite!(with_type: StaticSuite<i32>);
static_test_suite!(with_instance = StaticSuite::<i32>(PhantomData));
