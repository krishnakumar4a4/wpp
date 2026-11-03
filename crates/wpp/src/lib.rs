// Copyright (c) Microsoft Corporation
// License: MIT OR Apache-2.0

//! WPP software tracing runtime for Rust drivers.
//!
//! Provides the runtime types and ETW bindings needed by the code generated
//! by `wpp_control_guids!` and `trace!` macros.

#![no_std]

extern crate alloc;

pub mod etw;
pub mod field;
// IFR (WppRecorder) is kernel-mode only; user-mode builds trace through ETW alone.
#[cfg(feature = "kernel_mode")]
pub mod ifr;
pub mod provider;

pub use field::{IntoWppField, WppField, WppValue};
#[cfg(feature = "kernel_mode")]
pub use ifr::IFRState;
pub use provider::ProviderState;
pub use wpp_macros::{__wpp_trace_impl, WppField, wpp_control_guids};

/// Expands its input only when the `kernel_mode` feature is enabled.
///
/// Proc macros cannot see this crate's features, so generated IFR code is
/// wrapped in this macro to keep it out of user-mode builds.
#[doc(hidden)]
#[cfg(feature = "kernel_mode")]
#[macro_export]
macro_rules! __wpp_kernel_mode_only {
    ($($tokens:tt)*) => { $($tokens)* };
}

#[doc(hidden)]
#[cfg(not(feature = "kernel_mode"))]
#[macro_export]
macro_rules! __wpp_kernel_mode_only {
    ($($tokens:tt)*) => {};
}

/// GUID layout matching the Windows GUID structure.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct GUID {
    pub data1: u32,
    pub data2: u16,
    pub data3: u16,
    pub data4: [u8; 8],
}
