# Retained browser references

The desktop application runs in the Rust native host. This React tree retains
the original views, fixtures and application/domain tests for independent visual
and behavior comparisons. It has no desktop IPC adapter or Tauri dependency.

`npm run reference:dev` opens the Settings reference by default. Existing query parameters
select its page, route, theme, appearance and scenario. For example:

- `/?page=services&route=tts&appearance=dark`
- `/?page=history&theme=paper`
- `/?surface=result&mode=ocr&scenario=image`
- `/?surface=permission&scenario=attempted`

The Vite root, TypeScript/CSS configuration, HTML entry and public resources all
live in this directory. Root npm commands remain the common entry point; the
root Vitest configuration runs both `script/` and browser-reference tests.

The entry preserves these parameters and loads the isolated HTML fixture under
`/fixtures/` (for example `/fixtures/settings_reference.html`). Collectors open
those URLs directly. The browser reference bundle is built by
`npm run reference:build` into `reference/web/dist`; `npm run reference:preview`
serves that bundle. Native
application packaging uses `npm run native:build` and includes no frontend assets.

The macOS WebKit collectors live in `tools/reference`. Start Vite on port 5194,
then run `cargo run --locked -p snaplingo-reference --bin settings_snapshot` or
`cargo run --locked -p snaplingo-reference --bin result_snapshot`. Scene options
follow `--`; see [development tools](../../tools/README.md).
