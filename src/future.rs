use std::future::Future;
use std::iter::{IntoIterator, Iterator};
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

use pin_project::pin_project;

use super::action::Action;
use super::condition::Condition;

#[pin_project(project = RetryStateProj)]
enum RetryState<A>
where
    A: Action,
{
    Running(#[pin] A::Future),
    Sleeping(#[pin] futures_timer::Delay),
}

impl<A: Action> RetryState<A> {
    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> RetryFuturePoll<A> {
        match self.project() {
            RetryStateProj::Running(future) => RetryFuturePoll::Running(future.poll(cx)),
            RetryStateProj::Sleeping(future) => RetryFuturePoll::Sleeping(future.poll(cx)),
        }
    }
}

impl<A: Action> std::fmt::Debug for RetryState<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Running(_) => "RetryState::Running",
            Self::Sleeping(_) => "RetryState::Sleeping",
        };
        f.write_str(s)
    }
}

enum RetryFuturePoll<A>
where
    A: Action,
{
    Running(Poll<Result<A::Item, A::Error>>),
    Sleeping(Poll<()>),
}

/// Future that drives multiple attempts at an action via a retry strategy.
#[derive(Debug)]
#[pin_project]
pub struct Retry<I, A>
where
    I: Iterator<Item = Duration>,
    A: Action,
{
    #[pin]
    retry_if: RetryIf<I, A, fn(&A::Error) -> bool>,
}

impl<I, A> Retry<I, A>
where
    I: Iterator<Item = Duration>,
    A: Action,
{
    /// Create a retryable future that resolves
    /// when the response is succesful.
    pub fn new<T: IntoIterator<IntoIter = I, Item = Duration>>(
        strategy: T,
        action: A,
    ) -> Retry<I, A> {
        Retry {
            retry_if: RetryIf::new(strategy, action, (|_| true) as fn(&A::Error) -> bool),
        }
    }
}

impl<I, A> Future for Retry<I, A>
where
    I: Iterator<Item = Duration>,
    A: Action,
{
    type Output = Result<A::Item, A::Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let this = self.project();
        this.retry_if.poll(cx)
    }
}

/// Future that drives multiple attempts at an action via a retry strategy. Retries are only attempted if
/// the `Error` returned by the future satisfies a given condition.
#[derive(Debug)]
#[pin_project]
pub struct RetryIf<I, A, C>
where
    I: Iterator<Item = Duration>,
    A: Action,
    C: Condition<A::Error>,
{
    strategy: I,
    #[pin]
    state: RetryState<A>,
    action: A,
    condition: C,
}

impl<I, A, C> RetryIf<I, A, C>
where
    I: Iterator<Item = Duration>,
    A: Action,
    C: Condition<A::Error>,
{
    /// Create a [`RetryIf`] future that resolves when the future
    /// was succesful or that error condition was met.
    pub fn new<T: IntoIterator<IntoIter = I, Item = Duration>>(
        strategy: T,
        mut action: A,
        condition: C,
    ) -> RetryIf<I, A, C> {
        RetryIf {
            strategy: strategy.into_iter(),
            state: RetryState::Running(action.run()),
            action,
            condition,
        }
    }

    fn attempt(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<A::Item, A::Error>> {
        let future = {
            let this = self.as_mut().project();
            this.action.run()
        };
        self.as_mut()
            .project()
            .state
            .set(RetryState::Running(future));
        self.poll(cx)
    }

    fn retry(
        mut self: Pin<&mut Self>,
        err: A::Error,
        cx: &mut Context,
    ) -> Result<Poll<Result<A::Item, A::Error>>, A::Error> {
        match self.as_mut().project().strategy.next() {
            None => Err(err),
            Some(duration) => {
                let sleep = futures_timer::Delay::new(duration);
                self.as_mut()
                    .project()
                    .state
                    .set(RetryState::Sleeping(sleep));
                Ok(self.poll(cx))
            }
        }
    }
}

impl<I, A, C> Future for RetryIf<I, A, C>
where
    I: Iterator<Item = Duration>,
    A: Action,
    C: Condition<A::Error>,
{
    type Output = Result<A::Item, A::Error>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        match self.as_mut().project().state.poll(cx) {
            RetryFuturePoll::Running(poll_result) => match poll_result {
                Poll::Ready(Ok(ok)) => Poll::Ready(Ok(ok)),
                Poll::Pending => Poll::Pending,
                Poll::Ready(Err(err)) => {
                    if self.as_mut().project().condition.should_retry(&err) {
                        match self.retry(err, cx) {
                            Ok(poll) => poll,
                            Err(err) => Poll::Ready(Err(err)),
                        }
                    } else {
                        Poll::Ready(Err(err))
                    }
                }
            },
            RetryFuturePoll::Sleeping(poll_result) => match poll_result {
                Poll::Pending => Poll::Pending,
                Poll::Ready(_) => self.attempt(cx),
            },
        }
    }
}
