mod port;
mod runtime;

#[cfg(test)]
mod tests;

pub(crate) use port::{
    ResultWindowClipboardPort, ResultWindowMode, ResultWindowNotifierPort, ResultWindowOcrIntent,
    ResultWindowOpenRequest, ResultWindowPayload, ResultWindowWindowPort,
};
pub(crate) use runtime::ResultWindowRuntime;
