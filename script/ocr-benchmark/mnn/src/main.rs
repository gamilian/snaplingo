use image::GenericImageView;
use ocr_rs::{Backend, OcrEngine, OcrEngineConfig, PrecisionMode, RecOptions};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{env, error::Error, fs::File, io::{BufRead, BufReader, BufWriter, Read, Write}, path::{Path, PathBuf}, time::Instant};

const REVISION: &str = "664a945953e46186b37203fa4902a20d60a32809";
const MODEL_BASE: &str = "/tmp/snaplingo-ocr-benchmark/mnn/models";

struct Options {
    model: String,
    backend: Backend,
    threads: i32,
    batch: usize,
    parallel: bool,
    warmups: usize,
    repeats: usize,
    min_confidence: f32,
    precision: PrecisionMode,
    manifest: Option<PathBuf>,
    output: PathBuf,
    images: Vec<PathBuf>,
    check_backend: bool,
}

fn parse() -> Result<Options, Box<dyn Error>> {
    let mut options = Options {
        model: "v6_tiny".into(), backend: Backend::CPU, threads: 4, batch: 8, parallel: true,
        warmups: 1, repeats: 3, min_confidence: 0.5, precision: PrecisionMode::Normal,
        manifest: None, output: "mnn-results.jsonl".into(), images: vec![], check_backend: false,
    };
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--check-backend" { options.check_backend = true; continue; }
        if arg == "--help" {
            println!("snaplingo-mnn-ocr-bench [--model v6_tiny|v6_small|v6_medium|v5_fp16|v5_fp32] [--backend cpu|metal] [--threads 4] [--batch 8] [--parallel true|false] [--warmups 1] [--repeats 3] [--precision normal|high|low] [--min-confidence 0.5] [--manifest images.jsonl] [--output results.jsonl] [--check-backend] [IMAGE ...]");
            std::process::exit(0);
        }
        if !arg.starts_with("--") { options.images.push(arg.into()); continue; }
        let value = args.next().ok_or_else(|| format!("missing value for {arg}"))?;
        match arg.as_str() {
            "--model" => options.model = value,
            "--backend" => options.backend = match value.as_str() {
                "cpu" => Backend::CPU, "metal" => Backend::Metal,
                _ => return Err(format!("unsupported backend: {value}").into()),
            },
            "--threads" => options.threads = value.parse()?,
            "--batch" => options.batch = value.parse()?,
            "--parallel" => options.parallel = value.parse()?,
            "--warmups" => options.warmups = value.parse()?,
            "--repeats" => options.repeats = value.parse()?,
            "--min-confidence" => options.min_confidence = value.parse()?,
            "--precision" => options.precision = match value.as_str() {
                "normal" => PrecisionMode::Normal, "high" => PrecisionMode::High, "low" => PrecisionMode::Low,
                _ => return Err(format!("unsupported precision: {value}").into()),
            },
            "--manifest" => options.manifest = Some(value.into()),
            "--output" => options.output = value.into(),
            _ => return Err(format!("unknown option: {arg}").into()),
        }
    }
    if !(1..=32).contains(&options.threads) || options.repeats == 0 || options.batch == 0
        || !options.min_confidence.is_finite() || !(0.0..=1.0).contains(&options.min_confidence) {
        return Err("threads must be 1..32, repeats/batch positive, and min-confidence 0..1".into());
    }
    Ok(options)
}

fn model_paths(model: &str) -> Result<[PathBuf; 3], Box<dyn Error>> {
    let names = match model {
        "v6_tiny" => ["PP-OCRv6_tiny_det.mnn", "PP-OCRv6_tiny_rec.mnn", "ppocr_keys_v6_tiny.txt"],
        "v6_small" => ["PP-OCRv6_small_det.mnn", "PP-OCRv6_small_rec.mnn", "ppocr_keys_v6_small.txt"],
        "v6_medium" => ["PP-OCRv6_medium_det.mnn", "PP-OCRv6_medium_rec.mnn", "ppocr_keys_v6_medium.txt"],
        "v5_fp16" => ["PP-OCRv5_mobile_det_fp16.mnn", "PP-OCRv5_mobile_rec_fp16.mnn", "ppocr_keys_v5.txt"],
        "v5_fp32" => ["PP-OCRv5_mobile_det.mnn", "PP-OCRv5_mobile_rec.mnn", "ppocr_keys_v5.txt"],
        _ => return Err(format!("unsupported model: {model}").into()),
    };
    let base = env::var("MNN_BENCH_MODEL_DIR").unwrap_or_else(|_| MODEL_BASE.into());
    Ok(names.map(|name| Path::new(&base).join(name)))
}

fn file_metadata(path: &Path) -> Result<Value, Box<dyn Error>> {
    let mut file = File::open(path)?;
    let bytes = file.metadata()?.len();
    let mut sha = Sha256::new();
    let mut buffer = [0u8; 65536];
    loop { let count = file.read(&mut buffer)?; if count == 0 { break; } sha.update(&buffer[..count]); }
    Ok(json!({"path": path, "bytes": bytes, "sha256": format!("{:x}", sha.finalize()),
        "source_url": format!("https://raw.githubusercontent.com/zibo-chen/ocr-rs/{REVISION}/models/{}", path.file_name().unwrap().to_string_lossy())}))
}

