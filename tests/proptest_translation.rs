use generate_test_macro::generate_test_macro;
use proptest::strategy::Strategy;
use proptest::test_runner::{TestCaseError, TestCaseResult};

struct Suite;

#[allow(clippy::no_effect)]
#[generate_test_macro(translated_proptest_suite)]
impl Suite {
    #[proptest(value in 0u32..10)]
    fn assertions(value: u32) {
        proptest::prop_assume!(value > 0);
        proptest::prop_assert_eq!(value, 1);
    }

    #[proptest]
    fn empty_body() {}

    #[proptest()]
    fn original_tail_expression() {
        42
    }

    #[proptest]
    fn fallible_body() {
        let result: TestCaseResult = Err(TestCaseError::fail("failure"));
        result?;
    }
}

#[test]
fn proptest_methods_return_test_case_results() {
    let result: TestCaseResult = Suite::assertions(1);
    assert!(result.is_ok());
    assert!(Suite::empty_body().is_ok());
    assert!(Suite::original_tail_expression().is_ok());
}

#[test]
fn explicit_proptest_strategies_are_available_as_associated_methods() {
    fn assert_u32_strategy<S: Strategy<Value = u32>>(_: S) {}

    assert_u32_strategy(Suite::assertions_value_strategy());
    let mut runner = proptest::test_runner::TestRunner::default();
    let _tree = Suite::assertions_value_strategy()
        .new_tree(&mut runner)
        .unwrap();
}

#[test]
fn proptest_methods_preserve_failures_and_rejections() {
    assert!(matches!(
        Suite::assertions(0),
        Err(TestCaseError::Reject(_))
    ));
    assert!(matches!(Suite::assertions(2), Err(TestCaseError::Fail(_))));
    assert!(matches!(
        Suite::fallible_body(),
        Err(TestCaseError::Fail(_))
    ));
}
