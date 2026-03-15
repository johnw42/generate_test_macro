use test_suite_macro::test_suite_macro;

struct Plain {
    pub x: usize,
}

#[test_suite_macro(plain_suite)]
impl Plain {
    pub fn helper(&self) -> usize {
        self.x + 1
    }
}

fn main() {}
