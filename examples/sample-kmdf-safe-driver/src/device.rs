//! Device creation

use wdf::{Device, DeviceInit, NtResult};

pub(crate) fn device_create(device_init: &mut DeviceInit) -> NtResult<()> {
    let device = Device::create(device_init, None)?;
    trace!(VERBOSE, INIT, "Device created successfully");
    NtResult::Ok(())
}