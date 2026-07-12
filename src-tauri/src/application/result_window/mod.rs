#[cfg_attr(not(test), allow(dead_code))]
mod port;
#[cfg_attr(not(test), allow(dead_code))]
mod runtime;

#[cfg(test)]
mod tests;

#[cfg_attr(not(test), allow(unused_imports))]
pub(crate) use port::{
    ResultWindowClipboardPort, ResultWindowMode, ResultWindowNotifierPort, ResultWindowOcrIntent,
    ResultWindowOpenRequest, ResultWindowPayload, ResultWindowWindowPort,
};
#[cfg_attr(not(test), allow(unused_imports))]
pub(crate) use runtime::ResultWindowRuntime;
