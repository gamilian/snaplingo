//! AppKit work is queued directly on the process main dispatch queue.
//! The queue is owned by macOS and does not require a Tauri event loop.

pub(crate) fn dispatch(work: impl FnOnce() + Send + 'static) {
    dispatch2::DispatchQueue::main().exec_async(work);
}

pub(crate) async fn run<T: Send + 'static>(
    operation_name: &'static str,
    work: impl FnOnce() -> std::result::Result<T, String> + Send + 'static,
) -> crate::Result<T> {
    let (sender, receiver) = tokio::sync::oneshot::channel();
    dispatch(move || {
        // A canceled caller must not create or mutate a native window later.
        if !sender.is_closed() {
            let _ = sender.send(work());
        }
    });
    receiver
        .await
        .map_err(|error| format!("Failed to receive {operation_name} result: {error}"))?
        .map_err(Into::into)
}
