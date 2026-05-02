use generate_test_macro::generate_test_macro;

struct Plain {
    pub x: usize,
}

#[generate_test_macro(plain_suite)]
impl Plain {
    pub fn helper(&self) -> usize {
        self.x + 1
    }
}

fn main() {}
