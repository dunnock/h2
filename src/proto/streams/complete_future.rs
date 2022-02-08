use core::fmt;
use std::{
    pin::Pin,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    task::{Context, Poll},
};

use futures_util::task::AtomicWaker;

/// struct that implements Future which will be resolved
/// when Completehandle.resolve() called
#[derive(Debug)]
pub struct Completion {
    inner: Arc<CompleteInner>,
}

impl Completion {
    fn is_completed(&self) -> bool {
        self.inner.completed.load(Ordering::Acquire)
    }
}

pub struct CompleteHandle {
    inner: Arc<CompleteInner>,
}

impl fmt::Debug for CompleteHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "CompleteHandle completed={}",
            self.inner.completed.load(Ordering::Relaxed)
        )
    }
}

impl CompleteHandle {
    /// Creates an (`CompleteHandle`, `Complete`) pair which can be used
    /// to resolve a running future
    pub fn new_pair() -> (Self, Completion) {
        let inner = Arc::new(CompleteInner {
            waker: AtomicWaker::new(),
            completed: AtomicBool::new(false),
        });

        (
            Self {
                inner: inner.clone(),
            },
            Completion { inner },
        )
    }
}

// Inner type storing the waker to awaken and a bool indicating that it
// should be aborted.
#[derive(Debug)]
struct CompleteInner {
    waker: AtomicWaker,
    completed: AtomicBool,
}

impl std::future::Future for Completion {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // Check if the task has been aborted
        if self.is_completed() {
            return Poll::Ready(());
        }

        // Register to receive a wakeup if the task is aborted in the future
        self.inner.waker.register(cx.waker());

        // Check to see if the task was resolved between the first check and
        // registration.
        if self.is_completed() {
            return Poll::Ready(());
        }

        Poll::Pending
    }
}

impl CompleteHandle {
    /// Notify Completion future that task is resolved
    pub fn resolve(&self) {
        self.inner.completed.store(true, Ordering::Release);
        self.inner.waker.wake();
    }
}
