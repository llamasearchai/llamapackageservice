use tokio::time::{sleep, Duration};

/// Retry a fallible async operation with exponential backoff
/// 
/// # Arguments
/// * `f` - The async operation to retry
/// * `retries` - Maximum number of retry attempts
/// * `delay` - Base delay duration between retries (will be multiplied by attempt number)
/// 
/// # Returns
/// * `Ok(T)` if the operation succeeds
/// * `Err(E)` if all retry attempts fail
pub async fn with_retry<F, Fut, T, E>(
    f: F,
    retries: u32,
    delay: Duration,
) -> Result<T, E>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
{
    let mut attempts = 0;
    loop {
        match f().await {
            Ok(value) => return Ok(value),
            Err(e) => {
                attempts += 1;
                if attempts >= retries {
                    return Err(e);
                }
                sleep(delay * attempts as u32).await;
            }
        }
    }
}
