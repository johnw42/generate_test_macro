// Test 4 – impl block with no `#[test]` or `#[quickcheck]` methods
//
// When there are no special methods the macro should be a no-op (it just
// returns the transformed impl block).  Verify the struct is still usable.

mod no_test_methods {
    use test_suite_macro::test_suite_macro;

    pub struct Plain {
        pub x: usize,
    }

    // No #[test] or #[quickcheck] methods – the macro should leave the impl
    // block unchanged and emit no macro_rules!.
    #[test_suite_macro(no_test_methods)]
    impl Plain {
        #[allow(dead_code)]
        pub fn helper(&self) -> usize {
            self.x + 1
        }
    }
}

#[test]
fn no_test_methods_impl_is_intact() {
    let p = no_test_methods::Plain { x: 5 };
    assert_eq!(p.x, 5);
    assert_eq!(p.helper(), 6);
}
