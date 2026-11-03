# wpp

WPP software tracing for Rust Windows drivers.

## Crates

* [wpp](./crates/wpp): `no_std` tracing runtime, ETW bindings, field encoding, and
  kernel-mode Inflight Trace Recorder (IFR) support. Re-exports the tracing macros.
* [wpp-macros](./crates/wpp-macros): `wpp_control_guids!`, `WppField`, and the
  implementation of the generated `trace!` macro.

The `kernel_mode` feature selects kernel ETW APIs and IFR support. Without this
feature, the runtime uses user-mode ETW APIs from `onecore_apiset`, and the
generated tracing code omits IFR, which is only available in kernel mode.

Applications expanding tracing macros that emit PDB annotations currently require
nightly Rust and `#![feature(codeview_annotation)]`. Building and testing the
crates themselves does not require that feature gate. Linking a kernel-mode
driver still requires its usual WDK build environment.

Control GUIDs passed to `wpp_control_guids!` must be quoted string literals,
for example `MyProvider "84bdb2e9-829e-41b3-b891-02f454bc2bd7" { GENERAL }`.

## Examples

Each example is built separately from its own directory with `cargo +nightly build`.

* [sample-kmdf-safe-driver](./examples/sample-kmdf-safe-driver): KMDF driver
  tracing through ETW and IFR.
* [user-mode-app](./examples/user-mode-app): Rust console application tracing
  through ETW.

## License

Licensed under either the [MIT license](./LICENSE-MIT) or
[Apache License 2.0](./LICENSE-APACHE), at your option.
