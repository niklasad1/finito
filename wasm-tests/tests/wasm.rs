#![cfg(target_arch = "wasm32")]

use std::future;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use finito::{FixedInterval, Retry, RetryIf};
use wasm_bindgen_test::*;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

/// Run the tests by `$ wasm-pack test --firefox --headless`

fn init_tracing() {
    console_error_panic_hook::set_once();
    tracing_wasm::set_as_global_default();
}

#[wasm_bindgen_test]
async fn attempts_just_once() {
    init_tracing();

    let counter = Arc::new(AtomicUsize::new(0));
    let cloned_counter = counter.clone();
    let future = Retry::new(std::iter::empty(), move || {
        cloned_counter.fetch_add(1, Ordering::SeqCst);
        future::ready(Err::<(), u64>(42))
    });
    let res = future.await;

    assert_eq!(res, Err(42));
    assert_eq!(counter.load(Ordering::SeqCst), 1);
}

#[wasm_bindgen_test]
async fn attempts_until_max_retries_exceeded() {
    let s = FixedInterval::from_millis(100).take(2);
    let counter = Arc::new(AtomicUsize::new(0));
    let cloned_counter = counter.clone();
    let future = Retry::new(s, move || {
        cloned_counter.fetch_add(1, Ordering::SeqCst);
        future::ready(Err::<(), u64>(42))
    });
    let res = future.await;

    assert_eq!(res, Err(42));
    assert_eq!(counter.load(Ordering::SeqCst), 3);
}

#[wasm_bindgen_test]
async fn attempts_until_success() {
    let s = FixedInterval::from_millis(100);
    let counter = Arc::new(AtomicUsize::new(0));
    let cloned_counter = counter.clone();
    let future = Retry::new(s, move || {
        let previous = cloned_counter.fetch_add(1, Ordering::SeqCst);
        if previous < 3 {
            future::ready(Err::<(), u64>(42))
        } else {
            future::ready(Ok::<(), u64>(()))
        }
    });
    let res = future.await;

    assert_eq!(res, Ok(()));
    assert_eq!(counter.load(Ordering::SeqCst), 4);
}

#[wasm_bindgen_test]
async fn attempts_retry_only_if_given_condition_is_true() {
    let s = FixedInterval::from_millis(100).take(5);
    let counter = Arc::new(AtomicUsize::new(0));
    let cloned_counter = counter.clone();
    let future = RetryIf::new(
        s,
        move || {
            let previous = cloned_counter.fetch_add(1, Ordering::SeqCst);
            future::ready(Err::<(), usize>(previous + 1))
        },
        |e: &usize| *e < 3,
    );
    let res = future.await;

    assert_eq!(res, Err(3));
    assert_eq!(counter.load(Ordering::SeqCst), 3);
}
