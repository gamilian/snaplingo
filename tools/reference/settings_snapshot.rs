//! Render the existing React Settings views in WKWebView for native-page comparison.
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

    async fn feature_metrics() -> serde_json::Value {
        serde_json::from_str(&evaluate(r#"(() => {
            const frame=el=>{const r=el.getBoundingClientRect(),s=getComputedStyle(el);return {x:r.x,y:r.y,width:r.width,height:r.height,font:s.fontSize,weight:s.fontWeight,line:s.lineHeight,color:s.color};};
            const row=el=>({text:el.innerText,frame:frame(el),controls:[...el.querySelectorAll('button,input')].map(c=>({tag:c.tagName,text:c.innerText,title:c.title,value:c.value,disabled:c.disabled,frame:frame(c)}))});
            return JSON.stringify({page:document.documentElement.dataset.referencePage,theme:document.documentElement.dataset.interfaceTheme,
                content:frame(document.querySelector('.settings-content')),header:frame(document.querySelector('.settings-page-header')),
                context:frame(document.querySelector('.settings-context-nav')),tabs:[...document.querySelectorAll('.settings-section-tabs .settings-section-link')].map(row),
                cards:[...document.querySelectorAll('.settings-card')].map(el=>({title:el.querySelector('h2').textContent,frame:frame(el),heading:frame(el.querySelector('.settings-card-heading')),rows:[...el.querySelectorAll('.settings-control-row,.settings-hotkey-row')].map(row)})),
                themes:[...document.querySelectorAll('.theme-option')].map(frame),
                state:window.settingsFeatureReference?.snapshot()});
        })()"#).await).expect("Feature reference geometry is valid")
    }

    async fn library_metrics() -> String {
        evaluate(r#"(() => {
            const frame=el=>{const r=el.getBoundingClientRect(),s=getComputedStyle(el);return {x:r.x,y:r.y,width:r.width,height:r.height,font:s.fontSize,line:s.lineHeight};};
            const control=el=>({tag:el.tagName,label:el.getAttribute('aria-label')||el.title||el.innerText,value:el.value,disabled:el.disabled,frame:frame(el)});
            return JSON.stringify({content:frame(document.querySelector('.settings-content')),layout:frame(document.querySelector('.library-layout')),
                header:frame(document.querySelector('.library-heading')),list:frame(document.querySelector('.library-layout>section:first-child>div')),footer:frame(document.querySelector('.library-layout footer')),detail:frame(document.querySelector('.library-detail')),
                rows:[...document.querySelectorAll('.library-list-item')].map(control),cards:[...document.querySelectorAll('.library-detail-card')].map(frame),
                controls:[...document.querySelectorAll('.library-layout input,.library-layout textarea,.library-layout button:not(.library-list-item)')].map(control),state:window.libraryReference.snapshot()});
        })()"#).await
    }

    async fn capture_library_references(
        artifacts: &std::path::Path,
        compact: bool,
        appearance: &'static str,
    ) {
        for page in ["history", "favorites"] {
            for theme in ["native", "studio", "paper"] {
                on_main(move || REFERENCE.with_borrow(|reference| unsafe {
                let url=NSURL::URLWithString(&NSString::from_str(&format!("http://127.0.0.1:5194/reference/web/fixtures/settings_reference.html?theme={theme}&appearance={appearance}&page={page}"))).unwrap();
                reference.as_ref().unwrap().webview.loadRequest(&NSURLRequest::requestWithURL(&url));
            })).await;
                wait_for(format!("document.documentElement.dataset.referencePage==='{page}' && document.querySelectorAll('.library-list-item').length===20 && window.libraryReference ? 'ready' : ''"),"Library reference did not load").await;
                let metrics: serde_json::Value =
                    serde_json::from_str(&library_metrics().await).unwrap();
                let c = &metrics["content"];
                let clip = NSRect::new(
                    NSPoint::new(c["x"].as_f64().unwrap(), c["y"].as_f64().unwrap()),
                    NSSize::new(c["width"].as_f64().unwrap(), c["height"].as_f64().unwrap()),
                );
                for (index, selection) in [(0, "translation"), (1, "ocr"), (2, "screenshot")] {
                    if page == "history" && index == 2 {
                        continue;
                    }
                    evaluate(format!("window.libraryReference.select({index}); 'ok'")).await;
                    wait_for("[...document.querySelectorAll('.library-detail img')].every(image=>image.complete&&image.naturalWidth>0)?'ready':''","Library image did not load").await;
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    let name = format!(
                        "{theme}{}{}-{page}-{selection}",
                        if appearance == "dark" { "-dark" } else { "" },
                        if compact { "-compact" } else { "" }
                    );
                    std::fs::write(
                        artifacts.join(format!("reference-library-{name}.png")),
                        snapshot(clip).await,
                    )
                    .unwrap();
                    std::fs::write(
                        artifacts.join(format!("reference-library-{name}.json")),
                        library_metrics().await,
                    )
                    .unwrap();
                }
                evaluate("document.querySelector('.library-layout button[aria-label=\"下一页\"]').click(); 'ok'").await;
                wait_for(
                    "window.libraryReference.snapshot().queries.at(-1)?.page===1?'ready':''",
                    "Library pagination did not query page one",
                )
                .await;
                evaluate("[...document.querySelectorAll('.library-heading button')].find(button=>button.textContent==='OCR').click(); 'ok'").await;
                wait_for("window.libraryReference.snapshot().queries.at(-1)?.filter==='ocr' && document.querySelectorAll('.library-list-item').length===1?'ready':''","Library filter did not reset the page").await;
                evaluate(r#"(() => {const input=document.querySelector('.library-heading input');Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,'value').set.call(input,'unmatched');input.dispatchEvent(new Event('input',{bubbles:true}));return 'ok';})()"#).await;
                wait_for(
                    "document.querySelectorAll('.library-list-item').length===0?'ready':''",
                    "Library search did not update",
                )
                .await;
                let name = format!(
                    "{theme}{}{}-{page}-search-empty",
                    if appearance == "dark" { "-dark" } else { "" },
                    if compact { "-compact" } else { "" }
                );
                std::fs::write(
                    artifacts.join(format!("reference-library-{name}.json")),
                    library_metrics().await,
                )
                .unwrap();
            }
        }
    }

    async fn services_metrics() -> serde_json::Value {
        serde_json::from_str(&evaluate(r#"(() => {
            const frame=el=>{if(!el)return null;const r=el.getBoundingClientRect(),s=getComputedStyle(el);return {x:r.x,y:r.y,width:r.width,height:r.height,font:s.fontSize,weight:s.fontWeight,line:s.lineHeight,color:s.color,background:s.backgroundColor,padding:s.padding,gap:s.gap};};
            const control=el=>({tag:el.tagName,label:el.getAttribute('aria-label')||el.title||el.innerText,placeholder:el.placeholder,value:el.type==='password'?'':el.value,disabled:el.disabled,frame:frame(el)});
            return JSON.stringify({content:frame(document.querySelector('.settings-content')),header:frame(document.querySelector('.settings-page-header')),heading:frame(document.querySelector('.settings-page-header h1')),description:frame(document.querySelector('.settings-page-header p')),context:frame(document.querySelector('.settings-context-nav')),main:frame(document.querySelector('main.settings-page')),
              tabs:[...document.querySelectorAll('.settings-section-link')].map(control),
              providers:[...document.querySelectorAll('.provider-card')].map(el=>({name:el.querySelector('h3').textContent,frame:frame(el),title:frame(el.querySelector('h3')),subtitle:frame(el.querySelector('p')),icon:frame(el.querySelector('.h-9.w-9')),badges:[...el.querySelectorAll('.rounded-full')].map(el=>({text:el.textContent,frame:frame(el)})),controls:[...el.querySelectorAll('button')].map(control)})),
              groups:[...document.querySelectorAll('.settings-control-row,.settings-card')].map(el=>({text:el.innerText,frame:frame(el)})),
              labels:[...document.querySelectorAll('.settings-scroll-area label,.settings-scroll-area h3')].map(el=>({text:el.textContent,frame:frame(el)})),
              controls:[...document.querySelectorAll('.settings-scroll-layout button,.settings-scroll-layout input,.settings-scroll-layout textarea')].map(control),state:window.servicesReference.snapshot()});
        })()"#).await).expect("Services reference geometry is valid")
    }

    async fn capture_services_references(
        artifacts: &std::path::Path,
        compact: bool,
        appearance: &'static str,
    ) {
        for theme in ["native", "studio", "paper"] {
            for (scenario, route, provider) in [
                ("ocr", "ocr", ""),
                ("translation", "translation", ""),
                ("tts", "tts", ""),
                ("ocr-credentials", "ocr", "百度 OCR"),
                ("translation-credentials", "translation", "百度翻译"),
                ("deeplx", "translation", "DeepLX"),
                ("deepl", "translation", "DeepLX"),
                ("custom", "translation", "我的 GPT"),
            ] {
                on_main(move || REFERENCE.with_borrow(|reference| unsafe {
                    let url=NSURL::URLWithString(&NSString::from_str(&format!("http://127.0.0.1:5194/reference/web/fixtures/settings_reference.html?theme={theme}&appearance={appearance}&page=services&route={route}"))).unwrap();
                    reference.as_ref().unwrap().webview.loadRequest(&NSURLRequest::requestWithURL(&url));
                })).await;
                wait_for(format!("document.documentElement.dataset.referencePage==='services' && window.servicesReference?.snapshot().route==='{route}' && document.querySelectorAll('.settings-page-header').length===1 && (document.querySelectorAll('.provider-card').length>0 || document.querySelectorAll('.settings-control-row').length===3)?'ready':''"),"Services reference did not load").await;
                if !provider.is_empty() {
                    evaluate(format!("[...document.querySelectorAll('.provider-card')].find(el=>el.querySelector('h3').textContent==='{provider}').querySelector('button[aria-label=\"编辑\"]').click(); 'ok'")).await;
                    wait_for("document.querySelectorAll('.settings-scroll-area input').length>0?'ready':''","Services form did not load").await;
                }
                if scenario == "deepl" {
                    evaluate("document.querySelector('button[role=switch]').click(); 'ok'").await;
                }
                tokio::time::sleep(Duration::from_millis(120)).await;
                if scenario == "custom" {
                    evaluate("[...document.querySelectorAll('button')].find(button=>button.textContent==='保存策略').click(); 'ok'").await;
                    wait_for(
                        "window.servicesReference.snapshot().promptSaves===1?'ready':''",
                        "The existing React prompt save did not execute",
                    )
                    .await;
                    tokio::time::sleep(Duration::from_millis(50)).await;
                }
                let metrics = services_metrics().await;
                let c = &metrics["content"];
                let clip = NSRect::new(
                    NSPoint::new(c["x"].as_f64().unwrap(), c["y"].as_f64().unwrap()),
                    NSSize::new(c["width"].as_f64().unwrap(), c["height"].as_f64().unwrap()),
                );
                let name = format!(
                    "{theme}{}{}-{scenario}",
                    if appearance == "dark" { "-dark" } else { "" },
                    if compact { "-compact" } else { "" }
                );
                std::fs::write(
                    artifacts.join(format!("reference-services-{name}.json")),
                    serde_json::to_string_pretty(&metrics).unwrap(),
                )
                .unwrap();
                std::fs::write(
                    artifacts.join(format!("reference-services-{name}.png")),
                    snapshot(clip).await,
                )
                .unwrap();
                if scenario == "custom" {
                    evaluate("document.querySelector('.settings-scroll-area').scrollTop=650; 'ok'")
                        .await;
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    std::fs::write(
                        artifacts.join(format!("reference-services-{name}-prompt.json")),
                        serde_json::to_string_pretty(&services_metrics().await).unwrap(),
                    )
                    .unwrap();
                    std::fs::write(
                        artifacts.join(format!("reference-services-{name}-prompt.png")),
                        snapshot(clip).await,
                    )
                    .unwrap();
                }
            }
        }
    }

    async fn capture_feature_references(
        artifacts: &std::path::Path,
        compact: bool,
        appearance: &'static str,
        screenshot_only: bool,
    ) {
        let pages: &[&str] = if screenshot_only {
            &["screenshot"]
        } else {
            &["ocr", "translation"]
        };
        for &page in pages {
            for theme in ["native", "studio", "paper"] {
                let theme_key = format!(
                    "{theme}{}{}",
                    if appearance == "dark" { "-dark" } else { "" },
                    if compact { "-compact" } else { "" }
                );
                on_main(move || REFERENCE.with_borrow(|reference| unsafe {
                    let url = NSURL::URLWithString(&NSString::from_str(&format!("http://127.0.0.1:5194/reference/web/fixtures/settings_reference.html?theme={theme}&appearance={appearance}&page={page}&reference=features"))).unwrap();
                    reference.as_ref().unwrap().webview.loadRequest(&NSURLRequest::requestWithURL(&url));
                })).await;
                wait_for(format!("document.documentElement.dataset.referencePage==='{page}' && document.documentElement.dataset.interfaceTheme==='{theme}' && document.documentElement.dataset.theme==='{appearance}' && document.querySelectorAll('.settings-hotkey-row').length==={} && window.settingsFeatureReference ? 'ready' : ''", if screenshot_only { 5 } else { 3 }), "Feature reference did not render").await;
                tokio::time::sleep(Duration::from_millis(150)).await;
                let metrics = feature_metrics().await;
                let c = &metrics["content"];
                let expected = match theme {
                    "native" if compact => (65.0, 0.0, 635.0, 650.0),
                    "native" => (202.0, 0.0, 698.0, 650.0),
                    "studio" => (68.0, 0.0, if compact { 632.0 } else { 832.0 }, 650.0),
                    _ => (0.0, 68.0, if compact { 700.0 } else { 900.0 }, 582.0),
                };
                assert_eq!(
                    (
                        c["x"].as_f64().unwrap(),
                        c["y"].as_f64().unwrap(),
                        c["width"].as_f64().unwrap(),
                        c["height"].as_f64().unwrap()
                    ),
                    expected
                );
                let clip = NSRect::new(
                    NSPoint::new(expected.0, expected.1),
                    NSSize::new(expected.2, expected.3),
                );
                let sections: &[&str] = if page == "ocr" {
                    &["hotkeys", "recognition", "window"]
                } else if page == "screenshot" {
                    &["hotkeys", "output", "capture-interface"]
                } else {
                    &["hotkeys", "language", "window", "behavior"]
                };
                for (index, section) in sections.iter().enumerate() {
                    evaluate(format!(r#"(() => {{ const area=document.querySelector('.settings-scroll-area'),card=document.querySelectorAll('.settings-card')[{index}];
                        area.style.scrollBehavior='auto';area.scrollTop=card.getBoundingClientRect().top-area.getBoundingClientRect().top+area.scrollTop-document.querySelector('.settings-page-header').offsetHeight-16;
                        area.dispatchEvent(new Event('scroll'));return 'ok'; }})()"#)).await;
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    std::fs::write(
                        artifacts.join(format!("reference-{theme_key}-{page}-{section}.png")),
                        snapshot(clip).await,
                    )
                    .unwrap();
                    std::fs::write(
                        artifacts.join(format!("reference-{theme_key}-{page}-{section}.json")),
                        serde_json::to_vec_pretty(&feature_metrics().await).unwrap(),
                    )
                    .unwrap();
                }
                if screenshot_only {
                    let steps = evaluate(r#"(() => {
                        const input=document.querySelector('input[aria-label="长截图最大高度"]');
                        const original=input.value;
                        const cases=['30000','100','100000','100.5','100001','99.9',''];
                        const results=cases.map(value=>{input.value=value;input.stepUp();const up=input.value;input.value=value;input.stepDown();return {value,up,down:input.value};});
                        input.value=original;return JSON.stringify(results);
                    })()"#).await;
                    std::fs::write(
                        artifacts.join(format!(
                            "reference-{theme_key}-screenshot-number-steps.json"
                        )),
                        steps,
                    )
                    .unwrap();
                    continue;
                }
                evaluate("document.querySelector('.settings-scroll-area').scrollTop=0; document.querySelector('.settings-hotkey-row button').click(); 'ok'").await;
                wait_for("window.settingsFeatureReference.snapshot().recordingId && document.querySelector('[data-hotkey-recorder]') ? 'ready' : ''", "Feature recording did not start").await;
                std::fs::write(
                    artifacts.join(format!("reference-{theme_key}-{page}-recording.png")),
                    snapshot(clip).await,
                )
                .unwrap();
                evaluate("window.settingsFeatureReference.emit('⇧⌘R'); 'ok'").await;
                wait_for(format!("window.settingsFeatureReference.snapshot().recordingId===null && window.settingsFeatureReference.snapshot().hotkeys.{page}['{}']==='⇧⌘R' ? 'ready' : ''", if page=="ocr" { "screenshot-ocr" } else { "selection-translate" }), "Feature recording did not save through its store").await;
                std::fs::write(
                    artifacts.join(format!("reference-{theme_key}-{page}-hotkey-modified.png")),
                    snapshot(clip).await,
                )
                .unwrap();
                std::fs::write(
                    artifacts.join(format!("reference-{theme_key}-{page}-hotkey-modified.json")),
                    serde_json::to_vec_pretty(&feature_metrics().await).unwrap(),
                )
                .unwrap();
                evaluate("document.querySelector('.settings-hotkey-row button[aria-label=\"恢复默认值\"]').click(); 'ok'").await;
                wait_for("document.querySelector('.settings-hotkey-row button[aria-label=\"已是默认值\"]') ? 'ready' : ''", "Feature hotkey reset did not complete").await;
                assert_eq!(evaluate("window.settingsFeatureReference.snapshot().alerts.length===0 ? 'ready' : JSON.stringify(window.settingsFeatureReference.snapshot().alerts)").await, "ready");
            }
        }
    }

    async fn capture_permission_references(artifacts: &std::path::Path, appearance: &'static str) {
        for theme in ["native", "studio", "paper"] {
            for scenario in ["initial", "attempted", "error", "installation"] {
                on_main(move||REFERENCE.with_borrow(|reference|unsafe{let url=NSURL::URLWithString(&NSString::from_str(&format!("http://127.0.0.1:5194/reference/web/fixtures/permission_reference.html?theme={theme}&appearance={appearance}&scenario={scenario}"))).unwrap();reference.as_ref().unwrap().webview.loadRequest(&NSURLRequest::requestWithURL(&url));})).await;
                wait_for(format!("new URL(location.href).searchParams.get('theme')==='{theme}' && new URL(location.href).searchParams.get('scenario')==='{scenario}' && document.querySelector('[role=dialog]') && !document.querySelector('button:disabled') && ('{scenario}'==='initial'||'{scenario}'==='installation'||'{scenario}'==='error'&&document.querySelector('[role=alert]')||'{scenario}'==='attempted'&&document.querySelectorAll('button').length===3)?'ready':''"),"Permission guide reference did not load").await;
                tokio::time::sleep(Duration::from_millis(100)).await;
                let metrics:serde_json::Value=serde_json::from_str(&evaluate(r#"JSON.stringify((()=>{const card=document.querySelector('[role=dialog]');const rect=e=>{const r=e.getBoundingClientRect();return[r.x,r.y,r.width,r.height]};return{card:rect(card),controls:[...card.querySelectorAll('button')].map(e=>({text:e.textContent,frame:rect(e),disabled:e.disabled})),elements:[...card.children].map(e=>({tag:e.tagName,text:e.innerText,frame:rect(e),font:getComputedStyle(e).fontSize,line:getComputedStyle(e).lineHeight,margin:getComputedStyle(e).margin})),text:card.innerText}})())"#).await).unwrap();
                let r = &metrics["card"];
                let clip = NSRect::new(
                    NSPoint::new(r[0].as_f64().unwrap(), r[1].as_f64().unwrap()),
                    NSSize::new(r[2].as_f64().unwrap(), r[3].as_f64().unwrap()),
                );
                let key = format!(
                    "{theme}{}-{scenario}",
                    if appearance == "dark" { "-dark" } else { "" }
                );
                std::fs::write(
                    artifacts.join(format!("reference-permission-{key}.png")),
                    snapshot(clip).await,
                )
                .unwrap();
                std::fs::write(
                    artifacts.join(format!("reference-permission-{key}.json")),
                    serde_json::to_vec_pretty(&metrics).unwrap(),
                )
                .unwrap();
            }
        }
    }
    pub(super) fn run() {
        let mtm = MainThreadMarker::new().unwrap();
        let app = NSApplication::sharedApplication(mtm);
        let screenshot_only = std::env::args().any(|argument| argument == "screenshot");
        let library_pages = std::env::args().any(|argument| argument == "library");
        let services_pages = std::env::args().any(|argument| argument == "services");
        let permission_pages = std::env::args().any(|argument| argument == "permissions");
        let feature_pages =
            screenshot_only || std::env::args().any(|argument| argument == "features");
        let compact = std::env::args().any(|argument| argument == "compact");
        let appearance = if std::env::args().any(|argument| argument == "dark") {
            "dark"
        } else {
            "light"
        };
        let frame = NSRect::new(
            NSPoint::new(0.0, 0.0),
            NSSize::new(if compact { 700.0 } else { 900.0 }, 650.0),
        );
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
                let artifacts = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/native-settings");
                std::fs::create_dir_all(&artifacts).unwrap();
                if permission_pages{capture_permission_references(&artifacts,appearance).await;}
                else if library_pages { capture_library_references(&artifacts,compact,appearance).await; }
                else if services_pages { capture_services_references(&artifacts,compact,appearance).await; }
                else if feature_pages { capture_feature_references(&artifacts, compact, appearance, screenshot_only).await; } else {
                for theme in ["native", "studio", "paper"] {
                    let theme_key=format!("{theme}{}{}",if appearance=="dark"{"-dark"}else{""},if compact{"-compact"}else{""});
                    on_main(move || REFERENCE.with_borrow(|reference| unsafe {
                        let url = NSURL::URLWithString(&NSString::from_str(&format!("http://127.0.0.1:5194/reference/web/fixtures/settings_reference.html?theme={theme}&appearance={appearance}"))).unwrap();
                        reference.as_ref().unwrap().webview.loadRequest(&NSURLRequest::requestWithURL(&url));
                    })).await;
                    let deadline = Instant::now() + Duration::from_secs(10);
                    let metrics = loop {
                        let value = evaluate(format!(r#"(() => {{
                            if (document.documentElement.dataset.interfaceTheme !== '{theme}' || new URL(location.href).searchParams.get('theme') !== '{theme}') return '';
                            if (document.querySelectorAll('.theme-option').length !== 3) return '';
                            const frame = el => {{ const r = el.getBoundingClientRect(); return {{x:r.x,y:r.y,width:r.width,height:r.height}}; }};
                            return JSON.stringify({{theme:document.documentElement.dataset.interfaceTheme, content:frame(document.querySelector('.settings-content')), header:frame(document.querySelector('.settings-page-header')), card:frame(document.querySelector('.settings-card')), themes:[...document.querySelectorAll('.theme-option')].map(frame), rows:[...document.querySelectorAll('.settings-control-row')].map(frame), controls:[...document.querySelectorAll('.settings-control-row button,.settings-control-row input')].map(frame)}});
                        }})()"#)).await;
                        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&value) { break value; }
                        assert!(Instant::now() < deadline, "React reference did not render; check the Vite server and fixture");
                        tokio::time::sleep(Duration::from_millis(50)).await;
                    };
                    tokio::time::sleep(Duration::from_millis(200)).await;
                    let c = &metrics["content"];
                    let clip = NSRect::new(NSPoint::new(c["x"].as_f64().unwrap(), c["y"].as_f64().unwrap()), NSSize::new(c["width"].as_f64().unwrap(), c["height"].as_f64().unwrap()));
                    std::fs::write(artifacts.join(format!("reference-{theme}.png")), snapshot(clip).await).unwrap();
                    std::fs::write(artifacts.join(format!("reference-{theme}.json")), serde_json::to_vec_pretty(&metrics).unwrap()).unwrap();
                    std::fs::write(artifacts.join(format!("reference-{theme_key}-general-interface.json")),serde_json::to_vec_pretty(&feature_metrics().await).unwrap()).unwrap();
                    for (index, section) in [(1,"permissions"),(2,"network"),(3,"maintenance"),(4,"experimental"),(5,"about")] {
                        evaluate(format!(r#"(() => {{
                            const area=document.querySelector('.settings-scroll-area');
                            const card=document.querySelectorAll('.settings-card')[{index}];
                            const h=document.querySelector('.settings-page-header').offsetHeight;
                            area.style.scrollBehavior='auto';
                            area.scrollTop=card.getBoundingClientRect().top-area.getBoundingClientRect().top+area.scrollTop-h-16;
                            area.dispatchEvent(new Event('scroll'));
                            return 'ok';
                        }})()"#)).await;
                        tokio::time::sleep(Duration::from_millis(100)).await;
                        std::fs::write(artifacts.join(format!("reference-{theme}-{section}.png")),snapshot(clip).await).unwrap();
                        std::fs::write(artifacts.join(format!("reference-{theme_key}-general-{section}.json")),serde_json::to_vec_pretty(&feature_metrics().await).unwrap()).unwrap();
                    }
                    for (scenario,index,section) in [("recovery",1,"permissions-recovery"),("update",5,"about-update"),("logs",3,"maintenance-logs")] {
                        on_main(move || REFERENCE.with_borrow(|reference| unsafe {
                            let url=NSURL::URLWithString(&NSString::from_str(&format!("http://127.0.0.1:5194/reference/web/fixtures/settings_reference.html?theme={theme}&appearance={appearance}&scenario={scenario}"))).unwrap();
                            reference.as_ref().unwrap().webview.loadRequest(&NSURLRequest::requestWithURL(&url));
                        })).await;
                        let deadline=Instant::now()+Duration::from_secs(10);
                        loop {
                            if evaluate(format!("document.documentElement.dataset.scenario==='{scenario}' && document.documentElement.dataset.interfaceTheme==='{theme}' && document.querySelectorAll('.settings-card').length===6 ? 'ready' : ''")).await=="ready" {break;}
                            assert!(Instant::now()<deadline,"General scenario reference did not render");
                            tokio::time::sleep(Duration::from_millis(50)).await;
                        }
                        tokio::time::sleep(Duration::from_millis(150)).await;
                        evaluate(format!(r#"(() => {{
                            if ('{scenario}'==='recovery') document.querySelectorAll('.settings-card')[1].querySelector('details').open=true;
                            if ('{scenario}'==='update') document.querySelectorAll('.settings-card')[5].querySelector('details').open=true;
                            const area=document.querySelector('.settings-scroll-area');
                            const card=document.querySelectorAll('.settings-card')[{index}];
                            area.style.scrollBehavior='auto';area.scrollTop=card.getBoundingClientRect().top-area.getBoundingClientRect().top+area.scrollTop-document.querySelector('.settings-page-header').offsetHeight-16;
                            area.dispatchEvent(new Event('scroll'));return 'ok';
                        }})()"#)).await;
                        tokio::time::sleep(Duration::from_millis(100)).await;
                        std::fs::write(artifacts.join(format!("reference-{theme}-{section}.png")),snapshot(clip).await).unwrap();
                    }

                }
                for (theme, appearance) in [("native", "light"), ("paper", "dark")] {
                    on_main(move || REFERENCE.with_borrow(|reference| unsafe {
                        let url = NSURL::URLWithString(&NSString::from_str(&format!("http://127.0.0.1:5194/reference/web/fixtures/settings_reference.html?theme={theme}&appearance={appearance}&notices=1"))).unwrap();
                        reference.as_ref().unwrap().webview.loadRequest(&NSURLRequest::requestWithURL(&url));
                    })).await;
                    let deadline = Instant::now() + Duration::from_secs(10);
                    loop {
                        let script = format!("document.documentElement.dataset.interfaceTheme === '{theme}' && document.documentElement.dataset.theme === '{appearance}' && document.querySelector('.third-party-notices[open]') ? 'ready' : ''");
                        if evaluate(script).await == "ready" { break; }
                        assert!(Instant::now() < deadline, "Notices reference did not render");
                        tokio::time::sleep(Duration::from_millis(50)).await;
                    }
                    tokio::time::sleep(Duration::from_millis(200)).await;
                    let clip = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(900.0, 650.0));
                    std::fs::write(artifacts.join(format!("reference-{theme}-notices.png")), snapshot(clip).await).unwrap();
                    let metrics = evaluate(r#"JSON.stringify([...document.querySelectorAll('.third-party-notices, .third-party-notices header, .third-party-notices h2, .third-party-notices header p, .third-party-notices header button, .third-party-notices input, .third-party-notices table, .third-party-notices th, .third-party-notices tbody tr:first-child td')].map(el => { const r=el.getBoundingClientRect(), s=getComputedStyle(el); return {tag:el.tagName,x:r.x,y:r.y,width:r.width,height:r.height,font:s.fontSize,line:s.lineHeight,padding:s.padding,color:s.color}; }))"#).await;
                    std::fs::write(artifacts.join(format!("reference-{theme}-notices.json")), metrics).unwrap();
                    evaluate(r#"(() => { const input=document.querySelector('.third-party-notices input'); Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,'value').set.call(input,'objc2'); input.dispatchEvent(new Event('input',{bubbles:true})); return 'ok'; })()"#).await;
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    std::fs::write(artifacts.join(format!("reference-{theme}-notices-list.png")), snapshot(clip).await).unwrap();
                    let metrics = evaluate(r#"JSON.stringify([...document.querySelectorAll('.third-party-notices th')].map(el => { const r=el.getBoundingClientRect();return {x:r.x,y:r.y,width:r.width,height:r.height}; }))"#).await;
                    std::fs::write(artifacts.join(format!("reference-{theme}-notices-list.json")), metrics).unwrap();
                    evaluate("document.querySelector('.third-party-notices tbody button').click(); 'ok'").await;
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    std::fs::write(artifacts.join(format!("reference-{theme}-notices-detail.png")), snapshot(clip).await).unwrap();
                    let metrics = evaluate(r#"JSON.stringify([...document.querySelectorAll('.third-party-notices h3, .third-party-notices h3 + button, .third-party-notices pre, .third-party-notices p')].map(el => { const r=el.getBoundingClientRect(),s=getComputedStyle(el);return {tag:el.tagName,x:r.x,y:r.y,width:r.width,height:r.height,font:s.fontSize,line:s.lineHeight,padding:s.padding,color:s.color}; }))"#).await;
                    std::fs::write(artifacts.join(format!("reference-{theme}-notices-detail.json")), metrics).unwrap();
                    evaluate("[...document.querySelectorAll('.third-party-notices button')].find(button=>button.textContent.includes('返回组件列表')).click(); 'ok'").await;
                    tokio::time::sleep(Duration::from_millis(50)).await;
                    evaluate(r#"(() => {const input=document.querySelector('.third-party-notices input');Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,'value').set.call(input,'');input.dispatchEvent(new Event('input',{bubbles:true}));return 'ok';})()"#).await;
                    tokio::time::sleep(Duration::from_millis(50)).await;
                    evaluate("document.querySelector('.third-party-notices tbody button').click(); 'ok'").await;
                    tokio::time::sleep(Duration::from_millis(50)).await;
                    std::fs::write(artifacts.join(format!("reference-{theme}-notices-licenses.png")),snapshot(clip).await).unwrap();
                    let metrics=evaluate(r#"JSON.stringify([...document.querySelectorAll('.third-party-notices pre')].map(el=>{const r=el.getBoundingClientRect(),s=getComputedStyle(el);return{x:r.x,y:r.y,width:r.width,height:r.height,font:s.fontSize,line:s.lineHeight,padding:s.padding};}))"#).await;
                    std::fs::write(artifacts.join(format!("reference-{theme}-notices-licenses.json")),metrics).unwrap();
                }
                }
                on_main(|| {
                    REFERENCE.with_borrow_mut(|reference| reference.take().unwrap().window.close());
                    let app = NSApplication::sharedApplication(MainThreadMarker::new().unwrap()); app.stop(None);
                    let wake = NSEvent::otherEventWithType_location_modifierFlags_timestamp_windowNumber_context_subtype_data1_data2(NSEventType::ApplicationDefined, NSPoint::new(0.0, 0.0), NSEventModifierFlags::empty(), 0.0, 0, None, 0, 0, 0).unwrap(); app.postEvent_atStart(&wake, true);
                }).await;
            })
            }));
            if result.is_err() {
                eprintln!("Settings reference snapshot failed");
                std::process::exit(1);
            }
        });
        app.run();
        println!("Captured three existing React Settings references through native WebKit without Tauri initialization");
    }
}
