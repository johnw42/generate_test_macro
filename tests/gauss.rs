use std::ops::{Add, Div, Mul};

/// Computes the sum of integers from 0 to `n` using the Gaussian formula.
fn gauss_sum<N>(n: N) -> N
where
    N: From<u8> + Add<Output = N> + Div<Output = N> + Mul<Output = N> + Copy,
{
    n * (n + 1.into()) / 2.into()
}

#[cfg(test)]
mod naive_macro {
    use super::*;

    macro_rules! gauss_test {
        ($name:ident, $ty:ty, $n:expr) => {
            #[test]
            fn $name() {
                let expected: $ty = (0..=$n).into_iter().sum();
                let actual: $ty = gauss_sum($n);
                assert_eq!(expected, actual);
            }
        };
    }

    gauss_test!(for_u8, u8, 10);
    gauss_test!(for_i32, i32, 1024);
}

#[cfg(test)]
mod simple_tests {
    use super::*;

    use generate_test_macro::generate_test_macro;
    use std::fmt::Debug;
    use std::iter::Sum;
    use std::ops::RangeInclusive;

    struct GaussTest<N>(N);

    #[generate_test_macro(basic_tests)]
    impl<N> GaussTest<N>
    where
        N: From<u8> + Add<Output = N> + Div<Output = N> + Mul<Output = N> + Copy,
        N: Debug + Eq + Sum,
        RangeInclusive<N>: IntoIterator<Item = N>,
    {
        #[test]
        fn test_sum(&self) {
            let expected: N = (0.into()..=self.0).into_iter().sum();
            let actual: N = gauss_sum(self.0);
            assert_eq!(expected, actual);
        }
    }

    basic_tests!(for_u8 = GaussTest::<u8>(10));
    basic_tests!(for_i32 = GaussTest::<i32>(1024));
}

#[cfg(test)]
mod quickcheck_tests {
    use super::*;

    use generate_test_macro::generate_test_macro;
    use quickcheck::TestResult;
    use std::fmt::Debug;
    use std::iter::Sum;
    use std::marker::PhantomData;
    use std::ops::RangeInclusive;

    struct GaussTest<N, const LIMIT: usize>(PhantomData<N>);

    #[generate_test_macro(quickcheck_tests)]
    impl<N, const LIMIT: usize> GaussTest<N, LIMIT>
    where
        N: From<u8> + Add<Output = N> + Div<Output = N> + Mul<Output = N> + Copy,
        N: Debug + Eq + Sum + TryFrom<usize>,
        usize: TryFrom<N>,
        <N as TryFrom<usize>>::Error: Debug,
        RangeInclusive<N>: IntoIterator<Item = N>,
    {
        #[quickcheck]
        fn test_sum(arb: N) -> TestResult {
            // Discard the test if the conversion to usize fails
            let Ok(arb_usize) = usize::try_from(arb) else {
                return TestResult::discard();
            };
            // Generate a number in the range 0..=LIMIT
            let n: N = (arb_usize % LIMIT).try_into().unwrap();
            let expected: N = (0.into()..=n).into_iter().sum();
            let actual: N = gauss_sum(n);
            assert_eq!(expected, actual);
            TestResult::passed()
        }
    }

    quickcheck_tests!(for_u8: GaussTest<u8, 10>);
    quickcheck_tests!(for_i32: GaussTest<i32, 1024>);
}

#[cfg(test)]
mod propcheck_tests {
    use super::*;

    use generate_test_macro::generate_test_macro;
    use proptest::prelude::*;
    use std::fmt::Debug;
    use std::iter::Sum;
    use std::ops::RangeInclusive;

    struct GaussTest<N>(RangeInclusive<N>);

    #[generate_test_macro(propcheck_tests)]
    impl<N> GaussTest<N>
    where
        N: From<u8> + Add<Output = N> + Div<Output = N> + Mul<Output = N> + Copy,
        N: Debug + Eq + Sum,
        RangeInclusive<N>: IntoIterator<Item = N> + Strategy<Value = N>,
    {
        #[proptest(arb in self.0.clone())]
        fn test_sum(&self, arb: N) {
            let expected: N = (0.into()..=arb).into_iter().sum();
            let actual: N = gauss_sum(arb);
            prop_assert_eq!(expected, actual);
        }
    }

    propcheck_tests!(for_u8 = GaussTest::<u8>(0..=10));
    propcheck_tests!(for_i32 = GaussTest::<i32>(0..=1024));
    propcheck_tests!(for_i64 = GaussTest::<i64>(10_000..=100_000));
}
