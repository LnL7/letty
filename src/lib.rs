#![allow(non_upper_case_globals)]

#[macro_use(impl_TCFType)]
extern crate core_foundation;
extern crate core_foundation_sys;
extern crate mach;

extern crate io_kit_sys;

use std::error;
use std::ffi::{CStr, c_void};
use std::io;
use std::ptr;

use core_foundation::base::{kCFAllocatorDefault, CFGetRetainCount, CFRelease, TCFType};
use core_foundation::dictionary::{CFDictionary, CFDictionaryRef};
use core_foundation::number::CFNumber;
use core_foundation::array::{CFArrayGetCount, CFArrayGetValueAtIndex};
use core_foundation::set::{CFSetGetCount, CFSetApplyFunction};
use core_foundation::string::{kCFStringEncodingUTF8, CFString, CFStringCreateWithCString};
use io_kit_sys::hid::base::*;
use io_kit_sys::hid::device::*;
use io_kit_sys::hid::element::*;
use io_kit_sys::hid::keys::{kIOHIDOptionsTypeNone, kIOHIDDeviceUsagePageKey, kIOHIDDeviceUsageKey, kIOHIDProductKey, kIOHIDElementUsagePageKey, kIOHIDElementUsageKey};
use io_kit_sys::hid::manager::*;
use io_kit_sys::hid::value::*;
use io_kit_sys::hid::usage_tables::{kHIDPage_GenericDesktop, kHIDUsage_GD_Keyboard, kHIDUsage_LED_CapsLock, kHIDPage_LEDs};
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

            // eprintln!("  <CFSet release {}>", unsafe { CFGetRetainCount(devices_set as _) });
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
    // eprintln!("  <IOHIDDevice retain {}>", unsafe { CFGetRetainCount(device.as_CFTypeRef()) });
    vec.push(device);
}

#[repr(C)]
#[derive(Debug)]
pub struct IOHIDDevice(IOHIDDeviceRef);

impl_TCFType!(IOHIDDevice, IOHIDDeviceRef, IOHIDDeviceGetTypeID);

impl Drop for IOHIDDevice {
    fn drop(&mut self) {
        // eprintln!("  <IOHIDDevice drop {}>", self.retain_count());
        unsafe { CFRelease(self.as_CFTypeRef()) }
    }
}

impl IOHIDDevice {
    pub fn get_matching_elements(&self, dict: &CFDictionary<CFString, CFNumber>) -> Result<Vec<IOHIDElement>, io::Error> {
        let matching_dict = dict.as_CFTypeRef() as CFDictionaryRef;
        let elements = unsafe { IOHIDDeviceCopyMatchingElements(self.0, matching_dict, kIOHIDOptionsTypeNone) };

        if elements.is_null() {
            return Err(io::Error::new(io::ErrorKind::Other, "failed to obtain HID elements"))
        };

        let count = unsafe { CFArrayGetCount(elements) };
        let mut vec = Vec::with_capacity(count as _);

        for i in 0..count {
            let value = unsafe { CFArrayGetValueAtIndex(elements, i) };
            if value.is_null() {
                unreachable!("failed to obtain element at index {}", i);
            }

            // Moving ownersip of elements from the copied array to rust, since the array will be
            // released before returning.
            let element = unsafe { IOHIDElement::wrap_under_get_rule(value as IOHIDElementRef) };
            // eprintln!("  <IOHIDElement retain {}>", unsafe { CFGetRetainCount(element.as_CFTypeRef()) });
            vec.push(element);
        }

        // eprintln!("  <CFSet release {}>", unsafe { CFGetRetainCount(elements as _) });
        unsafe { CFRelease(elements as _) };

        Ok(vec)
    }

