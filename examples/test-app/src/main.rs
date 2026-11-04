//! A simple user-mode application that emits WPP trace messages over ETW.

#![feature(codeview_annotation)]
#![feature(core_os)]

use wpp::wpp_control_guids;

const SOME_CONST_VALUE: usize = 1024 * 40;

/// u8 size field less enum variant demonstration
#[allow(dead_code)]
#[repr(u8)]
#[derive(Clone, Copy, wpp::WppField)]
enum SampleByteState {
    Unknown = 0,
    Starting = 1,
    Ready = 2,
}

/// u16 size field less enum variant demonstration
#[allow(dead_code)]
#[repr(u16)]
#[derive(Clone, Copy, wpp::WppField)]
enum SampleShortState {
    Unknown = 0,
    Warning = 10,
    Failed = 100,
}

/// u32 size field less enum variant demonstration
#[allow(dead_code)]
#[repr(u32)]
#[derive(Clone, Copy, wpp::WppField)]
enum SampleLongState {
    Offline = 0,
    Online = 1,
    Recovering = 2,
}

/// Implicitly valued enum variant demonstration.
#[allow(dead_code)]
#[repr(u8)]
#[derive(Clone, Copy, wpp::WppField)]
enum SampleImplicitState {
    Unknown,
    Starting,
    Ready,
}


/// Mixed implicitly and explicitly valued enum variant demonstration.
#[allow(dead_code)]
#[repr(u16)]
#[derive(Clone, Copy, wpp::WppField)]
enum SampleMixedState {
    Unknown,
    Warning = 10,
    Failed,
}

// WPP control GUIDs declaration
wpp_control_guids!(
    UserModeApp "34746ee9-2982-4967-aac1-fece7a3dfc3c" {
        GENERAL,
        LITERALS,
        VARIABLES,
        WDF_TYPES,
        ENUMS,
        WORK,
    }
    SecondProvider "c5a04b92-0ed8-45ec-9732-aa27fdc95e45" {
        GENERICS,
        DEFAULT
    }
);

fn main() {
    // SAFETY: `clean_up` is called before the program exits.
    unsafe { UserModeApp::init(); SecondProvider::init(); };

    trace!(
        INFO,
        GENERAL,
        "User-mode app started, pid={}",
        std::process::id()
    );
    
    // Boolean and C-string fields.
    trace!(
        INFO, LITERALS,
        "Flags: initialized={}, safe_mode={}, build={}",
        true, false, c"safe-rust"
    );
    
    // Signed integer widths.
    trace!(
        INFO, LITERALS,
        "Direct Signed literals: i8={}, i16={}, i32={}, i64={}, isize={}",
        -8_i8, -1600_i16, -320_000_i32, -64_000_000_000_i64, -42_isize
    );

    // Unsigned integer widths.
    trace!(
        INFO, LITERALS,
        "Direct Unsigned literals: u8={}, u16={}, u32={}, u64={}, usize={}",
        200_u8, 60_000_u16, 4_000_000_000_u32, 18_000_000_000_000_000_000_u64, SOME_CONST_VALUE
    );

    let v1: i8 = -8;
    let v2: i16 = -1600;
    let v3: i32 = -320_000;
    let v4: i64 = -64_000_000_000;
    let v5: isize = -42;
    let u1: u8 = 200;
    let u2: u16 = 60_000;
    let u3: u32 = 4_000_000_000;
    let u4: u64 = 18_000_000_000_000_000_000;
    let u5: usize = SOME_CONST_VALUE;

    trace!(
        INFO, VARIABLES,
        "Signed literal variables: v1={}, v2={}, v3={}, v4={}, v5={}",
        v1, v2, v3, v4, v5
    );

    trace!(
        INFO, VARIABLES,
        "Unsigned literal variables: u1={}, u2={}, u3={}, u4={}, u5={}",
        u1, u2, u3, u4, u5
    );

    // Custom enum fields
    let e1 = SampleByteState::Ready;
    trace!(
        INFO, ENUMS,
        "Custom enums: byte={}, short={}, long={}, implicit={}, mixed={}",
        e1,
        SampleShortState::Warning,
        SampleLongState::Recovering,
        SampleImplicitState::Ready,
        SampleMixedState::Failed
    );

    // Variables
    let mut total = 0_u64;
    for item in 1..=5_u32 {
        total += u64::from(item);
        trace!(
            VERBOSE,
            WORK, "Processed item={}, running total={}", item, total
        );
    }

    let name = "test-app";
    trace!(
        INFO,
        LITERALS, "Done: name={}, total={}, ok={}", name, total, true
    );

    trace!(
        INFO,
        LITERALS, "sample trace from app: name={}, ok={}", name, true
    );

    // Generics
    let v1 = 123_i32;
    let v2 = "another-generic-value";
    trace_generic_values(v1, v2);

    // Same trace statement with different generic types
    let v1 = i32::MAX;
    let v2 = i16::MAX;
    trace_generic_values(v1, v2);

    // Same trace statement with different generic types but having same WPP 
    // type representation. E.g: Both i32 and u32 is represented ItemLong for
    // the decoder.
    let v1 = u32::MAX;
    let v2 = u16::MAX;
    trace_generic_values(v1, v2);

    // Trace statement without a keyword, using the default provider.
    trace!(INFO, "Default provider message without keyword");
    
    SecondProvider::clean_up();
    UserModeApp::clean_up();
}

fn trace_generic_values<T: wpp::IntoWppField, U: wpp::IntoWppField>(first: T, second: U) {
    trace!(
        INFO, GENERICS,
        "Generic values: first={}, second={}",
        first, second
    );
}
