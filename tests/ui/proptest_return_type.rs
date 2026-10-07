use generate_test_macro::generate_test_macro;

struct BoolSuite;

#[generate_test_macro(bool_suite)]
impl BoolSuite {
    #[proptest(value in 0..10)]
    fn property(value: u32) -> bool {
        value < 10
    }
}

struct UnitSuite;

#[generate_test_macro(unit_suite)]
impl UnitSuite {
    #[proptest]
    fn property() -> () {}
}

struct ResultSuite;

#[generate_test_macro(result_suite)]
impl ResultSuite {
    #[proptest()]
    fn property() -> proptest::test_runner::TestCaseResult {
        Ok(())
    }
}

fn main() {}
