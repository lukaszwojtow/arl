## ARL - Async Rate Limiter

Arl's main purpose is to simplify dealing with various APIs' limits.
For example, GitHub allows 5000 requests per hour.
Keeping a timer inside your business logic can obscure the main flow and will 
add lots of boilerplate.

With Arl it's possible to limit a task's speed with single line of code.
As an added benefit, the limiter can be cloned and send to other threads/tasks 
and all of them will see the same instance and adhere to the same limits.

Example:

```
    // Create a rate limiter. Limit speed to 10 times in 2 seconds.
    let limiter = RateLimiter::new(10, Duration::from_secs(2));
    // Spawn 4 threads.
    for i in 0..4 {
        // RateLimiter can be cloned - all threads will use the same timer/stats underneath.
        let limiter = limiter.clone();
        tokio::spawn(async move {
            // Create some imaginary client (for a rest service or sth).
            let client = Client::new();
            // Do things in a loop. Notice there is no `sleep` in here.
            loop {
                // Wait if needed. First 10 will be executed immediately.
                limiter.wait().await;
                // Call some api here.
                let response = client.call();
            }
        });
    }
```