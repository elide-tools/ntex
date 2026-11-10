//! Randomized exponential backoff, matching `backoff` 0.4's `ExponentialBackoff::default()`.

use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub(crate) struct ExponentialBackoff {
    pub(crate) current_interval: Duration,
    pub(crate) initial_interval: Duration,
    pub(crate) randomization_factor: f64,
    pub(crate) multiplier: f64,
    pub(crate) max_interval: Duration,
    pub(crate) start_time: Instant,
    pub(crate) max_elapsed_time: Option<Duration>,
}

impl Default for ExponentialBackoff {
    fn default() -> Self {
        let initial_interval = Duration::from_millis(500);
        Self {
            current_interval: initial_interval,
            initial_interval,
            randomization_factor: 0.5,
            multiplier: 1.5,
            max_interval: Duration::from_secs(60),
            start_time: Instant::now(),
            max_elapsed_time: Some(Duration::from_secs(900)),
        }
    }
}

impl ExponentialBackoff {
    pub(crate) fn reset(&mut self) {
        self.current_interval = self.initial_interval;
        self.start_time = Instant::now();
    }

    pub(crate) fn next_backoff(&mut self) -> Option<Duration> {
        let elapsed = self.start_time.elapsed();
        if self.max_elapsed_time.is_some_and(|max| elapsed > max) {
            return None;
        }
        let randomized = randomize(self.randomization_factor, random(), self.current_interval);
        self.current_interval = self.next_interval();
        match self.max_elapsed_time {
            Some(max) if elapsed + randomized > max => None,
            _ => Some(randomized),
        }
    }

    fn next_interval(&self) -> Duration {
        let current = self.current_interval.as_secs_f64();
        if current >= self.max_interval.as_secs_f64() / self.multiplier {
            self.max_interval
        } else {
            Duration::from_secs_f64(current * self.multiplier)
        }
    }
}

/// Uniform value in `[min, max]` nanoseconds around `interval`.
fn randomize(factor: f64, random: f64, interval: Duration) -> Duration {
    let nanos = interval.as_nanos() as f64;
    let delta = factor * nanos;
    let min = nanos - delta;
    let value = min + random * (2.0 * delta + 1.0);
    Duration::from_nanos(value as u64)
}

/// Non-cryptographic value in `[0, 1)` from std's randomly keyed SipHash.
fn random() -> f64 {
    let mut hasher = RandomState::new().build_hasher();
    hasher.write_u64(0);
    (hasher.finish() >> 11) as f64 / (1u64 << 53) as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_and_growth() {
        let mut b = ExponentialBackoff {
            randomization_factor: 0.0,
            ..Default::default()
        };
        assert_eq!(b.next_backoff(), Some(Duration::from_millis(500)));
        assert_eq!(b.current_interval, Duration::from_millis(750));
        for _ in 0..20 {
            b.next_backoff();
        }
        assert_eq!(b.current_interval, Duration::from_secs(60));
        b.reset();
        assert_eq!(b.current_interval, Duration::from_millis(500));
    }

    #[test]
    fn randomized_range() {
        for _ in 0..100 {
            let d = randomize(0.5, random(), Duration::from_millis(500));
            assert!(d >= Duration::from_millis(250) && d <= Duration::from_millis(751));
        }
    }

    #[test]
    fn max_elapsed() {
        let mut b = ExponentialBackoff {
            max_elapsed_time: Some(Duration::ZERO),
            ..Default::default()
        };
        b.start_time -= Duration::from_millis(1);
        assert_eq!(b.next_backoff(), None);
    }
}
