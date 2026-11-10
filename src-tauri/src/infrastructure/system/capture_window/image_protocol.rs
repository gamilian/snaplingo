use tauri::{http, Manager, UriSchemeContext, Wry};

use crate::domain::capture::CaptureSessionId;

pub(crate) fn respond(
    context: UriSchemeContext<'_, Wry>,
    request: http::Request<Vec<u8>>,
) -> http::Response<Vec<u8>> {
    let pixels = (|| {
        if context.webview_label() != super::backend::CAPTURE_WINDOW_LABEL {
            return None;
        }
        let (session_id, monitor_id) = image_ids(request.uri().path())?;
        let state = context.app_handle().try_state::<crate::AppState>()?;
        state
            .capture
            .sessions
            .monitor_snapshot_png(&session_id, &monitor_id)
            .ok()
    })();
    http::Response::builder()
        .status(if pixels.is_some() { 200 } else { 404 })
        .header(http::header::CONTENT_TYPE, "image/png")
        .header(http::header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
        .header(http::header::CACHE_CONTROL, "no-store")
        .body(pixels.unwrap_or_default())
        .expect("static capture image response headers are valid")
}

fn image_ids(path: &str) -> Option<(CaptureSessionId, String)> {
    let decoded = urlencoding::decode(path).ok()?;
    let (session, monitor) = decoded.trim_start_matches('/').split_once('/')?;
    if session.is_empty() || monitor.is_empty() {
        return None;
    }
    Some((CaptureSessionId(session.to_owned()), monitor.to_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_identifiers_from_a_converted_image_url() {
        assert_eq!(
            image_ids("/%2Fcapture-1%2FDisplay%20A"),
            Some((CaptureSessionId("capture-1".into()), "Display A".into()))
        );
        assert_eq!(
            image_ids("/capture-1/monitor-2"),
            Some((CaptureSessionId("capture-1".into()), "monitor-2".into()))
        );
        assert_eq!(image_ids("/capture-1"), None);
        assert_eq!(image_ids("/capture-1/"), None);
    }
}
