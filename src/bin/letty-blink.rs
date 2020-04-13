use std::error;
use std::thread;
use std::time;

use letty::hid::*;


fn main() -> Result<(), Box<dyn error::Error>> {

    let keyboards = keyboard_matching_dictionary();
    let leds = capslock_matching_dictionary();

    for _ in 0..4 {
        let mut manager = IOHIDManager::new()?;
        manager.set_device_matching(&keyboards);

        let mut devices = manager.get_devices();
        for device in &mut devices {
            // device.show();

            if let Some(name) = device.get_name() {
                eprintln!("name {:?}", name);
            }

            if let Ok(mut elements) = device.get_matching_elements(&leds) {
                for element in &mut elements {
                    if let Ok(current) = device.get_value(&element) {
                        if let Ok(updated) = device.toggle_value(element) {
                            eprintln!("value {} -> {}", current, updated);
                        }
                    }
                }
            }
        }

        thread::sleep(time::Duration::from_millis(400));
    }

    Ok(())
}
