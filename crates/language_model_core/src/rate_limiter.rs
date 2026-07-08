use async_io::Timer;
use async_lock::{Semaphore, SemaphoreGuardArc};
use futures::Stream;
use std::{
    cmp, fmt,
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex, MutexGuard},
    task::{Context, Poll},
    time::{Duration, Instant},
};

use crate::LanguageModelCompletionError;

#[derive(Clone)]
pub struct RateLimiter {
    semaphore: Arc<Semaphore>,
    config: RateLimiterConfig,
    window: Arc<Mutex<RateLimitWindow>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RateLimiterConfig {
    pub max_concurrent_requests: usize,
    pub requests_per_minute: Option<u32>,
    pub burst_size: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RateLimitStatus {
    pub requests_per_minute: Option<u32>,
    pub burst_size: u32,
    pub remaining_burst_requests: u32,
    pub retry_after: Option<Duration>,
}

struct RateLimitWindow {
    available_requests: f64,
    last_refill: Instant,
}

pub struct RateLimitGuard<T> {
    inner: T,
    _guard: SemaphoreGuardArc,
}

impl<T> Stream for RateLimitGuard<T>
where
    T: Stream,
{
    type Item = T::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        unsafe { Pin::map_unchecked_mut(self, |this| &mut this.inner).poll_next(cx) }
    }
}

impl RateLimiter {
    pub fn new(limit: usize) -> Self {
        Self::with_config(RateLimiterConfig::concurrent(limit))
    }

    pub fn with_config(config: RateLimiterConfig) -> Self {
        let config = config.normalized();
        let available_requests = config.burst_size as f64;
        Self {
            semaphore: Arc::new(Semaphore::new(config.max_concurrent_requests)),
            config,
            window: Arc::new(Mutex::new(RateLimitWindow {
                available_requests,
                last_refill: Instant::now(),
            })),
        }
    }

    pub fn status(&self) -> RateLimitStatus {
        let mut window = self.lock_window();
        self.refill(&mut window);
        let retry_after = self.retry_after(&window);

        RateLimitStatus {
            requests_per_minute: self.config.requests_per_minute,
            burst_size: self.config.burst_size,
            remaining_burst_requests: window.available_requests.floor() as u32,
            retry_after,
        }
    }

    pub fn run<'a, Fut, T>(
        &self,
        future: Fut,
    ) -> impl 'a + Future<Output = Result<T, LanguageModelCompletionError>>
    where
        Fut: 'a + Future<Output = Result<T, LanguageModelCompletionError>>,
    {
        let guard = self.semaphore.acquire_arc();
        let limiter = self.clone();
        async move {
            limiter.acquire_rate_slot().await;
            let guard = guard.await;
            let result = future.await?;
            drop(guard);
            Ok(result)
        }
    }

    pub fn stream<'a, Fut, T>(
        &self,
        future: Fut,
    ) -> impl 'a
    + Future<
        Output = Result<impl Stream<Item = T::Item> + use<Fut, T>, LanguageModelCompletionError>,
    >
    where
        Fut: 'a + Future<Output = Result<T, LanguageModelCompletionError>>,
        T: Stream,
    {
        let guard = self.semaphore.acquire_arc();
        let limiter = self.clone();
        async move {
            limiter.acquire_rate_slot().await;
            let guard = guard.await;
            let inner = future.await?;
            Ok(RateLimitGuard {
                inner,
                _guard: guard,
            })
        }
    }

    async fn acquire_rate_slot(&self) {
        loop {
            let delay = {
                let mut window = self.lock_window();
                self.refill(&mut window);

                if window.available_requests >= 1.0 {
                    window.available_requests -= 1.0;
                    None
                } else {
                    self.retry_after(&window)
                }
            };

            match delay {
                Some(delay) if !delay.is_zero() => {
                    Timer::after(delay).await;
                }
                Some(_) => continue,
                None => break,
            }
        }
    }

    fn refill(&self, window: &mut RateLimitWindow) {
        let Some(requests_per_minute) = self.config.requests_per_minute else {
            window.available_requests = self.config.burst_size as f64;
            window.last_refill = Instant::now();
            return;
        };

        let now = Instant::now();
        let elapsed = now.saturating_duration_since(window.last_refill);
        window.last_refill = now;

        let requests_per_second = requests_per_minute as f64 / 60.0;
        let refill = elapsed.as_secs_f64() * requests_per_second;
        window.available_requests =
            (window.available_requests + refill).min(self.config.burst_size as f64);
    }

    fn retry_after(&self, window: &RateLimitWindow) -> Option<Duration> {
        let requests_per_minute = self.config.requests_per_minute?;
        if window.available_requests >= 1.0 {
            return None;
        }

        let requests_per_second = requests_per_minute as f64 / 60.0;
        let seconds = (1.0 - window.available_requests) / requests_per_second;
        Some(Duration::from_secs_f64(seconds.max(0.0)))
    }

    fn lock_window(&self) -> MutexGuard<'_, RateLimitWindow> {
        match self.window.lock() {
            Ok(window) => window,
            Err(poisoned) => poisoned.into_inner(),
        }
    }
}

