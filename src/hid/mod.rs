#![allow(non_upper_case_globals)]

pub mod manager;
pub mod device;
pub mod element;

pub use device::{IOHIDDevice, keyboard_matching_dictionary};
pub use element::{IOHIDElement, capslock_matching_dictionary};
pub use manager::IOHIDManager;