    pub fn get_name(&self) -> Option<String> {
        let product_key = unsafe { CFStringCreateWithCString(kCFAllocatorDefault, kIOHIDProductKey, kCFStringEncodingUTF8) };
        let property = unsafe { IOHIDDeviceGetProperty(self.0, product_key) };

        unsafe { CFRelease(product_key as _) };

        if property.is_null() {
            return None;
        }

        let name = unsafe { CFString::wrap_under_get_rule(property as _) };
        Some(name.to_string())
    }

    pub fn get_value(&self, element: &IOHIDElement) -> Result<i64, IOReturn> {
        let mut value: IOHIDValueRef = ptr::null_mut();
        let ret = unsafe { IOHIDDeviceGetValue(self.0, element.0, &mut value) };

        // Don't release here?
        // eprintln!("  <IOHIDValue {}>", unsafe { CFGetRetainCount(value as _) });
        match ret {
            kIOReturnSuccess => Ok(unsafe { IOHIDValueGetIntegerValue(value) }),
            _ => Err(ret),
        }
    }

    pub fn set_value(&mut self, element: &mut IOHIDElement, new: i64) -> Result<i64, IOReturn> {
        let value = unsafe { IOHIDValueCreateWithIntegerValue(kCFAllocatorDefault, element.0, 0, new) };
        let ret = unsafe { IOHIDDeviceSetValue(self.0, element.0, value) };

        // eprintln!("  <IOHIDValue release {}>", unsafe { CFGetRetainCount(value as _) });
        unsafe { CFRelease(value as _) };

        match ret {
            kIOReturnSuccess => Ok(new),
            _ => Err(ret),
        }
    }

    pub fn toggle_value(&mut self, element: &mut IOHIDElement) -> Result<i64, IOReturn> {
        let current = self.get_value(&element)?;
        let min = unsafe { IOHIDElementGetLogicalMin(element.0) };
        let max = unsafe { IOHIDElementGetLogicalMax(element.0) };
        self.set_value(element, if current > min { min } else { max })
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct IOHIDElement(IOHIDElementRef);

impl_TCFType!(IOHIDElement, IOHIDElementRef, IOHIDElementGetTypeID);

impl Drop for IOHIDElement {
    fn drop(&mut self) {
        // eprintln!("  <IOHIDElement drop {}>", self.retain_count());
        unsafe { CFRelease(self.as_CFTypeRef()) }
    }
}

impl IOHIDElement {
    pub fn is_led_capslock(&self) -> bool {
        let page = unsafe { IOHIDElementGetUsagePage(self.0) };
        let usage = unsafe { IOHIDElementGetUsage(self.0) };
        kHIDPage_LEDs == page && usage == kHIDUsage_LED_CapsLock
    }
}

pub fn keyboard_matching_dictionary() -> CFDictionary<CFString, CFNumber> {
    let page_str = unsafe { CStr::from_ptr(kIOHIDDeviceUsagePageKey) };
    let page_key = CFString::from(page_str.to_str().unwrap());
    let page_value = CFNumber::from(kHIDPage_GenericDesktop as i32);

    let usage_str = unsafe { CStr::from_ptr(kIOHIDDeviceUsageKey) };
    let usage_key = CFString::from(usage_str.to_str().unwrap());
    let usage_value = CFNumber::from(kHIDUsage_GD_Keyboard as i32);

    CFDictionary::from_CFType_pairs(&[(page_key, page_value), (usage_key, usage_value)])
}

pub fn capslock_matching_dictionary() -> CFDictionary<CFString, CFNumber> {
    let page_str = unsafe { CStr::from_ptr(kIOHIDElementUsagePageKey) };
    let page_key = CFString::from(page_str.to_str().unwrap());
    let page_value = CFNumber::from(kHIDPage_LEDs as i32);

    let usage_str = unsafe { CStr::from_ptr(kIOHIDElementUsageKey) };
    let usage_key = CFString::from(usage_str.to_str().unwrap());
    let usage_value = CFNumber::from(kHIDUsage_LED_CapsLock as i32);

    CFDictionary::from_CFType_pairs(&[(page_key, page_value), (usage_key, usage_value)])
}
