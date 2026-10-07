use generate_test_macro::generate_test_macro;

struct Suite;

#[generate_test_macro(tuple_suite)]
impl Suite {
    #[proptest]
    fn tuple((a, b): (u32, u32)) {}
}

#[generate_test_macro(wildcard_suite)]
impl Suite {
    #[proptest]
    fn wildcard(_: u32) {}
}

#[generate_test_macro(mut_suite)]
impl Suite {
    #[proptest]
    fn mut_param(mut _x: u32) {}
}

#[generate_test_macro(ref_suite)]
impl Suite {
    #[proptest]
    fn ref_param(ref _x: u32) {}
}

#[generate_test_macro(named_pattern_suite)]
impl Suite {
    #[proptest]
    fn named_pattern(_pair @ (_a, _b): (u32, u32)) {}
}

fn main() {}
