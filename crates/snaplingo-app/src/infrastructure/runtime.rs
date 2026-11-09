use std::future::Future;
use std::sync::OnceLock;

use tokio::runtime::{Handle, Runtime};
use tokio::task::JoinHandle;

fn runtime() -> &'static Runtime {
    static RUNTIME: OnceLock<Runtime> = OnceLock::new();
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .thread_name("snaplingo-worker")
            .build()
            .expect("Failed to create the application async runtime")
    })
}

pub fn handle() -> Handle {
    runtime().handle().clone()
}

pub fn spawn<F>(future: F) -> JoinHandle<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    runtime().spawn(future)
}

pub fn spawn_blocking<F, T>(operation: F) -> JoinHandle<T>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    runtime().spawn_blocking(operation)
}

pub fn block_on<F: Future>(future: F) -> F::Output {
    runtime().block_on(future)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;
    use std::time::Duration;

    #[test]
    fn runs_background_and_blocking_work_without_a_framework_executor() {
        let (sender, receiver) = mpsc::channel();
        let task = spawn(async move {
            tokio::time::sleep(Duration::from_millis(5)).await;
            let result = spawn_blocking(|| 42).await.unwrap();
            sender.send(result).unwrap();
        });
        assert_eq!(receiver.recv_timeout(Duration::from_secs(5)).unwrap(), 42);
        block_on(task).unwrap();
    }

    #[test]
    fn joins_aborted_tasks_with_native_cancellation() {
        let task = spawn(std::future::pending::<()>());
        task.abort();
        assert!(block_on(task).unwrap_err().is_cancelled());
    }

    #[test]
    fn exposes_the_same_executor_to_the_remaining_ipc_shell() {
        let result = handle().spawn(async {
            tokio::task::yield_now().await;
            std::thread::current().name().unwrap().to_owned()
        });
        assert_eq!(block_on(result).unwrap(), "snaplingo-worker");
    }
}
