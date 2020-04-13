use std::error;
use std::thread;
use std::time;

use letty::hid::*;


fn main() -> Result<(), Box<dyn error::Error>> {

    let keyboards = keyboard_matching_dictionary();
    let leds = capslock_matching_dictionary();

    let mut manager = IOHIDManager::new()?;
    manager.set_device_matching(&keyboards);

    let mut vec = Vec::with_capacity(1);

    for mut device in manager.get_devices() {
        if let Ok(elements) = device.get_matching_elements(&leds) {
            for mut element in elements {
                if let Ok(_updated) = device.set_value(&mut element, 0) {
                    vec.push((device, element));
                    break;
                }
            }
        }
    }

    for _ in 0..8 {
        for (device, element) in &mut vec {
            device.set_value(element, 1).unwrap();
            thread::sleep(time::Duration::from_millis(800));

            device.set_value(element, 0).unwrap();
            thread::sleep(time::Duration::from_millis(200));
        }
    }

    Ok(())
}
