A proc macro for writing a suite of unit tests as methods on a struct.  This is
useful for running the same suite of tests on multiple instances of the struct,
especially when testing multiple implementations of a trait.

Attach `#[generate_test_macro(name)]` to an `impl` block to produce:

1. The same `impl` block with `#[test]`, and `#[quickcheck]` methods
   made `pub` / `#[doc(hidden)]` (and their special attributes stripped).
2. A `macro_rules! name { … }` definition that, when invoked with a module name,
   concrete type arguments, and a constructor expression, creates an isolated
   test module with a test function for each `#[test]` or `#[quickcheck]` method
   in the `impl` block.

## Basic Usage

To define a type as a test suite, use the `generate_test_macro` in a `impl` block, passing the name of the new macro to be generated.
The `impl` block should contain methods annotated with `#[test]`:

```rust
struct ExampleSuite<T> {...}

#[generate_test_macro(example_suite)]
impl ExampleSuite {
  #[test]
  pub fn instance_method_test(&self) {...}
}
```

This generates a new macro which generates a package containing a `#[test]`
function for each `#[test]` method of the type.  This calling convention of the
macro is

```rust
example_suite($package_name: $test_type = $test_instance);
```

For example, invoking the macro like this

```rust
example_suite(test1: ExampleSuite<i32> = ExampleSuite {...});

```

will expand (roughly) to this module definition:

```rust
mod test1 {
  use super::*;

  #[test]
  fn instance_method_test() {
    let instance: ExampleSuite<i32> = ExampleSuite {...};
    instance.instance_method_test();
  }
}
```

## Advanced Usage

### Quickcheck

In addition to methods marked as `#[test]`, methods can be marked with
`#[quickcheck]`.  This operates like the `quickcheck` macro provided by the
`quickcheck_macros` crate.  To use it, your crate will need to depend
on `quickcheck`.  Note that `#[quickcheck]` methods cannot take a `self`
parameter due to limitations of `quickcheck`.

### The `self` Parameter

The `self` parameter of test methods may be passed by value, reference, or mut
reference.  The distinction is largely irrelevant, because a new instance of the
test suite type is created for each test.

### Static Methods

Test methods may be static. If all test methods are static, there is no need to
pass a `$test_instance` parameter to the generated macro.  Instances of the test
suite type are not created for static test methods.

### Abbreviated Signatures

The `$test_type` parameter may be omitted when it is obvious from the
`$test_instance` expression.  The type is considered obvious when the expression
starts with the unqualified name of the test type, possibly followed by type
arguments.

The `$test_instance` paremeter may be omitted if the test suite type implments
`Default`, or if all test methods are static.