//! Bounded admission with backpressure.
//!
//! Each model has a concurrency limit (`max_concurrency` permits) and a bounded
//! waiting room (`queue_capacity`). The flow is:
//!
//! 1. Reserve a queue slot. If the waiting room is full → reject (`QueueFull`).
//! 2. Wait for a concurrency permit, up to `queue_timeout`. On timeout →
//!    reject (`QueueTimeout`).
//! 3. On success, leave the waiting room and hold the concurrency permit until
//!    the request finishes (the permit is released on drop — including when the
//!    client disconnects and the handler future is cancelled).
//!
//! This makes overload observable and bounded instead of unbounded latency
//! growth: the process never queues more than `queue_capacity` waiters per
//! model, and never runs more than `max_concurrency` upstream calls.

use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdmitReject {
    QueueFull,
    QueueTimeout,
    /// The semaphore was closed (shutting down).
    Closed,
}

/// RAII proof that a request is admitted and occupying one concurrency slot.
/// Dropping it releases the slot and decrements the in-flight gauge callback.
pub struct AdmissionPermit {
    _permit: OwnedSemaphorePermit,
    inflight: Arc<AtomicI64>,
}

impl Drop for AdmissionPermit {
    fn drop(&mut self) {
        self.inflight.fetch_sub(1, Ordering::Relaxed);
    }
}

pub struct Admission {
    concurrency: Arc<Semaphore>,
    max_queue: usize,
    queue_timeout: Duration,
    /// Number of requests currently waiting for a permit.
    queue_depth: Arc<AtomicI64>,
    /// Number of requests currently holding a permit.
    inflight: Arc<AtomicI64>,
}

impl Admission {
    pub fn new(max_concurrency: usize, max_queue: usize, queue_timeout: Duration) -> Self {
        Self {
            concurrency: Arc::new(Semaphore::new(max_concurrency)),
            max_queue,
            queue_timeout,
            queue_depth: Arc::new(AtomicI64::new(0)),
            inflight: Arc::new(AtomicI64::new(0)),
        }
    }

    pub fn queue_depth(&self) -> i64 {
        self.queue_depth.load(Ordering::Relaxed)
    }

    pub fn inflight(&self) -> i64 {
        self.inflight.load(Ordering::Relaxed)
    }

    /// Try to admit a request. On success returns a permit that must be held for
    /// the lifetime of the upstream call.
    pub async fn admit(&self) -> Result<AdmissionPermit, AdmitReject> {
        // Step 1: reserve a slot in the bounded waiting room.
        let depth = self.queue_depth.fetch_add(1, Ordering::AcqRel);
        if depth as usize >= self.max_queue {
            self.queue_depth.fetch_sub(1, Ordering::AcqRel);
            return Err(AdmitReject::QueueFull);
        }

        // Step 2: wait for a concurrency permit, bounded by the queue timeout.
        let acquire = self.concurrency.clone().acquire_owned();
        let result = tokio::time::timeout(self.queue_timeout, acquire).await;

        // Leaving the waiting room regardless of outcome.
        self.queue_depth.fetch_sub(1, Ordering::AcqRel);

        match result {
            Ok(Ok(permit)) => {
                self.inflight.fetch_add(1, Ordering::Relaxed);
                Ok(AdmissionPermit {
                    _permit: permit,
                    inflight: self.inflight.clone(),
                })
            }
            Ok(Err(_closed)) => Err(AdmitReject::Closed),
            Err(_elapsed) => Err(AdmitReject::QueueTimeout),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn admits_up_to_concurrency() {
        let a = Admission::new(2, 4, Duration::from_millis(50));
        let p1 = a.admit().await.unwrap();
        let p2 = a.admit().await.unwrap();
        assert_eq!(a.inflight(), 2);
        drop(p1);
        drop(p2);
        assert_eq!(a.inflight(), 0);
    }

    #[tokio::test]
    async fn rejects_when_queue_full() {
        // concurrency 1, queue 1. Hold the one permit, fill the one queue slot,
        // then the next admit must be rejected as QueueFull.
        let a = Arc::new(Admission::new(1, 1, Duration::from_secs(10)));
        let held = a.admit().await.unwrap();

        // This one enters the waiting room and blocks on the permit.
        let a2 = a.clone();
        let waiter = tokio::spawn(async move { a2.admit().await });
        // Give the waiter time to occupy the queue slot.
        tokio::time::sleep(Duration::from_millis(20)).await;
        assert_eq!(a.queue_depth(), 1);

        // Waiting room is now full → immediate rejection.
        let rejected = a.admit().await;
        assert_eq!(rejected.err(), Some(AdmitReject::QueueFull));

        drop(held);
        let admitted = waiter.await.unwrap();
        assert!(admitted.is_ok());
    }

    #[tokio::test]
    async fn times_out_in_queue() {
        let a = Admission::new(1, 4, Duration::from_millis(30));
        let _held = a.admit().await.unwrap();
        let start = std::time::Instant::now();
        let r = a.admit().await;
        assert_eq!(r.err(), Some(AdmitReject::QueueTimeout));
        assert!(start.elapsed() >= Duration::from_millis(30));
    }
}
