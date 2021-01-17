use hidapi::*;
use crate::device::Device;
use crate::corsair::CorsairLighting;
use crate::profile_manager::ProfileManager;

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
                Some(Box::new(CorsairLighting::new_commander_pro(api.open(0x1b1c, 0xc10).unwrap())) as Box<_>)
            }
            0xc1a => {
                Some(Box::new(CorsairLighting::new_lighting_node_core(api.open(0x1b1c, 0xc1a).unwrap())) as Box<_>)
            }
            _ => None,
        }
        _ => None
    }).collect();

    for device in devices.iter_mut() {
        device.initialize().unwrap();
    }

    let mut profile_manager = ProfileManager::new(devices, config);

    loop {
        profile_manager.update();
    }
}
