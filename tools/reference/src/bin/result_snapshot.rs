//! Render the existing React Result views in WKWebView for native-page comparison.
//! Run Vite on 127.0.0.1:5194 first. No installed settings or Tauri runtime are used.
#[cfg(not(target_os = "macos"))]
fn main() {}

#[cfg(target_os = "macos")]
fn main() {
    native::run();
}

#[cfg(target_os = "macos")]
mod native {
    use block2::RcBlock;
    use objc2::{msg_send, rc::Retained, runtime::AnyObject, MainThreadMarker};
    use objc2_app_kit::{
        NSApplication, NSBackingStoreType, NSBitmapImageFileType, NSBitmapImageRep,
        NSColorRenderingIntent, NSColorSpace, NSEvent, NSEventModifierFlags, NSEventType, NSImage,
        NSWindow, NSWindowStyleMask,
    };
    use objc2_foundation::{
        NSDictionary, NSError, NSPoint, NSRect, NSSize, NSString, NSURLRequest, NSURL,
    };
    use objc2_web_kit::{WKSnapshotConfiguration, WKWebView, WKWebViewConfiguration};
    use std::cell::RefCell;
    use std::time::{Duration, Instant};

    struct Reference {
        window: Retained<NSWindow>,
        webview: Retained<WKWebView>,
    }
    thread_local! { static REFERENCE: RefCell<Option<Reference>> = const { RefCell::new(None) }; }

