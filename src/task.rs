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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::sync::oneshot;

    #[tokio::test]
    async fn when_task_completes_then_it_should_return_result() {
        let task = |_| 42;

        let handle = spawn_blocking_with_cancellation(task);
        let result = handle.await.unwrap();

        assert_eq!(result, 42);
    }

    #[tokio::test]
    async fn when_handle_is_dropped_then_token_should_be_cancelled() {
        let (tx, rx) = oneshot::channel();
        let handle = spawn_blocking_with_cancellation(move |token| {
            while !token.is_cancelled() {
                std::thread::sleep(Duration::from_millis(1));
            }
            let _ = tx.send(());
        });

        drop(handle);

        let result = tokio::time::timeout(Duration::from_secs(5), rx).await;
        assert!(result.is_ok());
    }
}
