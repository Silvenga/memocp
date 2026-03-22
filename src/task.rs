use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::task::{JoinError, JoinHandle, spawn_blocking};
use tokio_util::sync::{CancellationToken, DropGuard};

pub fn spawn_blocking_with_cancellation<F, R>(f: F) -> CancellableJoinHandle<R>
where
    F: FnOnce(CancellationToken) -> R + Send + 'static,
    R: Send + 'static,
{
    let token = CancellationToken::new();
    let guard = token.clone().drop_guard();

    let handle = spawn_blocking(move || f(token));

    CancellableJoinHandle {
        handle,
        _guard: guard,
    }
}

pub struct CancellableJoinHandle<R> {
    handle: JoinHandle<R>,
    _guard: DropGuard,
}

impl<R> Future for CancellableJoinHandle<R> {
    type Output = Result<R, JoinError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.handle).poll(cx)
    }
}
