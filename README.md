Crate for generating test suites from the methods of a type.

This crate exposes a proc macro, `test_suite_macro`, that generates a new macro which can be expanded
multiple times to generate test suites.

## Basic Usage

To define a type as a test suite, use the `test_suite_macro` in a `impl` block, passing the name of the new macro to be generated.
The `impl` block should contain methods annotated with `#[test]`:

```rust
struct ExampleSuite<T> {...}

#[test_suite_macro(example_suite)]
impl ExampleSuite {
  #[test]
  pub fn instance_method_test(&self) {...}
}
```

This generates a new macro which generates a package containing a `#[test]`
function for each `#[test]` method of the type.  This calling convention of the
macro is

```
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
`quickcheck_macros` crate.

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
starts with the unqualified name of the test type, possibly followed by type arguments.

The `$test_instance` paremeter may be omitted if the test suite type implments `Default`.