    async fn on_main<T: Send + 'static>(task: impl FnOnce() -> T + Send + 'static) -> T {
        let (sender, receiver) = tokio::sync::oneshot::channel();
        dispatch2::DispatchQueue::main().exec_async(move || {
            let _ = sender.send(std::panic::catch_unwind(std::panic::AssertUnwindSafe(task)));
        });
        match receiver.await.unwrap() {
            Ok(value) => value,
            Err(panic) => std::panic::resume_unwind(panic),
        }
    }
    async fn evaluate(script: impl Into<String>) -> String {
        let script = script.into();
        let (sender, receiver) = tokio::sync::oneshot::channel();
        on_main(move || {
            let sender = RefCell::new(Some(sender));
            let callback = RcBlock::new(move |value: *mut AnyObject, error: *mut NSError| {
                let text = if error.is_null() {
                    unsafe { value.as_ref() }
                        .and_then(|v| v.downcast_ref::<NSString>())
                        .map(|v| v.to_string())
                        .unwrap_or_default()
                } else {
                    String::new()
                };
                let _ = sender.borrow_mut().take().unwrap().send(text);
            });
            REFERENCE.with_borrow(|reference| unsafe {
                reference
                    .as_ref()
                    .unwrap()
                    .webview
                    .evaluateJavaScript_completionHandler(
                        &NSString::from_str(&script),
                        Some(&callback),
                    );
            });
        })
        .await;
        tokio::time::timeout(Duration::from_secs(5), receiver)
            .await
            .unwrap()
            .unwrap()
    }
    fn encode(image: &NSImage) -> Vec<u8> {
        let data = image.TIFFRepresentation().unwrap();
        let bitmap = NSBitmapImageRep::imageRepWithData(&data).unwrap();
        let bitmap = bitmap
            .bitmapImageRepByConvertingToColorSpace_renderingIntent(
                &NSColorSpace::sRGBColorSpace(),
                NSColorRenderingIntent::RelativeColorimetric,
            )
            .unwrap();
        let data = unsafe {
            bitmap.representationUsingType_properties(
                NSBitmapImageFileType::PNG,
                &NSDictionary::new(),
            )
        }
        .unwrap();
        unsafe { data.as_bytes_unchecked() }.to_vec()
    }
    async fn snapshot(clip: NSRect) -> Vec<u8> {
        let (sender, receiver) = tokio::sync::oneshot::channel();
        on_main(move || {
            let sender = RefCell::new(Some(sender));
            let callback = RcBlock::new(move |image: *mut NSImage, error: *mut NSError| {
                assert!(
                    error.is_null(),
                    "WKWebView could not snapshot the reference"
                );
                let _ = sender
                    .borrow_mut()
                    .take()
                    .unwrap()
                    .send(encode(unsafe { image.as_ref() }.unwrap()));
            });
            let configuration =
                unsafe { WKSnapshotConfiguration::new(MainThreadMarker::new().unwrap()) };
            unsafe {
                let _: () = msg_send![&configuration, setRect: clip];
            }
            REFERENCE.with_borrow(|reference| unsafe {
                reference
                    .as_ref()
                    .unwrap()
                    .webview
                    .takeSnapshotWithConfiguration_completionHandler(
                        Some(&configuration),
                        &callback,
                    );
            });
        })
        .await;
        tokio::time::timeout(Duration::from_secs(5), receiver)
            .await
            .unwrap()
            .unwrap()
    }

    async fn wait_for(script: impl Into<String>, description: &str) {
        let script = script.into();
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if evaluate(script.clone()).await == "ready" {
                return;
            }
            if Instant::now() >= deadline {
                eprintln!("Reference state: {}", evaluate("JSON.stringify({url:location.href,title:document.title,errors:window.referenceErrors,text:document.body.innerText.slice(0,2000)})").await);
                panic!("{description}");
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }

    pub(super) fn run() {
        let mtm = MainThreadMarker::new().unwrap();
        let app = NSApplication::sharedApplication(mtm);
        let frame = NSRect::new(NSPoint::ZERO, NSSize::new(660., 780.));
        let window = unsafe {
            NSWindow::initWithContentRect_styleMask_backing_defer(
                mtm.alloc(),
                frame,
                NSWindowStyleMask::Titled,
                NSBackingStoreType::Buffered,
                false,
            )
        };
        unsafe {
            window.setReleasedWhenClosed(false);
        }
        let configuration = unsafe { WKWebViewConfiguration::new(mtm) };
        let webview =
            unsafe { WKWebView::initWithFrame_configuration(mtm.alloc(), frame, &configuration) };
        window.setContentView(Some(&webview));
        REFERENCE.with_borrow_mut(|slot| *slot = Some(Reference { window, webview }));
        std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                runtime.block_on(async {
                let artifacts=std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/shared-result/reference");std::fs::create_dir_all(&artifacts).unwrap();
                for (theme,dark,mode,scale,scenario) in [("native",false,"translation",1.,"baseline"),("studio",true,"translation",1.,"baseline"),("paper",false,"translation",1.75,"baseline"),("native",false,"ocr",1.,"baseline"),("native",false,"ocr",1.,"image"),("studio",true,"ocr",1.,"image"),("paper",false,"ocr",1.75,"image")] {
                    on_main(move || REFERENCE.with_borrow(|slot|unsafe {
                        let reference=slot.as_ref().unwrap();reference.window.setContentSize(NSSize::new(660.*scale,780.*scale));
                        let url=NSURL::URLWithString(&NSString::from_str(&format!("http://127.0.0.1:5194/fixtures/result_reference.html?theme={theme}&appearance={}&mode={mode}&scale={scale}&scenario={scenario}",if dark{"dark"}else{"light"}))).unwrap();
                        reference.webview.loadRequest(&NSURLRequest::requestWithURL(&url));
                    })).await;
                    wait_for(format!("document.documentElement.dataset.referenceResult === '{mode}-{theme}-{}-{scale}-{scenario}' && document.querySelector('.result-panel textarea')?'ready':''",if dark{"dark"}else{"light"}),"Result reference did not load").await;
                    wait_for("[...document.images].every(image=>image.complete&&image.naturalWidth>0)?'ready':''","Result reference image did not load").await;
                    tokio::time::sleep(Duration::from_millis(250)).await;
                    let metrics=evaluate(r#"(() => {const frame=el=>{const r=el.getBoundingClientRect(),s=getComputedStyle(el);return{x:r.x,y:r.y,width:r.width,height:r.height,font:s.fontSize,line:s.lineHeight,padding:s.padding,color:s.color,background:s.backgroundColor,border:s.borderColor,radius:s.borderRadius};};return JSON.stringify({panel:frame(document.querySelector('.result-panel')),header:frame(document.querySelector('.result-panel>div')),textareas:[...document.querySelectorAll('textarea')].map(frame),cards:[...document.querySelectorAll('.translation-result-card')].map(frame),buttons:[...document.querySelectorAll('button')].map(el=>({label:el.getAttribute('aria-label')||el.title||el.innerText,frame:frame(el)})),images:[...document.images].map(image=>({frame:frame(image),parent:frame(image.parentElement),width:image.naturalWidth,height:image.naturalHeight})),tokens:[...document.querySelectorAll('button')].filter(el=>(el.getAttribute('aria-label')||'').startsWith('复制')&&(el.getAttribute('aria-label')||'').includes(': ')).map(el=>({label:el.getAttribute('aria-label'),frame:frame(el)})),headings:[...document.querySelectorAll('h3')].map(el=>({text:el.textContent,frame:frame(el)})),blocks:[...document.querySelectorAll('.translation-result-copy')].map(frame),body:document.body.innerText,errors:window.referenceErrors});})()"#).await;
                    let name=format!("{mode}-{theme}{}-{scale}{}",if dark{"-dark"}else{""},if scenario=="image"{"-image"}else{""});std::fs::write(artifacts.join(format!("{name}.json")),metrics).unwrap();
                    std::fs::write(artifacts.join(format!("{name}.png")),snapshot(NSRect::new(NSPoint::ZERO,NSSize::new(660.*scale,780.*scale))).await).unwrap();
                    if scenario == "image" {
                        let copied=evaluate(r#"(()=>{const buttons=[...document.querySelectorAll('button')].filter(el=>(el.getAttribute('aria-label')||'').includes(': '));for(const button of buttons)button.click();return JSON.stringify(window.resultReference.copied);})()"#).await;
                        std::fs::write(artifacts.join(format!("{name}-copied.json")),copied).unwrap();
                        on_main(move || REFERENCE.with_borrow(|slot| slot.as_ref().unwrap().window.setContentSize(NSSize::new(660.*scale,500.*scale)))).await;
                        tokio::time::sleep(Duration::from_millis(100)).await;
                        let scroll=evaluate(r#"(()=>{const area=document.querySelector('.result-window-scrollbar');area.scrollTop=area.scrollHeight;area.dispatchEvent(new Event('scroll'));return 'ok';})()"#).await;let _=scroll;
                        tokio::time::sleep(Duration::from_millis(100)).await;
                        let metrics=evaluate(r#"(()=>{const frame=el=>{const r=el.getBoundingClientRect();return{x:r.x,y:r.y,width:r.width,height:r.height};};const area=document.querySelector('.result-window-scrollbar');return JSON.stringify({panel:frame(document.querySelector('.result-panel')),scroll:{top:area.scrollTop,height:area.scrollHeight,client:area.clientHeight},textareas:[...document.querySelectorAll('textarea')].map(frame),buttons:[...document.querySelectorAll('button')].map(el=>({label:el.getAttribute('aria-label')||el.title||el.innerText,frame:frame(el)})),images:[...document.images].map(image=>({frame:frame(image),parent:frame(image.parentElement),width:image.naturalWidth,height:image.naturalHeight}))});})()"#).await;
                        std::fs::write(artifacts.join(format!("{name}-scrolled.json")),metrics).unwrap();
                        std::fs::write(artifacts.join(format!("{name}-scrolled.png")),snapshot(NSRect::new(NSPoint::ZERO,NSSize::new(660.*scale,500.*scale))).await).unwrap();

                    }

                }
                on_main(||{REFERENCE.with_borrow_mut(|slot|slot.take().unwrap().window.close());let app=NSApplication::sharedApplication(MainThreadMarker::new().unwrap());app.stop(None);let event=NSEvent::otherEventWithType_location_modifierFlags_timestamp_windowNumber_context_subtype_data1_data2(NSEventType::ApplicationDefined,NSPoint::ZERO,NSEventModifierFlags::empty(),0.,0,None,0,0,0).unwrap();app.postEvent_atStart(&event,true);}).await;
            })
            }));
            if result.is_err() {
                std::process::exit(1);
            }
        });
        app.run();
        println!("Captured existing React ResultWindow references");
    }
}
