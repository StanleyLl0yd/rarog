#![deny(unsafe_op_in_unsafe_fn)]

#[cfg(feature = "spidermonkey")]
mod backend;

#[cfg(feature = "spidermonkey")]
pub use backend::{SpiderMonkeyEngine, SpiderMonkeyRuntime};
