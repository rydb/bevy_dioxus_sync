//! Conditional tracing macros for workspace in order to not #[cfg(feature = "trace")] every trace call

#[cfg(feature = "trace")]
#[allow(unused_imports)]
pub use tracing::{debug, error, info, trace, warn};

#[cfg(not(feature = "trace"))]
use macro_v::macro_v;

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
