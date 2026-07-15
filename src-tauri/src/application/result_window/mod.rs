#[cfg_attr(not(test), allow(dead_code))]
mod port;
#[cfg_attr(not(test), allow(dead_code))]
mod runtime;

#[cfg(test)]
mod tests;

#[cfg_attr(not(test), allow(unused_imports))]
pub(crate) use port::{
    ResultWindowMode, ResultWindowNotifierPort, ResultWindowOcrIntent, ResultWindowOpenRequest,
    ResultWindowOrigin, ResultWindowPayload, ResultWindowRequestId, ResultWindowWindowPort,
};
#[cfg_attr(not(test), allow(unused_imports))]
pub(crate) use runtime::ResultWindowRuntime;
