use tokio::time::{sleep, Duration};

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
