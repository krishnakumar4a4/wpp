# wpp

WPP software tracing for Rust Windows drivers.

## Crates

* [wpp](./crates/wpp): `no_std` tracing runtime, ETW bindings, field encoding, and
  kernel-mode Inflight Trace Recorder (IFR) support. Re-exports the tracing macros.
* [wpp-macros](./crates/wpp-macros): `wpp_control_guids!`, `WppField`, and the
  implementation of the generated `trace!` macro.

The `kernel_mode` feature selects kernel ETW APIs and IFR support. Without this
feature, the runtime uses user-mode ETW APIs from `onecore_apiset`.

Applications expanding tracing macros that emit PDB annotations currently require
nightly Rust and `#![feature(codeview_annotation)]`. Building and testing the
crates themselves does not require that feature gate. Linking a kernel-mode
driver still requires its usual WDK build environment.

## License

Licensed under either the [MIT license](./LICENSE-MIT) or
[Apache License 2.0](./LICENSE-APACHE), at your option.
