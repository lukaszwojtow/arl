use arl::RateLimiter;
use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::Duration;

#[tokio::main]
async fn main() {
    // Limit rate to 10 times in 2 seconds.
    let limiter = RateLimiter::new(10, Duration::from_secs(2));
    let counter = Arc::new(Mutex::new(0));
    // Spawn 4 threads.
    for i in 0..4 {
        // RateLimiter can be cloned - all threads will use the same timer/stats underneath.
        let limiter = limiter.clone();
        let counter = counter.clone();
        tokio::spawn(async move {
            // Do things in a loop. Notice there is no `sleep` in here.
            loop {
                // Wait if needed. First 10 will be executed immediately.
                limiter.wait().await;
                // Increment and show counter just for fun.
                let mut c = counter.lock().unwrap();
                *c += 1;
                println!("Counter: {} by thread #{}", c, i);
            }
        });
    }
    // Let other threads do some work.
    sleep(Duration::from_secs(21));
}
