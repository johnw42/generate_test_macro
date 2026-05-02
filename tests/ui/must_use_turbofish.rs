// Test 7 – #[test] methods without a self parameter
//
mod static_test_suite {
    use generate_test_macro::generate_test_macro;

    #[derive(Default)]
    pub struct StaticSuite<T>(pub T);

    #[generate_test_macro(static_test_suite)]
    impl<T> StaticSuite<T> {
        #[test]
        fn not_static(&self) {}

        #[test]
        fn static_method() {}
    }
}

#[allow(unused)]
use static_test_suite::StaticSuite;
static_test_suite!(with_instance = StaticSuite(0i32));

fn main() {}
