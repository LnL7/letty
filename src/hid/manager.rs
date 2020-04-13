use std::ffi::c_void;
use std::io;
use std::ptr;

use crate::hid::device::IOHIDDevice;

use core_foundation::base::{TCFType, CFRelease, kCFAllocatorDefault};
use core_foundation::dictionary::CFDictionary;
use core_foundation::number::CFNumber;
use core_foundation::set::{CFSetGetCount, CFSetApplyFunction};
use core_foundation::string::CFString;
use io_kit_sys::hid::base::{IOHIDDeviceRef};
use io_kit_sys::hid::keys::kIOHIDOptionsTypeNone;
use io_kit_sys::hid::manager::*;
use io_kit_sys::ret::{kIOReturnSuccess, IOReturn};


#[repr(C)]
#[derive(Debug)]
pub struct IOHIDManager(IOHIDManagerRef);

impl_TCFType!(IOHIDManager, IOHIDManagerRef, IOHIDManagerGetTypeID);

impl Drop for IOHIDManager {
    fn drop(&mut self) {
        self.close();

        // eprintln!("  <IOHIDManager drop {}>", self.retain_count());
        unsafe { CFRelease(self.as_CFTypeRef()) }
    }
}

impl IOHIDManager {
    pub fn new() -> Result<Self, io::Error> {
        let value = unsafe { IOHIDManagerCreate(kCFAllocatorDefault, kIOHIDManagerOptionNone) };
        if value.is_null() {
            return Err(io::Error::new(io::ErrorKind::Other, "failed to create manager"));
        }

        let mut manager = IOHIDManager(value);
        manager.open()
            .map_err(|_e| io::Error::new(io::ErrorKind::Other, "failed to open manager"))?;

        Ok(manager)
    }

    pub fn open(&mut self) -> Result<(), IOReturn> {
        let ret = unsafe { IOHIDManagerOpen(self.0, kIOHIDOptionsTypeNone) };
        match ret {
            kIOReturnSuccess => Ok(()),
            _ => Err(ret),
        }
    }

    pub fn close(&mut self) {
        unsafe { IOHIDManagerSetDeviceMatching(self.0, ptr::null_mut()) }
        unsafe { IOHIDManagerClose(self.0, kIOHIDOptionsTypeNone); };
    }

    pub fn set_device_matching(&mut self, dict: &CFDictionary<CFString, CFNumber>) {
        unsafe { IOHIDManagerSetDeviceMatching(self.0, dict.as_concrete_TypeRef()) }
    }

    pub fn get_devices(&mut self) -> Vec<IOHIDDevice> {
        let devices_set = unsafe { IOHIDManagerCopyDevices(self.0) };
        if devices_set.is_null() {
            vec![]
        } else {
            let device_count = unsafe { CFSetGetCount(devices_set) };
            let mut devices = Vec::with_capacity(device_count as usize);
            let context = &mut devices as *mut _ as *mut c_void;
            unsafe { CFSetApplyFunction(devices_set, vec_push_applier, context) };

            unsafe { CFRelease(devices_set as _) };

            devices
        }
    }
}

extern "C" fn vec_push_applier(value: *const c_void, context: *const c_void) {
    let vec = unsafe { &mut *(context as *mut Vec<IOHIDDevice>) };
    // Moving ownersip of devices from the copied set to rust, since the array will be
    // released before returning.
    let device = unsafe { IOHIDDevice::wrap_under_get_rule(value as IOHIDDeviceRef) };
    vec.push(device);
}
