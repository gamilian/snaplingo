mod port;
mod runtime;

#[cfg(test)]
mod tests;

#[allow(unused_imports)]
pub(crate) use port::{
    ResultWindowClipboardPort, ResultWindowMode, ResultWindowNotifierPort, ResultWindowOcrIntent,
    ResultWindowOpenRequest, ResultWindowPayload, ResultWindowWindowPort,
};
#[allow(unused_imports)]
pub(crate) use runtime::ResultWindowRuntime;
