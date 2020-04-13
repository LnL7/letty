use std::ffi::CStr;
use std::io;
use std::ptr;

use crate::hid::element::IOHIDElement;

use core_foundation::array::{CFArrayGetCount, CFArrayGetValueAtIndex};
use core_foundation::base::{TCFType, CFRelease, kCFAllocatorDefault};
use core_foundation::dictionary::{CFDictionary, CFDictionaryRef};
use core_foundation::number::CFNumber;
use core_foundation::string::{CFString, CFStringCreateWithCString, kCFStringEncodingUTF8};
use io_kit_sys::hid::base::{IOHIDDeviceRef, IOHIDValueRef, IOHIDElementRef};
use io_kit_sys::hid::device::*;
use io_kit_sys::hid::keys::{kIOHIDOptionsTypeNone, kIOHIDDeviceUsagePageKey, kIOHIDDeviceUsageKey, kIOHIDProductKey};
use io_kit_sys::hid::usage_tables::{kHIDPage_GenericDesktop, kHIDUsage_GD_Keyboard};
use io_kit_sys::hid::value::{IOHIDValueGetIntegerValue, IOHIDValueCreateWithIntegerValue};
use io_kit_sys::ret::{IOReturn, kIOReturnSuccess};

#[repr(C)]
#[derive(Debug)]
pub struct IOHIDDevice(IOHIDDeviceRef);

impl_TCFType!(IOHIDDevice, IOHIDDeviceRef, IOHIDDeviceGetTypeID);

impl Drop for IOHIDDevice {
    fn drop(&mut self) {
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
            vec.push(element);
        }

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
        let ret = unsafe { IOHIDDeviceGetValue(self.0, element.as_concrete_TypeRef(), &mut value) };

        match ret {
            kIOReturnSuccess => Ok(unsafe { IOHIDValueGetIntegerValue(value) }),
            _ => Err(ret),
        }
    }

    pub fn set_value(&mut self, element: &mut IOHIDElement, new: i64) -> Result<i64, IOReturn> {
        let value = unsafe { IOHIDValueCreateWithIntegerValue(kCFAllocatorDefault, element.as_concrete_TypeRef(), 0, new) };
        let ret = unsafe { IOHIDDeviceSetValue(self.0, element.as_concrete_TypeRef(), value) };

        unsafe { CFRelease(value as _) };

        match ret {
            kIOReturnSuccess => Ok(new),
            _ => Err(ret),
        }
    }

    pub fn toggle_value(&mut self, element: &mut IOHIDElement) -> Result<i64, IOReturn> {
        let current = self.get_value(&element)?;
        // let min = unsafe { IOHIDElementGetLogicalMin(element.0) };
        // let max = unsafe { IOHIDElementGetLogicalMax(element.0) };
        let (min, max) = (0, 1);
        self.set_value(element, if current > min { min } else { max })
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
