[![Build Status](https://github.com/johnw42/generate_test_macro/actions/workflows/rust.yml/badge.svg)](https://github.com/rust-lang/generate_test_macro/actions)
[![crates.io](https://img.shields.io/crates/v/generate_test_macro.svg)](https://crates.io/crates/generate_test_macro)
[![Documentation](https://docs.rs/generate_test_macro/badge.svg)](https://docs.rs/generate_test_macro)
[![Rust](https://img.shields.io/badge/rust-1.85.0%2B-blue.svg?maxAge=3600)](https://github.com/rust-lang/generate_test_macro)


# generate_test_macro

A proc macro for writing a suite of unit tests as methods of a type, with
support for `proptest` and `quickcheck`.  This is useful for running the same
suite of tests on multiple instances of the type, especially when testing
multiple implementations of a trait.

The most basic usage of this crate looks like this:

```rust
struct TestSuite;

#[generate_test_macro(macro_name)]
impl TestSuite {
    #[test]
    fn test_function(&self) {
      // Test logic goes here.
    }
}

macro_name!(module_name: TestSuite = TestSuite)
```

The example above offers no advantages over simply writing a `#[test]` function;
the real power of this crate comes from cases where `TestSuite` has fields
and/or type parameters.  See the module-level docs for details instructions an
motivation.