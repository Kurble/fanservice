use hidapi::*;
use crate::device::Device;
use crate::corsair::CorsairLighting;
use crate::profile_manager::ProfileManager;
use std::time::{Instant, Duration};

mod device;
mod corsair;
mod profile_manager;
mod profile;
mod color;
mod effect;

fn main() {
    let config = ron::from_str(std::fs::read_to_string("config.ron").unwrap().as_str()).unwrap();

    let api = HidApi::new().unwrap();

    let mut devices: Vec<Box<dyn Device>> = api.device_list().filter_map(|device| match device.vendor_id() {
        0x1b1c => match device.product_id() {
            0xc10 => {
                Some(Box::new(CorsairLighting::new_commander_pro(device.open_device(&api).unwrap())) as Box<_>)
            }
            0xc1a => {
                Some(Box::new(CorsairLighting::new_lighting_node_core(device.open_device(&api).unwrap())) as Box<_>)
            }
            _ => None,
        }
        _ => None
    }).collect();

    for device in devices.iter_mut() {
        device.initialize().unwrap();
        std::thread::sleep(Duration::from_millis(50));
    }

    let mut profile_manager = ProfileManager::new(devices, config);

    let mut deadline = Instant::now() + Duration::from_millis(30);
    loop {
        profile_manager.update();
        let now = Instant::now();
        if now < deadline {
            let sleep_for = deadline.duration_since(now);
            std::thread::sleep(sleep_for);
            deadline = deadline + Duration::from_millis(30);
        } else {
            let sleep_for = Duration::from_millis(5);
            std::thread::sleep(sleep_for);
            deadline = now + Duration::from_millis(30);
        }
    }
}
