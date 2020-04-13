use std::ffi::CStr;

use core_foundation::base::{TCFType, CFRelease};
use core_foundation::dictionary::CFDictionary;
use core_foundation::number::CFNumber;
use core_foundation::string::CFString;
use io_kit_sys::hid::base::IOHIDElementRef;
use io_kit_sys::hid::keys::{kIOHIDElementUsagePageKey, kIOHIDElementUsageKey};
use io_kit_sys::hid::usage_tables::{kHIDUsage_LED_CapsLock, kHIDPage_LEDs};
use io_kit_sys::hid::element::*;

#[repr(C)]
#[derive(Debug)]
pub struct IOHIDElement(IOHIDElementRef);

impl_TCFType!(IOHIDElement, IOHIDElementRef, IOHIDElementGetTypeID);

impl Drop for IOHIDElement {
    fn drop(&mut self) {
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

pub fn capslock_matching_dictionary() -> CFDictionary<CFString, CFNumber> {
    let page_str = unsafe { CStr::from_ptr(kIOHIDElementUsagePageKey) };
    let page_key = CFString::from(page_str.to_str().unwrap());
    let page_value = CFNumber::from(kHIDPage_LEDs as i32);

    let usage_str = unsafe { CStr::from_ptr(kIOHIDElementUsageKey) };
    let usage_key = CFString::from(usage_str.to_str().unwrap());
    let usage_value = CFNumber::from(kHIDUsage_LED_CapsLock as i32);

    CFDictionary::from_CFType_pairs(&[(page_key, page_value), (usage_key, usage_value)])
}
