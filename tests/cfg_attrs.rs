// Test 6 – #[cfg(...)] attributes are propagated to generated wrapper fns
//
// Verifies that a cfg attribute on a #[test] (or #[quickcheck]) method is
// copied onto the generated wrapper, so the wrapper is only compiled/run when
// the condition holds.  Uses platform-width attributes as a reliable cfg
// condition available in every build.

mod cfg_suite {
    use test_suite_macro::test_suite_macro;

    pub struct CfgSuite;

    #[test_suite_macro(cfg_suite)]
    impl CfgSuite {
        fn new() -> Self {
            Self
        }

        // This wrapper must only exist (and run) on 64-bit targets.
        // If the #[cfg] were not propagated to the wrapper, this would fail to
        // compile on 32-bit targets because `CfgSuite::only_on_64bit` wouldn't
        // exist there.
        #[test]
        #[cfg(target_pointer_width = "64")]
        fn only_on_64bit(&self) {
            assert_eq!(std::mem::size_of::<usize>(), 8);
        }

        // Symmetrically compiled only on non-64-bit targets.
        #[test]
        #[cfg(not(target_pointer_width = "64"))]
        fn only_on_non_64bit(&self) {
            assert!(std::mem::size_of::<usize>() < 8);
        }

        // Always present – ensures at least one wrapper is always generated.
        #[test]
        fn always_runs(&self) {}
    }
}

use cfg_suite::CfgSuite;
cfg_suite!(run_cfg_suite: CfgSuite = CfgSuite);

// Quickcheck variant: cfg on a #[quickcheck] method.
mod cfg_quickcheck_suite {
    use test_suite_macro::test_suite_macro;

    pub struct CfgQcSuite;

    #[test_suite_macro(cfg_quickcheck_suite)]
    impl CfgQcSuite {
        #[quickcheck]
        #[cfg(target_pointer_width = "64")]
        fn usize_fits_u64(n: u64) -> bool {
            // On 64-bit usize == u64, so casting back must be lossless.
            n as usize as u64 == n
        }
    }
}

use cfg_quickcheck_suite::CfgQcSuite;
cfg_quickcheck_suite!(run_cfg_quickcheck_suite: CfgQcSuite);
