#[cfg(target_os = "macos")]
mod mnn_engine;
mod ort_engine;
#[cfg(test)]
mod tests;

use std::path::PathBuf;
use std::sync::Arc;

use image::RgbImage;
use parking_lot::{Mutex, RwLock};

use crate::{AppError, Result};
use snaplingo_core::application::providers::ocr::{
    OcrAccelerationStatus, OcrExecutionDevice, PaddleOcrEngine,
};
use snaplingo_core::domain::ocr::{OcrBoundingBox, OcrLine, OcrRequest, OcrResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OcrBackend {
    OrtCpu,
    CoreMl,
    DirectMl,
    MnnCpu,
}

impl OcrBackend {
    fn is_gpu(self) -> bool {
        matches!(self, Self::CoreMl | Self::DirectMl)
    }
}

trait MediumSession: Send {
    fn recognize(&mut self, image: &RgbImage) -> Result<OcrResult>;
}

trait SessionFactory: Send + Sync {
    fn is_available(&self) -> bool;
    fn cpu_backend(&self) -> OcrBackend;
    fn gpu_backend(&self) -> Option<OcrBackend>;
    fn create(&self, backend: OcrBackend) -> Result<Box<dyn MediumSession>>;
}

struct SessionState {
    gpu_enabled: Option<bool>,
    backend: Option<OcrBackend>,
    session: Option<Box<dyn MediumSession>>,
}

pub(crate) struct LocalPaddleOcrEngine {
    factory: Arc<dyn SessionFactory>,
    state: Mutex<SessionState>,
    status: RwLock<OcrAccelerationStatus>,
}

impl LocalPaddleOcrEngine {
    pub(crate) fn new(root: Option<PathBuf>, cache_dir: PathBuf) -> Self {
        Self::with_factory(Arc::new(NativeSessionFactory {
            root,
            cache_dir,
            platform: Platform::current(),
        }))
    }

    fn with_factory(factory: Arc<dyn SessionFactory>) -> Self {
        Self {
            factory,
            state: Mutex::new(SessionState {
                gpu_enabled: None,
                backend: None,
                session: None,
            }),
            status: RwLock::new(OcrAccelerationStatus {
                gpu_enabled: true,
                device: None,
                fallback_reason: None,
            }),
        }
    }

    fn initialize(&self, state: &mut SessionState, backend: OcrBackend) -> Result<()> {
        state.session = Some(self.factory.create(backend)?);
        state.backend = Some(backend);
        self.status.write().device = Some(if backend.is_gpu() {
            OcrExecutionDevice::Gpu
        } else {
            OcrExecutionDevice::Cpu
        });
        log::info!("PaddleOCR medium initialized: {backend:?}");
        Ok(())
    }

    fn fallback(&self, state: &mut SessionState, error: &AppError) -> Result<()> {
        log::warn!("PaddleOCR GPU failed, switching to CPU: {error}");
        state.session = None;
        state.backend = None;
        self.status.write().device = None;
        self.initialize(state, self.factory.cpu_backend())?;
        self.status.write().fallback_reason = Some("GPU 加速不可用，已使用 CPU".into());
        Ok(())
    }
}

impl PaddleOcrEngine for LocalPaddleOcrEngine {
    fn is_available(&self) -> bool {
        self.factory.is_available()
    }

    fn status(&self) -> OcrAccelerationStatus {
        self.status.read().clone()
    }

    fn recognize(&self, request: &OcrRequest, gpu_enabled: bool) -> Result<OcrResult> {
        // Decode before initializing GPU resources; invalid input must not disable acceleration.
        let image = image::load_from_memory(&request.image_data)
            .map_err(|error| AppError::Other(format!("Invalid OCR image: {error}")))?
            .to_rgb8();
        if image.width() == 0 || image.height() == 0 {
            return Err(AppError::Other("Empty OCR image".into()));
        }
        let mut state = self.state.lock();
        if state.gpu_enabled != Some(gpu_enabled) {
            state.session = None;
            state.backend = None;
            state.gpu_enabled = Some(gpu_enabled);
            *self.status.write() = OcrAccelerationStatus {
                gpu_enabled,
                device: None,
                fallback_reason: None,
            };
        }
        if state.session.is_none() {
            let gpu = gpu_enabled.then(|| self.factory.gpu_backend()).flatten();
            let backend = gpu.unwrap_or_else(|| self.factory.cpu_backend());
            if gpu_enabled && gpu.is_none() {
                self.status.write().fallback_reason = Some("当前平台使用 CPU 识别".into());
            }
            if let Err(error) = self.initialize(&mut state, backend) {
                if !backend.is_gpu() {
                    return Err(error);
                }
                self.fallback(&mut state, &error)?;
            }
        }
        let result = state.session.as_mut().unwrap().recognize(&image);
        match result {
            Err(error) if state.backend.is_some_and(OcrBackend::is_gpu) => {
                self.fallback(&mut state, &error)?;
                state.session.as_mut().unwrap().recognize(&image)
            }
            result => result,
        }
    }
}

#[derive(Clone, Copy)]
#[allow(dead_code)] // Keep every platform strategy available to platform-selection tests.
enum Platform {
    MacArm(u32),
    MacIntel,
    Windows,
    Other,
}

impl Platform {
    fn current() -> Self {
        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        {
            let version = objc2_foundation::NSProcessInfo::processInfo().operatingSystemVersion();
            return Self::MacArm(version.majorVersion as u32);
        }
        #[cfg(all(target_os = "macos", not(target_arch = "aarch64")))]
        {
            return Self::MacIntel;
        }
        #[cfg(target_os = "windows")]
        {
            return Self::Windows;
        }
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        {
            Self::Other
        }
    }

    fn cpu(self) -> OcrBackend {
        match self {
            Self::MacArm(0..=13) | Self::MacIntel => OcrBackend::MnnCpu,
            _ => OcrBackend::OrtCpu,
        }
    }

    fn gpu(self) -> Option<OcrBackend> {
        match self {
            Self::MacArm(14..) => Some(OcrBackend::CoreMl),
            Self::Windows => Some(OcrBackend::DirectMl),
            _ => None,
        }
    }
}

struct NativeSessionFactory {
    root: Option<PathBuf>,
    cache_dir: PathBuf,
    platform: Platform,
}

impl SessionFactory for NativeSessionFactory {
    fn is_available(&self) -> bool {
        let Some(root) = &self.root else {
            return false;
        };
        let format = if self.cpu_backend() == OcrBackend::MnnCpu {
            "mnn"
        } else {
            "onnx"
        };
        [
            format!("medium_det.{format}"),
            format!("medium_rec.{format}"),
            "keys.txt".into(),
        ]
        .iter()
        .all(|file| root.join(file).is_file())
            && (format == "mnn"
                || root
                    .join("runtime")
                    .join(ort_engine::LIBRARY_NAME)
                    .is_file())
    }
    fn cpu_backend(&self) -> OcrBackend {
        self.platform.cpu()
    }
    fn gpu_backend(&self) -> Option<OcrBackend> {
        self.platform.gpu()
    }
    fn create(&self, backend: OcrBackend) -> Result<Box<dyn MediumSession>> {
        let root = self
            .root
            .as_deref()
            .ok_or_else(|| AppError::Other("Packaged OCR resources are unavailable".into()))?;
        if backend == OcrBackend::MnnCpu {
            #[cfg(target_os = "macos")]
            {
                return Ok(Box::new(mnn_engine::MnnMediumSession::new(root)?));
            }
            #[cfg(not(target_os = "macos"))]
            {
                return Err(AppError::Other("MNN CPU is only bundled on macOS".into()));
            }
        }
        Ok(Box::new(ort_engine::OrtMediumSession::new(
            root,
            &self.cache_dir,
            backend,
        )?))
    }
}

fn normalized_box(points: &[(f32, f32)], width: u32, height: u32) -> Option<OcrBoundingBox> {
    if points.is_empty() || points.iter().any(|(x, y)| !x.is_finite() || !y.is_finite()) {
        return None;
    }
    let x0 = points
        .iter()
        .map(|p| p.0)
        .fold(f32::INFINITY, f32::min)
        .clamp(0.0, width as f32);
    let x1 = points
        .iter()
        .map(|p| p.0)
        .fold(f32::NEG_INFINITY, f32::max)
        .clamp(0.0, width as f32);
    let y0 = points
        .iter()
        .map(|p| p.1)
        .fold(f32::INFINITY, f32::min)
        .clamp(0.0, height as f32);
    let y1 = points
        .iter()
        .map(|p| p.1)
        .fold(f32::NEG_INFINITY, f32::max)
        .clamp(0.0, height as f32);
    Some(OcrBoundingBox {
        x: x0 / width as f32,
        y: 1.0 - y1 / height as f32,
        width: (x1 - x0) / width as f32,
        height: (y1 - y0) / height as f32,
    })
}

fn from_lines(lines: Vec<OcrLine>) -> OcrResult {
    let scores: Vec<_> = lines.iter().filter_map(|line| line.confidence).collect();
    OcrResult {
        text: lines
            .iter()
            .map(|line| line.text.as_str())
            .collect::<Vec<_>>()
            .join("\n"),
        confidence: (!scores.is_empty()).then(|| scores.iter().sum::<f32>() / scores.len() as f32),
        lines,
        detected_language: None,
        provider_id: None,
    }
}
