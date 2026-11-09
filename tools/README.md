# Development tools

Production crates live under `crates/`. These executables collect independent
references or inspect production behavior; they are excluded from native bundles.
Rust entries live directly beside their manifests or explicit Cargo targets.

## Browser reference collectors

`reference/` is the unpublished `snaplingo-reference` workspace package. It owns
the macOS WebKit dependencies and builds without App, Core or the native OCR
runtime. Default application builds do not select this package.

From the repository root, start the fixture server:

```sh
npm run reference:dev -- --host 127.0.0.1 --port 5194
```

In another terminal, capture Settings or Result references:

```sh
cargo run --locked -p snaplingo-reference --bin settings_snapshot
cargo run --locked -p snaplingo-reference --bin settings_snapshot -- features dark compact
cargo run --locked -p snaplingo-reference --bin settings_snapshot -- screenshot
cargo run --locked -p snaplingo-reference --bin settings_snapshot -- library
cargo run --locked -p snaplingo-reference --bin settings_snapshot -- services
cargo run --locked -p snaplingo-reference --bin settings_snapshot -- permissions
cargo run --locked -p snaplingo-reference --bin result_snapshot
```

Settings output stays under `target/native-settings`; Result output stays under
`target/shared-result/reference`. The collectors use isolated React fixtures and
never open the installed application's database. They run on macOS; Windows
cross compilation checks only the platform gates.

## Native probes and tests

`native/pin_snapshot.rs` is an explicit App example because it exercises App's
private pinned-image decoding/preparation implementation with the shared UI and
renderer. Keeping the source mount in this development target avoids exporting
implementation details or copying pixel preparation into the probe.

```sh
cargo run --locked -p snaplingo --example pin_snapshot
cargo test --locked -p snaplingo --test single_instance_native
```

The pin probe writes synthetic images to `target/shared-pin`. The single-instance
integration test launches real child processes with a temporary lock directory,
checking early/live activation and takeover after process termination. Ordinary
App tests and CI run it automatically.

Reusable UI geometry reports, renderer snapshots and the standalone Settings
preview remain in their owning library's `examples/` directory and run with
`cargo run -p <package> --example <name>`.
