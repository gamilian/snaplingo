mod baidu;
#[cfg(feature = "local-ocr")]
pub mod paddle;

pub use baidu::BaiduOcrProvider;
