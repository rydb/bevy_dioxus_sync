//! Conditional tracing macros for workspace in order to not #[cfg(feature = "trace")] every trace call

#[cfg(not(all(feature = "trace", feature = "trace_perf")))]
use macro_v::macro_v;

#[cfg(feature = "trace")]
#[allow(unused_imports)]
pub use tracing::{debug, error, info, trace, warn};

#[cfg(feature = "trace_perf")]
#[allow(unused_imports)]
pub use tracing::{trace_span, info_span, debug_span, warn_span, error_span};

/// No-op version of span. For actual documentation, enable related feature flag.
pub struct NoOpSpan;

impl NoOpSpan {
    pub fn entered(&self) -> NoOpEntered {
        NoOpEntered
    }

    pub fn exit(&self) {}
}

pub struct NoOpEntered;

impl NoOpEntered {
    pub fn exit(self) {}
}

#[cfg(not(feature = "trace_perf"))]
#[macro_v(pub)]
macro_rules! trace_span {
    ($($arg:tt)*) => {
        $crate::NoOpSpan
    };
}

#[cfg(not(feature = "trace_perf"))]
#[macro_v(pub)]
macro_rules! info_span {
    ($($arg:tt)*) => {
        $crate::NoOpSpan
    };
}

#[cfg(not(feature = "trace_perf"))]
#[macro_v(pub)]
macro_rules! debug_span {
    ($($arg:tt)*) => {
        $crate::NoOpSpan
    };
}

#[cfg(not(feature = "trace_perf"))]
#[macro_v(pub)]
macro_rules! warn_span {
    ($($arg:tt)*) => {
        $crate::NoOpSpan
    };
}

#[cfg(not(feature = "trace_perf"))]
#[macro_v(pub)]
macro_rules! error_span {
    ($($arg:tt)*) => {
        $crate::NoOpSpan
    };
}



#[cfg(not(feature = "trace"))]
#[macro_v(pub)]
macro_rules! debug {
    ($($arg:tt)*) => {
        ()
    };
}

#[cfg(not(feature = "trace"))]
#[macro_v(pub)]
macro_rules! error {
    ($($arg:tt)*) => {
        ()
    };
}

#[cfg(not(feature = "trace"))]
#[macro_v(pub)]
macro_rules! info {
    ($($arg:tt)*) => {
        ()
    };
}

#[cfg(not(feature = "trace"))]
#[macro_v(pub)]
macro_rules! trace {
    ($($arg:tt)*) => {
        ()
    };
}

#[cfg(not(feature = "trace"))]
#[macro_v(pub)]
macro_rules! warn {
    ($($arg:tt)*) => {
        ()
    };
}
