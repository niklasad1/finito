# finito

This library provides retry mechanisms to retry async operations.

It's based off [tokio-retry](https://github.com/srijs/rust-tokio-retry) with the difference that it isn't coupled
to any specific async runtime and that it compiles for WASM.

## Examples

```rust
use finito::{Retry, ExponentialBackoff};

async fn action() -> Result<u64, ()> {
    // do some real-world stuff here...
    Err(())
}

#[tokio::main]
async fn main() -> Result<(), ()> {
    let retry_strategy = ExponentialBackoff::from_millis(10).take(3);    // limit to 3 retries

    let result = Retry::new(retry_strategy, action).await?;

    Ok(())
}
```
