//! A simple user-mode application that emits WPP trace messages over ETW.

#![feature(codeview_annotation)]
#![feature(core_os)]

use wpp::wpp_control_guids;

wpp_control_guids!(
    UserModeApp 6f1c2b8a-3d4e-4f5a-9b6c-7d8e9f0a1b2c {
        GENERAL,
        WORK,
    }
);

fn main() {
    // SAFETY: `clean_up` is called before the program exits.
    unsafe { UserModeApp::init() };

    trace!(
        INFO,
        GENERAL,
        "User-mode app started, pid={}",
        std::process::id()
    );

    let mut total = 0_u64;
    for item in 1..=5_u32 {
        total += u64::from(item);
        trace!(
            VERBOSE,
            WORK, "Processed item={}, running total={}", item, total
        );
    }

    let name = "wpp-user-mode";
    trace!(
        INFO,
        GENERAL, "Done: name={}, total={}, ok={}", name, total, true
    );
    trace!(INFO, "Default provider message without keyword");

    UserModeApp::clean_up();

    println!("Processed 5 items, total={total}");
}
