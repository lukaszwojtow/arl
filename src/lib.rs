//! Async rate limiter for tokio runtime.
//! Used to prevent DOSing a remote service or limiting usage of some other resources.

#![forbid(unsafe_code)]
#![warn(
    anonymous_parameters,
    clippy::needless_borrow,
    missing_docs,
    missing_copy_implementations,
    missing_debug_implementations,
    nonstandard_style,
    rust_2018_idioms,
    single_use_lifetimes,
    trivial_casts,
    trivial_numeric_casts,
    unreachable_pub,
    unused_extern_crates,
    unused_qualifications,
    variant_size_differences
)]

use std::time::{Duration, Instant};
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::sync::oneshot;
use tokio::time::sleep;

/// A rate limiter, or `time barrier` to prevent continuing execution before given time passes.
/// Used mainly for things like dealing with remote API DOS protection.
#[derive(Clone, Debug)]
pub struct RateLimiter {
    sender: Sender<Message>,
}

impl RateLimiter {
    /// Creates a new RateLimiter that prevents a async task from continuing too quickly.
    /// # Example:
    /// ```no_run
    /// use std::time::Duration;
    /// use arl::RateLimiter;
    /// let limiter = RateLimiter::new(75, Duration::from_secs(60));
    /// async {
    /// loop {
    ///         limiter.wait().await;
    ///         // Call a remote api here.
    ///         // This will ensure that the remote api will not be hit more than 75 times in 60 seconds block.
    ///      }
    /// };
    ///```
    /// RateLimiter can be cloned and send to other threads: it will use the same counter and limits
    /// for all the threads.
    pub fn new(count: usize, duration: Duration) -> Self {
        let (sender, receiver) = channel(count);
        RateLimiter::spawn_receiver(receiver, count, duration);
        Self { sender }
    }

    /// Make the current task wait until given limits have passed.
    /// Uses `tokio::time::sleep()` internally, so it allows other tasks to continue in the meantime.
    /// # Example:
    /// ```no_run
    /// use std::time::Duration;
    /// use arl::RateLimiter;
    /// let limiter = RateLimiter::new(2, Duration::from_secs(1));
    /// async {
    ///     loop {
    ///         limiter.wait().await;
    ///         // continue here knowing that it won't be executed more than twice in a second
    ///     }   
    /// };
    /// ```
    pub async fn wait(&self) {
        let (s, r) = oneshot::channel::<()>();
        self.sender
            .send(Message { sender: s })
            .await
            .expect("unable to send to arl channel");
        r.await.expect("unable to read from arl channel");
    }

    fn spawn_receiver(mut receiver: Receiver<Message>, count: usize, duration: Duration) {
        tokio::spawn(async move {
            let mut queue = Vec::with_capacity(count);
            while let Some(message) = receiver.recv().await {
                if queue.len() >= count {
                    let alarm = queue[0] + duration;
                    sleep(alarm - Instant::now()).await;
                }
                message
                    .sender
                    .send(())
                    .expect("unable to send to arl client channel");
                queue.push(Instant::now());
                while !queue.is_empty() && queue[0] <= Instant::now() - duration {
                    queue.remove(0);
                }
            }
        });
    }
}

#[derive(Debug)]
struct Message {
    sender: oneshot::Sender<()>,
}

#[cfg(test)]
mod test {
    use crate::RateLimiter;
    use std::time::Duration;
    use tokio::time::Instant;

    #[tokio::test]
    async fn up_to_limit_execute_quickly() {
        const COUNT: usize = 10;
        let limiter = RateLimiter::new(COUNT, Duration::from_secs(60));
        let start = Instant::now();
        for _ in 0..COUNT {
            limiter.wait().await;
        }
        let elapsed = start.elapsed();
        assert!(elapsed < Duration::from_millis(10));
    }

    #[tokio::test]
    async fn over_limit_execute_proportionally() {
        const COUNT: usize = 10;
        const CHUNKS: usize = 3;
        let limiter = RateLimiter::new(COUNT, Duration::from_secs(1));
        let start = Instant::now();
        for _ in 0..CHUNKS {
            for _ in 0..COUNT {
                limiter.wait().await;
            }
        }
        let elapsed = start.elapsed();
        // Time below compared to 2 seconds:
        // First chunk (10 calls to wait()) was executed immediately,
        // Second chunk executed after 1 seconds.
        // Third chunk executed after 2 seconds.
        assert!(elapsed > Duration::from_secs(CHUNKS as u64 - 1));
    }
}
