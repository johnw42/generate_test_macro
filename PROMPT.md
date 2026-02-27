This crate contains a proc macro for writing a suite of unit tests as methods of an object which can be instantiated via a new macro to run each method as an individual test.

# Example

Consider this test suite in `src/mytrait_tests.rs`:

```rust
pub struct TestSuite<T> {
  // Data needed for each test in the suite.
  param1: usize,
  param2: String,
}

#[test_suite_macro(mytrait_test_suite)]
impl<T: MyTrait> TestSuite<T> {
  fn new(param1: usize, param2: String) -> Self {
    Self { param1, param2 }
  }

  #[test]
  fn my_test(&self) {
    todo!("run a test");
  }
}
```

The expansion of the impl looks like this:

```rust
impl<T: MyTrait> TestSuite<T> {
  #[doc(hidden)]
  pub fn new(param1: usize, param2: String) -> Self {
    Self { param1, param2 }
  }

  #[doc(hidden)]
  pub fn my_test(&self) {
    todo!("run a test");
  }
}

#[macro_export]
macro_rules! mytrait_test_suite {
  ($mod_name:ident, $T:ty, $param1:expr, $param2:expr) => {
    mod $mod_name {
      use super::*;

      #[test]
      fn my_test() {
        TestSuite::<$T>::new($param1, $param2).my_test();
      }
    }
  }
}
```

The `TestSuite` type is referenced without qualification.  The generated module
contains `use super::*;`, so `TestSuite` must be in scope at the site where the
macro is invoked — either because it is defined there, or via an explicit `use`
statement:

```rust
use my_crate::mytrait_tests::TestSuite;
mytrait_test_suite!(my_tests, ConcreteType, 1, "hello".to_string());
```

If the "quickcheck" feature is enabled, quickcheck tests are also supported.  Consider this implementation of `TestSuite<T>`:

```rust
#[test_suite_macro(mytrait_test_suite)]
impl<T: MyTrait> TestSuite<T> {
  #[quickcheck]
  fn test_result_prop(data: MyTestData<T>) -> TestResult {
    todo!("generate a TestResult")
  }

  #[quickcheck]
  fn boolean_prop(data: MyTestData<T>) -> bool {
    todo!("generate a boolean result")
  }
}
```

The expansion looks like this:

```rust
impl<T: MyTrait> TestSuite<T> {
  #[doc(hidden)]
  pub fn test_result_prop(data: MyTestData<T>) -> TestResult {
    todo!("generate a TestResult")
  }

  #[doc(hidden)]
  pub fn boolean_prop(data: MyTestData<T>) -> bool {
    todo!("generate a boolean result")
  }
}

#[macro_export]
macro_rules! mytrait_test_suite {
  ($mod_name:ident, $T:ty) => {
    mod $mod_name {
      use super::*;

      #[test]
      pub fn test_result_prop() {
        quickcheck::quickcheck(TestSuite::<$T>::test_result_prop as fn(_) -> _);
      }


      #[test]
      pub fn boolean_prop() {
        quickcheck::quickcheck(TestSuite::<$T>::boolean_prop as fn(_) -> _);
      }
    }
  }
}
```

Other than a `new` method and methods annotated with `#[test]` or `#[quickcheck]`, all methods are copied as-is to the result.