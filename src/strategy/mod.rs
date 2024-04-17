//! Retry strategies.

mod exponential_backoff;
mod fibonacci_backoff;
mod fixed_interval;

pub use self::exponential_backoff::ExponentialBackoff;
pub use self::fibonacci_backoff::FibonacciBackoff;
pub use self::fixed_interval::FixedInterval;
