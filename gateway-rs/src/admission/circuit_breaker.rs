//! A three-state circuit breaker (closed → open → half-open → closed).
//!
//! Closed: requests flow; consecutive failures are counted.
//! Open:   requests are rejected fast until the cooldown elapses.
//! HalfOpen: a limited number of trial requests are allowed; enough successes
//!           close the breaker, any failure re-opens it.
//!
//! The critical sections are tiny and never span an `.await`, so a
//! `std::sync::Mutex` is the right tool.

use crate::config::CircuitBreakerConfig;
use std::sync::Mutex;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Closed,
    Open,
    HalfOpen,
}

impl State {
    /// Numeric encoding for the Prometheus gauge.
    pub fn code(self) -> i64 {
        match self {
            State::Closed => 0,
            State::HalfOpen => 1,
            State::Open => 2,
        }
    }
}

#[derive(Debug)]
struct Inner {
    state: State,
    consecutive_failures: usize,
    half_open_successes: usize,
    half_open_probe_inflight: bool,
    opened_at: Option<Instant>,
}

pub struct CircuitBreaker {
    inner: Mutex<Inner>,
    failure_threshold: usize,
    open_cooldown: Duration,
    half_open_success_threshold: usize,
}

impl CircuitBreaker {
    pub fn new(cfg: &CircuitBreakerConfig) -> Self {
        Self {
            inner: Mutex::new(Inner {
                state: State::Closed,
                consecutive_failures: 0,
                half_open_successes: 0,
                half_open_probe_inflight: false,
                opened_at: None,
            }),
            failure_threshold: cfg.failure_threshold,
            open_cooldown: Duration::from_millis(cfg.open_cooldown_ms),
            half_open_success_threshold: cfg.half_open_success_threshold,
        }
    }

    /// Decide whether a request may proceed. Transitions Open → HalfOpen once
    /// the cooldown has elapsed (and admits that first trial request).
    pub fn allow(&self) -> bool {
        let mut g = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        match g.state {
            State::Closed => true,
            State::HalfOpen if !g.half_open_probe_inflight => {
                g.half_open_probe_inflight = true;
                true
            }
            State::HalfOpen => false,
            State::Open => {
                let elapsed = g
                    .opened_at
                    .map(|t| t.elapsed())
                    .unwrap_or(self.open_cooldown);
                if elapsed >= self.open_cooldown {
                    g.state = State::HalfOpen;
                    g.half_open_successes = 0;
                    g.half_open_probe_inflight = true;
                    true
                } else {
                    false
                }
            }
        }
    }

    pub fn on_success(&self) {
        let mut g = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        match g.state {
            State::Closed => {
                g.consecutive_failures = 0;
            }
            State::HalfOpen => {
                g.half_open_probe_inflight = false;
                g.half_open_successes += 1;
                if g.half_open_successes >= self.half_open_success_threshold {
                    g.state = State::Closed;
                    g.consecutive_failures = 0;
                    g.half_open_successes = 0;
                    g.opened_at = None;
                }
            }
            State::Open => {}
        }
    }

    pub fn on_failure(&self) {
        let mut g = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        match g.state {
            State::Closed => {
                g.consecutive_failures += 1;
                if g.consecutive_failures >= self.failure_threshold {
                    g.state = State::Open;
                    g.opened_at = Some(Instant::now());
                }
            }
            State::HalfOpen => {
                // A failed trial immediately re-opens the breaker.
                g.state = State::Open;
                g.opened_at = Some(Instant::now());
                g.half_open_successes = 0;
                g.half_open_probe_inflight = false;
            }
            State::Open => {}
        }
    }

    pub fn state(&self) -> State {
        self.inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .state
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> CircuitBreakerConfig {
        CircuitBreakerConfig {
            failure_threshold: 3,
            open_cooldown_ms: 20,
            half_open_success_threshold: 2,
        }
    }

    #[test]
    fn opens_after_threshold_failures() {
        let cb = CircuitBreaker::new(&cfg());
        assert!(cb.allow());
        cb.on_failure();
        cb.on_failure();
        assert_eq!(cb.state(), State::Closed);
        cb.on_failure(); // 3rd
        assert_eq!(cb.state(), State::Open);
        assert!(!cb.allow());
    }

    #[test]
    fn half_open_recovers_to_closed() {
        let cb = CircuitBreaker::new(&cfg());
        for _ in 0..3 {
            cb.on_failure();
        }
        assert_eq!(cb.state(), State::Open);
        std::thread::sleep(Duration::from_millis(25));
        // Cooldown elapsed → allow() flips to half-open and admits a trial.
        assert!(cb.allow());
        assert_eq!(cb.state(), State::HalfOpen);
        cb.on_success();
        assert!(cb.allow());
        cb.on_success(); // meets half_open_success_threshold
        assert_eq!(cb.state(), State::Closed);
    }

    #[test]
    fn half_open_failure_reopens() {
        let cb = CircuitBreaker::new(&cfg());
        for _ in 0..3 {
            cb.on_failure();
        }
        std::thread::sleep(Duration::from_millis(25));
        assert!(cb.allow());
        assert_eq!(cb.state(), State::HalfOpen);
        cb.on_failure();
        assert_eq!(cb.state(), State::Open);
    }

    #[test]
    fn success_in_closed_resets_failures() {
        let cb = CircuitBreaker::new(&cfg());
        cb.on_failure();
        cb.on_failure();
        cb.on_success();
        cb.on_failure();
        cb.on_failure();
        // Only two consecutive failures after the reset → still closed.
        assert_eq!(cb.state(), State::Closed);
    }

    #[test]
    fn half_open_allows_only_one_probe_at_a_time() {
        let cb = CircuitBreaker::new(&cfg());
        for _ in 0..3 {
            cb.on_failure();
        }
        std::thread::sleep(Duration::from_millis(25));
        assert!(cb.allow());
        assert!(!cb.allow());
        cb.on_success();
        assert!(cb.allow());
    }
}
