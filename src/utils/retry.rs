use crate::services::backup::logger::JobLogger;
use crate::settings::CONFIG;
use rand::Rng;
use std::fmt::Display;
use std::time::Duration;

pub struct RetryPolicy {
    pub attempts: u32,
    pub base_backoff: Duration,
    pub max_backoff: Duration,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            attempts: CONFIG.retry_attempts,
            base_backoff: Duration::from_millis(CONFIG.retry_backoff_ms),
            max_backoff: Duration::from_secs(30),
        }
    }
}

impl RetryPolicy {
    pub(crate) fn delay(&self, attempt: u32) -> Duration {
        let exp = self.base_backoff.saturating_mul(1u32 << (attempt - 1).min(16));
        let capped = exp.min(self.max_backoff);
        let half = capped / 2;
        let jitter = rand::rng().random_range(0..=half.as_millis() as u64);

        half + Duration::from_millis(jitter)
    }
}

pub async fn retry<T, E, F>(
    op: &str,
    logger: &JobLogger,
    policy: &RetryPolicy,
    mut f: F,
) -> Result<T, E>
where
    F: AsyncFnMut(u32) -> Result<T, E>,
    E: Display,
{
    let total = policy.attempts;
    let mut attempt = 1;

    loop {
        match f(attempt).await {
            Ok(v) => {
                if attempt > 1 {
                    logger.log("info", format!("{op} succeeded on attempt {attempt}/{total}"));
                }
                return Ok(v);
            }
            Err(e) if attempt < total => {
                let delay = policy.delay(attempt);
                logger.log(
                    "warn",
                    format!(
                        "{op} attempt {attempt}/{total} failed: {e} — retrying in {}ms",
                        delay.as_millis()
                    ),
                );
                tokio::time::sleep(delay).await;
                attempt += 1;
            }
            Err(e) => {
                logger.log("error", format!("{op} failed after {total} attempts: {e}"));
                return Err(e);
            }
        }
    }
}