fn emit(output: &mut BufWriter<File>, event: Value) -> Result<(), Box<dyn Error>> {
    serde_json::to_writer(&mut *output, &event)?;
    output.write_all(b"\n")?;
    output.flush()?;
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let options = parse()?;
    let mut output = BufWriter::new(File::create(&options.output)?);
    let runtime_available = options.backend.is_available();
    let native = match options.backend { Backend::CPU => mnn_runtime::sys::Backend::Cpu, Backend::Metal => mnn_runtime::sys::Backend::Metal, _ => unreachable!() };
    let native_registered = mnn_runtime::sys::backend_available(native);
    emit(&mut output, json!({"type": "backend", "backend": options.backend.as_str(),
        "rust_feature_available": runtime_available, "native_registered": native_registered,
        "metal_feature": cfg!(feature = "metal"), "mnn_version": ocr_rs::mnn_version(),
        "ocr_rs_version": ocr_rs::version(), "ocr_rs_revision": REVISION,
        "mnn_runtime_version": "0.1.0", "native_source_revision": "d407447ed56c4121a11ccbd266dc184ca1ead0c2",
        "native_prebuilt_tag": "mnn-3.6.1-d407447e-r1", "strict_session_backend_guard": true,
        "pid": std::process::id(), "os": env::consts::OS, "arch": env::consts::ARCH}))?;
    if !runtime_available || !native_registered { return Err(format!("requested backend {} is unavailable; CPU fallback is forbidden", options.backend.as_str()).into()); }
    if options.check_backend { return Ok(()); }
    let mut inputs: Vec<(String, PathBuf)> = vec![];
    if let Some(manifest) = &options.manifest {
        let parent = manifest.parent().unwrap_or(Path::new("."));
        for line in BufReader::new(File::open(manifest)?).lines() {
            let line = line?;
            if line.trim().is_empty() { continue; }
            let item: Value = serde_json::from_str(&line)?;
            let image = item.get("image").or_else(|| item.get("path")).and_then(Value::as_str).ok_or("manifest rows need image/path")?;
            let path = PathBuf::from(image);
            let path = if path.is_absolute() { path } else { parent.join(path) };
            let id = item.get("id").and_then(Value::as_str).unwrap_or(image).to_owned();
            inputs.push((id, path));
        }
    }
    inputs.extend(options.images.iter().map(|path| (path.to_string_lossy().into_owned(), path.clone())));
    if inputs.is_empty() { return Err("provide --manifest or IMAGE arguments".into()); }
    let paths = model_paths(&options.model)?;
    let weights = paths.iter().map(|path| file_metadata(path)).collect::<Result<Vec<_>, _>>()?;
    let config = OcrEngineConfig::new().with_backend(options.backend).with_threads(options.threads)
        .with_precision(options.precision).with_min_result_confidence(options.min_confidence)
        .with_rec_options(RecOptions::default().with_batch_size(options.batch))
        .with_parallel(options.parallel);
    let config_description = format!("{config:?}");
    let start = Instant::now();
    let engine = OcrEngine::new(&paths[0], &paths[1], &paths[2], Some(config))?;
    emit(&mut output, json!({"type": "session", "model": options.model, "backend": options.backend.as_str(),
        "threads": options.threads, "batch": options.batch, "parallel": options.parallel,
        "warmups": options.warmups, "repeats": options.repeats,
        "load_ms": start.elapsed().as_secs_f64() * 1000.0, "config": config_description,
        "model_files": weights, "session_primary_backend_matches_request": true,
        "weight_storage_report": "weights-manifest.json; FP16 storage is separate from runtime precision",
        "timing_scope": "load includes model reads/session creation; ocr excludes image decode; total includes image decode and ocr"}))?;
    for pass in 0..(options.warmups + options.repeats) {
        for (id, path) in &inputs {
            let total = Instant::now();
            let decode = Instant::now();
            let image = image::open(path)?;
            let decode_ms = decode.elapsed().as_secs_f64() * 1000.0;
            let start = Instant::now();
            let recognized = engine.recognize(&image);
            let ocr_ms = start.elapsed().as_secs_f64() * 1000.0;
            let total_ms = total.elapsed().as_secs_f64() * 1000.0;
            let mut event = json!({"type": "image", "id": id, "image": path, "model": options.model,
                "backend": options.backend.as_str(), "threads": options.threads, "batch": options.batch,
                "parallel": options.parallel, "warmup": pass < options.warmups,
                "iteration": if pass < options.warmups { pass } else { pass - options.warmups },
                "width": image.dimensions().0, "height": image.dimensions().1,
                "decode_ms": decode_ms, "ocr_ms": ocr_ms, "total_ms": total_ms});
            match recognized {
                Ok(results) => {
                    event["text"] = json!(results.iter().map(|line| line.text.as_str()).collect::<Vec<_>>().join("\n"));
                    event["lines"] = json!(results.iter().map(|line| {
                        let rect = &line.bbox.rect;
                        let points = line.bbox.points.map(|points| points.map(|p| [p.x, p.y]));
                        json!({"text": line.text, "confidence": line.confidence, "detection_confidence": line.bbox.score,
                            "bbox": [rect.left(), rect.top(), rect.width() as i32, rect.height() as i32], "points": points})
                    }).collect::<Vec<_>>());
                }
                Err(error) => event["error"] = json!(error.to_string()),
            }
            emit(&mut output, event)?;
        }
    }
    emit(&mut output, json!({"type": "complete", "model": options.model, "images": inputs.len(),
        "batch": options.batch, "parallel": options.parallel,
        "warmups": options.warmups, "repeats": options.repeats}))?;
    Ok(())
}
