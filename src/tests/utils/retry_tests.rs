use crate::services::backup::logger::JobLogger;
use crate::tests::init_tracing_for_test;
use crate::utils::retry::{RetryPolicy, retry};

use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

fn fast_policy(attempts: u32) -> RetryPolicy {
    RetryPolicy {
        attempts,
        base_backoff: Duration::from_millis(1),
        max_backoff: Duration::from_millis(4),
    }
}

#[test]
fn delay_grows_with_the_attempt_number() {
    let policy = RetryPolicy {
        attempts: 5,
        base_backoff: Duration::from_millis(100),
        max_backoff: Duration::from_secs(30),
    };

    assert!(policy.delay(1) >= Duration::from_millis(50));
    assert!(policy.delay(1) <= Duration::from_millis(100));
    assert!(policy.delay(2) >= Duration::from_millis(100));
    assert!(policy.delay(2) <= Duration::from_millis(200));
    assert!(policy.delay(3) >= Duration::from_millis(200));
    assert!(policy.delay(3) <= Duration::from_millis(400));
}

#[test]
fn delay_never_exceeds_max_backoff() {
    let policy = RetryPolicy {
        attempts: 5,
        base_backoff: Duration::from_millis(1000),
        max_backoff: Duration::from_millis(2000),
    };

    for attempt in 1..=5 {
        assert!(policy.delay(attempt) <= Duration::from_millis(2000));
    }
}

#[tokio::test]
async fn first_attempt_success_logs_nothing() {
    init_tracing_for_test();
    let logger = JobLogger::new();

    let result: Result<u32, anyhow::Error> =
        retry("Test op", &logger, &fast_policy(3), |_| async { Ok(7) }).await;

    assert_eq!(result.unwrap(), 7);
    assert!(logger.into_entries().is_empty());
}

#[tokio::test]
async fn retries_until_success_and_logs_each_attempt() {
    init_tracing_for_test();
    let logger = JobLogger::new();
    let calls = AtomicU32::new(0);

    let result: Result<u32, anyhow::Error> =
        retry("Test op", &logger, &fast_policy(3), |_| async {
            let n = calls.fetch_add(1, Ordering::SeqCst) + 1;
            if n < 3 {
                Err(anyhow::anyhow!("boom {n}"))
            } else {
                Ok(n)
            }
        })
        .await;

    assert_eq!(result.unwrap(), 3);
    assert_eq!(calls.load(Ordering::SeqCst), 3);

    let entries = logger.into_entries();

    let warns: Vec<_> = entries.iter().filter(|e| e.level == "warn").collect();
    assert_eq!(warns.len(), 2);
    assert!(warns[0].message.starts_with("Test op attempt 1/3 failed: boom 1"));
    assert!(warns[1].message.starts_with("Test op attempt 2/3 failed: boom 2"));

    let infos: Vec<_> = entries.iter().filter(|e| e.level == "info").collect();
    assert_eq!(infos.len(), 1);
    assert_eq!(infos[0].message, "Test op succeeded on attempt 3/3");
}

#[tokio::test]
async fn exhausts_attempts_and_logs_a_single_error() {
    init_tracing_for_test();
    let logger = JobLogger::new();
    let calls = AtomicU32::new(0);

    let result: Result<(), anyhow::Error> =
        retry("Test op", &logger, &fast_policy(3), |_| async {
            calls.fetch_add(1, Ordering::SeqCst);
            Err(anyhow::anyhow!("always"))
        })
        .await;

    assert!(result.is_err());
    assert_eq!(calls.load(Ordering::SeqCst), 3);

    let entries = logger.into_entries();
    assert_eq!(entries.iter().filter(|e| e.level == "warn").count(), 2);
    assert_eq!(entries.iter().filter(|e| e.level == "error").count(), 1);
    assert_eq!(
        entries.iter().find(|e| e.level == "error").unwrap().message,
        "Test op failed after 3 attempts: always"
    );
}

#[tokio::test]
async fn closure_receives_the_attempt_number() {
    init_tracing_for_test();
    let logger = JobLogger::new();
    let seen = Mutex::new(Vec::new());
    let seen_ref = &seen;

    let result: Result<(), anyhow::Error> =
        retry("Test op", &logger, &fast_policy(3), move |attempt| async move {
            seen_ref.lock().unwrap().push(attempt);
            Err(anyhow::anyhow!("nope"))
        })
        .await;

    assert!(result.is_err());
    assert_eq!(*seen.lock().unwrap(), vec![1, 2, 3]);
}

#[test]
fn config_defaults_are_within_the_documented_range() {
    let policy = RetryPolicy::default();
    assert!(policy.attempts >= 3 && policy.attempts <= 5);
    assert!(policy.base_backoff >= Duration::from_millis(100));
    assert!(policy.base_backoff <= Duration::from_millis(30_000));
}
