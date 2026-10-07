use generate_test_macro::generate_test_macro;

#[derive(Default)]
struct ReceiverSuite {
    changed: bool,
}

#[generate_test_macro(receiver_suite)]
impl ReceiverSuite {
    #[proptest(value in 1u32..10)]
    fn owned(self, value: u32) {
        proptest::prop_assert!(!self.changed);
        proptest::prop_assert!((1..10).contains(&value));
    }

    #[proptest]
    fn shared(&self, value: u32) {
        proptest::prop_assert!(!self.changed);
        proptest::prop_assert_eq!(value.wrapping_add(0), value);
    }

    #[proptest(value in 1u32..10)]
    fn mutable(&mut self, value: u32) {
        proptest::prop_assert!(!self.changed);
        self.changed = true;
        proptest::prop_assert!((1..10).contains(&value));
    }

    #[proptest]
    fn receiver_only(&self) {
        proptest::prop_assert!(!self.changed);
    }

    #[proptest(value in 1u32..10)]
    fn static_property(value: u32) {
        proptest::prop_assert!((1..10).contains(&value));
    }
}

receiver_suite!(explicit: ReceiverSuite = ReceiverSuite { changed: false });
receiver_suite!(defaulted: ReceiverSuite);
receiver_suite!(abbreviated = ReceiverSuite { changed: false });

struct GenericSuite<T> {
    value: T,
}

#[generate_test_macro(generic_receiver_suite)]
impl<T: Copy + PartialEq + std::fmt::Debug> GenericSuite<T> {
    #[proptest(value in 1u32..10)]
    fn shared(&self, value: u32) {
        proptest::prop_assert_eq!(self.value, self.value);
        proptest::prop_assert!((1..10).contains(&value));
    }
}

generic_receiver_suite!(generic_abbreviated = GenericSuite { value: 42u32 });
generic_receiver_suite!(generic_turbofish = GenericSuite::<u32> { value: 42 });