impl RateLimiterConfig {
    pub fn concurrent(max_concurrent_requests: usize) -> Self {
        Self {
            max_concurrent_requests,
            requests_per_minute: None,
            burst_size: 1,
        }
    }

    pub fn requests_per_minute(max_concurrent_requests: usize, requests_per_minute: u32) -> Self {
        Self {
            max_concurrent_requests,
            requests_per_minute: Some(requests_per_minute),
            burst_size: cmp::max(1, requests_per_minute),
        }
    }

    pub fn with_burst_size(mut self, burst_size: u32) -> Self {
        self.burst_size = burst_size;
        self.normalized()
    }

    fn normalized(mut self) -> Self {
        self.max_concurrent_requests = cmp::max(1, self.max_concurrent_requests);
        self.burst_size = cmp::max(1, self.burst_size);
        if self.requests_per_minute == Some(0) {
            self.requests_per_minute = None;
        }
        self
    }
}

impl fmt::Debug for RateLimiter {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RateLimiter")
            .field("config", &self.config)
            .field("status", &self.status())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::future::join_all;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn new_preserves_concurrency_only_defaults() {
        let limiter = RateLimiter::new(4);
        assert_eq!(
            limiter.status(),
            RateLimitStatus {
                requests_per_minute: None,
                burst_size: 1,
                remaining_burst_requests: 1,
                retry_after: None,
            }
        );
    }

    #[test]
    fn config_normalizes_zero_values() {
        let config = RateLimiterConfig {
            max_concurrent_requests: 0,
            requests_per_minute: Some(0),
            burst_size: 0,
        };

        assert_eq!(
            config.normalized(),
            RateLimiterConfig {
                max_concurrent_requests: 1,
                requests_per_minute: None,
                burst_size: 1,
            }
        );
    }

    #[test]
    fn status_reports_exhausted_burst() {
        smol::block_on(async {
            let limiter = RateLimiter::with_config(RateLimiterConfig::requests_per_minute(1, 60));

            limiter
                .run(async { Ok(()) })
                .await
                .expect("first request should run");
            let status = limiter.status();

            assert_eq!(status.remaining_burst_requests, 59);
            assert_eq!(status.retry_after, None);
        });
    }

    #[test]
    fn waits_when_burst_is_exhausted() {
        smol::block_on(async {
            let limiter = RateLimiter::with_config(
                RateLimiterConfig::requests_per_minute(1, 60).with_burst_size(1),
            );
            limiter
                .run(async { Ok(()) })
                .await
                .expect("first request should run");

            let started = Instant::now();
            limiter
                .run(async { Ok(()) })
                .await
                .expect("second request should run after rate-limit delay");

            assert!(started.elapsed() >= Duration::from_millis(900));
        });
    }

    #[test]
    fn still_limits_concurrent_requests() {
        smol::block_on(async {
            let limiter = RateLimiter::new(2);
            let active = Arc::new(AtomicUsize::new(0));
            let max_active = Arc::new(AtomicUsize::new(0));

            let tasks = (0..6).map(|_| {
                let active = active.clone();
                let max_active = max_active.clone();
                limiter.run(async move {
                    let current = active.fetch_add(1, Ordering::SeqCst) + 1;
                    max_active.fetch_max(current, Ordering::SeqCst);
                    Timer::after(Duration::from_millis(10)).await;
                    active.fetch_sub(1, Ordering::SeqCst);
                    Ok(())
                })
            });

            join_all(tasks).await.into_iter().for_each(|result| {
                result.expect("limited request should complete");
            });

            assert_eq!(max_active.load(Ordering::SeqCst), 2);
        });
    }
}
