The `react-result-*.json` files record actual `getBoundingClientRect` geometry and computed styles from the existing React `reference/web/views/ResultWindow/ResultWindow.tsx` rendered in WKWebView. They are independent of `ResultUi`.

Run Vite at `127.0.0.1:5194`, then `cargo run --locked -p snaplingo-reference --bin result_snapshot`. The isolated React fixture is `reference/web/fixtures/result_reference.tsx`; it injects only state and runtime ports. It does not start Tauri or alter installed settings. Screenshots and measurements go to `target/shared-result/reference`.

These measurements preserve the browser migration reference. The shared Result
presentation now follows the native macOS window, so its acceptance baseline is
`crates/ui/tests/fixtures/native-result-layout.json`. The Rust layout and render
tests compare against those independently captured AppKit frames. Browser
measurements stay here alongside their source fixture and do not define native
geometry. Both implementations retain the original email and date copy targets.
