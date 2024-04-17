//! finito provides retry mechanisms to retry async operations.
//!
//! It's based off [tokio-retry](https://github.com/srijs/rust-tokio-retry) with the difference that it isn't coupled
//! to any specific async runtime and that it compiles for WASM.

#![warn(missing_docs, missing_debug_implementations)]

mod action;
mod condition;
mod future;
mod strategy;

pub use action::Action;
pub use condition::Condition;
pub use future::{Retry, RetryIf};
pub use strategy::{ExponentialBackoff, FibonacciBackoff, FixedInterval};
