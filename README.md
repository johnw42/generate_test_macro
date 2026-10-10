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