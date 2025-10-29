use std::sync::Arc;
use tokio::sync::Semaphore;
use std::future::Future;
use crate::error::ProcessorError;
use std::pin::Pin;

/// Executes tasks in parallel with a specified concurrency limit
pub struct ParallelProcessor {
    max_concurrent: usize,
    semaphore: Arc<Semaphore>,
}

impl ParallelProcessor {
    /// Creates a new parallel processor with the specified concurrency limit
    pub fn new(max_concurrent: usize) -> Self {
        Self {
            max_concurrent,
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
        }
    }

    /// Processes a collection of futures concurrently and returns their results
    pub async fn process<F, T>(&self, tasks: Vec<F>) -> Vec<Result<T, ProcessorError>>
    where
        F: Future<Output = Result<T, ProcessorError>> + Send + 'static,
        T: Send + 'static,
    {
        let mut handles = Vec::with_capacity(tasks.len());
        
        for task in tasks {
            let permit = self.semaphore.clone().acquire_owned().await.unwrap();
            handles.push(tokio::spawn(async move {
                let result = task.await;
                drop(permit);
                result
            }));
        }

        let mut results = Vec::with_capacity(handles.len());
        for handle in handles {
            results.push(handle.await.unwrap());
        }
        results
    }
} 

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::sleep;
    use std::time::Duration;

    #[tokio::test]
    async fn test_parallel_processing() {
        let processor = ParallelProcessor::new(3);
        
        // Create closure functions with the same signature
        let make_task = |duration: u64, value: i32| async move {
            sleep(Duration::from_millis(duration)).await;
            Ok::<_, ProcessorError>(value)
        };
        
        let tasks: Vec<_> = vec![
            Box::pin(make_task(100, 1)),
            Box::pin(make_task(50, 2)),
            Box::pin(make_task(200, 3)),
            Box::pin(make_task(75, 4)),
            Box::pin(make_task(150, 5)),
        ];
        
        let results = processor.process(tasks).await;
        
        assert_eq!(results.len(), 5);
        for result in results {
            assert!(result.is_ok());
        }
    }

    #[tokio::test]
    async fn test_error_handling() {
        let processor = ParallelProcessor::new(2);
        
        // Helper to box futures into a common trait-object type
        fn boxed_future<T>(fut: impl Future<Output = Result<T, ProcessorError>> + Send + 'static) -> Pin<Box<dyn Future<Output = Result<T, ProcessorError>> + Send>> {
            Box::pin(fut)
        }

        let tasks: Vec<_> = vec![
            boxed_future(async { Ok::<_, ProcessorError>(1) }),
            boxed_future(async { Err::<i32, _>(ProcessorError::new("Test error")) }),
            boxed_future(async { Ok::<_, ProcessorError>(3) }),
        ];
        
        let results = processor.process(tasks).await;
        
        assert_eq!(results.len(), 3);
        assert!(results[0].is_ok());
        assert!(results[1].is_err());
        assert!(results[2].is_ok());
    }
}
