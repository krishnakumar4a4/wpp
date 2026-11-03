//! A barebones sample KMDF driver
//! Demonstrates WPP tracing

#![no_std]
#![feature(codeview_annotation)]
#![feature(core_os)]

use wdf::{
    driver_entry, wpp_control_guids, DeviceInit, Driver, DriverConfig, DriverObject,
    status_codes, IRQL, NtResult, NtStatus, UnicodeString,
};

extern crate alloc;

wpp_control_guids!(
    DriverTracer "f6912289-953b-48fa-b213-44bf82ae9623" {
        GENERAL,
        WDF_TYPES,
    }
    DeviceTracer "06e9c359-6a6f-4748-93e4-72f948e2302d" {
        INIT
    }
);

// Modules declared after wpp_control_guids! so trace macros are in scope
mod device;

#[driver_entry(trace_providers(DriverTracer, DeviceTracer))]
fn driver_entry(driver_object: &mut DriverObject, registry_path: &UnicodeString) -> NtResult<()> {
    let config = DriverConfig::new(evt_device_add);
    let _driver = Driver::create(driver_object, registry_path, config)?;

    // Predefined WDF enums
    trace!(
        INFO, WDF_TYPES,
        "NTSTATUS values: success={}, access_denied={}",
        NtStatus::from(status_codes::STATUS_SUCCESS),
        NtStatus::from(status_codes::STATUS_ACCESS_DENIED)
    );
    
    trace!(
        INFO, GENERAL,
        "Driver entry complete at IRQL={}",
        IRQL::current().unwrap_or(IRQL::Passive)
    );

    Ok(())
}


fn evt_device_add(device_init: &mut DeviceInit) -> NtResult<()> {
    trace!(VERBOSE, "Enter evt_device_add");
    device::device_create(device_init)
}
