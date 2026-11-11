//! macOS-only research probe. RSS is sampled outside request timing; peak RSS is
//! reported separately. This does not initialize Tauri or use user data.
#[cfg(not(target_os = "macos"))]
compile_error!("This RSS/rusage harness is calibrated for macOS only");

use ocr_rs::{Backend, OcrEngine, OcrEngineConfig, RecOptions};
use serde_json::json;
use std::{
    env,
    io::{self, Write},
    path::Path,
    process::Command,
    thread,
    time::{Duration, Instant},
};
fn usage() -> libc::rusage {
    unsafe {
        let mut r = std::mem::zeroed();
        assert_eq!(libc::getrusage(libc::RUSAGE_SELF, &mut r), 0);
        r
    }
}
fn cpu_ms() -> f64 {
    let r = usage();
    (r.ru_utime.tv_sec + r.ru_stime.tv_sec) as f64 * 1000.
        + (r.ru_utime.tv_usec + r.ru_stime.tv_usec) as f64 / 1000.
}
fn rss() -> u64 {
    let output = Command::new("ps")
        .args(["-o", "rss=", "-p", &std::process::id().to_string()])
        .output()
        .unwrap();
    String::from_utf8(output.stdout)
        .unwrap()
        .trim()
        .parse::<u64>()
        .unwrap()
        * 1024
}
fn emit(v: serde_json::Value) {
    println!("{}", v);
    io::stdout().flush().unwrap();
}
fn idle(label: &str) {
    let mem = rss();
    let before = cpu_ms();
    let timer = Instant::now();
    thread::sleep(Duration::from_secs(2));
    let elapsed = timer.elapsed().as_secs_f64() * 1000.;
    let cpu = cpu_ms() - before;
    emit(
        json!({"type":"idle","label":label,"duration_ms":elapsed,"cpu_ms":cpu,"cpu_pct_one_core":cpu/elapsed*100.,"rss_before_bytes":mem,"rss_after_bytes":rss(),"peak_rss_bytes":usage().ru_maxrss}),
    );
}
fn load(root: &Path, threads: i32, parallel: bool) -> OcrEngine {
    OcrEngine::new(
        root.join("PP-OCRv6_tiny_det.mnn"),
        root.join("PP-OCRv6_tiny_rec.mnn"),
        root.join("ppocr_keys_v6_tiny.txt"),
        Some(
            OcrEngineConfig::new()
                .with_backend(Backend::CPU)
                .with_threads(threads)
                .with_parallel(parallel)
                .with_rec_options(RecOptions::default().with_batch_size(8)),
        ),
    )
    .unwrap()
}
fn main() {
    let args: Vec<_> = env::args().collect();
    assert!(
        (5..=7).contains(&args.len()),
        "Usage: tiny-lifecycle MODELS IMAGE cached|per-request|worker REPEATS [THREADS] [PARALLEL]"
    );
    let root = Path::new(&args[1]);
    let image_path = &args[2];
    let mode = &args[3];
    assert!(["cached", "per-request", "worker"].contains(&mode.as_str()));
    let repeats = args[4].parse::<usize>().unwrap();
    assert!(repeats > 0);
    let threads = args.get(5).map(|s| s.parse::<i32>().unwrap()).unwrap_or(4);
    let parallel = args
        .get(6)
        .map(|s| s.parse::<bool>().unwrap())
        .unwrap_or(true);
    let image = image::open(image_path).unwrap();
    emit(
        json!({"type":"start","mode":mode,"image":image_path,"width":image.width(),"height":image.height(),"pid":std::process::id(),"threads":threads,"parallel":parallel,"batch":8,"timing":"load includes model/session creation; recognize includes image pre/postprocessing; decoded input retained in baseline; cold means new process with filesystem cache unspecified"}),
    );
    if mode != "worker" {
        idle("baseline");
    }
    let mut cached = None;
    for iteration in 0..repeats {
        let before = rss();
        let total = Instant::now();
        let load_timer = Instant::now();
        if cached.is_none() {
            cached = Some(load(root, threads, parallel));
        }
        let load_ms = load_timer.elapsed().as_secs_f64() * 1000.;
        let recognize = Instant::now();
        let results = cached.as_ref().unwrap().recognize(&image).unwrap();
        let recognize_ms = recognize.elapsed().as_secs_f64() * 1000.;
        let total_ms = total.elapsed().as_secs_f64() * 1000.;
        assert!(!results.is_empty());
        let after = rss();
        emit(
            json!({"type":"request","iteration":iteration,"load_ms":load_ms,"recognize_ms":recognize_ms,"request_ms":total_ms,"rss_before_bytes":before,"rss_after_bytes":after,"peak_rss_bytes":usage().ru_maxrss,"lines":results.len(),"chars":results.iter().map(|r|r.text.chars().count()).sum::<usize>()}),
        );
        if mode == "per-request" || mode == "worker" {
            let drop_timer = Instant::now();
            drop(cached.take());
            let drop_ms = drop_timer.elapsed().as_secs_f64() * 1000.;
            emit(
                json!({"type":"drop","iteration":iteration,"drop_ms":drop_ms,"rss_bytes":rss(),"peak_rss_bytes":usage().ru_maxrss}),
            );
            if iteration == 0 && mode != "worker" {
                idle("after_first_drop");
            }
        }
    }
    if mode == "cached" {
        idle("cached_session");
        let drop_timer = Instant::now();
        drop(cached.take());
        let drop_ms = drop_timer.elapsed().as_secs_f64() * 1000.;
        emit(
            json!({"type":"drop","drop_ms":drop_ms,"rss_bytes":rss(),"peak_rss_bytes":usage().ru_maxrss}),
        );
    }
    if mode != "worker" {
        idle("after_final_drop");
    }
}
