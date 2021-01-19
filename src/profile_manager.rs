use std::time::{Duration, Instant};

use crate::device::Device;
use crate::profile::{ColorProfile, Config, FanProfile, Trigger};

pub struct ProfileManager {
    devices: Vec<Box<dyn Device>>,
    color_profiles: Vec<ColorProfile>,
    color_profile_current: Option<usize>,
    fan_profiles: Vec<FanProfile>,
    fan_profile_current: Option<usize>,
    frame: usize,
    probes: Vec<Option<f32>>,
    last_update: Instant,
    last_log: Instant,
}

impl ProfileManager {
    pub fn new(devices: Vec<Box<dyn Device>>, mut config: Config) -> Self {
        for p in config.color_profiles.iter_mut() {
            p.initialize();
        }

        Self {
            devices,
            color_profiles: config.color_profiles,
            color_profile_current: None,
            fan_profiles: config.fan_profiles,
            fan_profile_current: None,
            frame: 0,
            probes: vec![],
            last_update: Instant::now(),
            last_log: Instant::now(),
        }
    }

    pub fn update(&mut self) {
        self.frame += 1;

        // check for a new color profile
        let mut next_color_profile = self.color_profile_current.or(Some(0));
        for (i, p) in self.color_profiles.iter().enumerate() {
            if !p.transient {
                for t in p.triggers.iter() {
                    if self.check_trigger(t) {
                        next_color_profile.replace(i);
                    }
                }
            }
        }

        if next_color_profile != self.color_profile_current
            || self.color_profiles[next_color_profile.unwrap()].is_animated()
        {
            if next_color_profile != self.color_profile_current {
                log::info!(
                    "Activating color profile: {}",
                    self.color_profiles[next_color_profile.unwrap()].name
                );
            }
            self.color_profile_current = next_color_profile;

            // apply color profile
            for config in self.color_profiles[next_color_profile.unwrap()]
                .strip_profiles
                .iter_mut()
            {
                for device in self.devices.iter_mut() {
                    if device.name() == config.device.as_str() {
                        if let Some(strip) = device.strips().get_mut(config.channel) {
                            config.apply(strip, &self.probes, self.frame);
                        }
                    }
                }
            }
        }

        // check if transient profiles should be applied
        for p in self.color_profiles.iter() {
            if p.transient && p.triggers.iter().any(|t| self.check_trigger(t)) {
                // apply transient color profile
                for config in p.strip_profiles.iter() {
                    for device in self.devices.iter_mut() {
                        if device.name() == config.device.as_str() {
                            if let Some(strip) = device.strips().get_mut(config.channel) {
                                config.apply(strip, &self.probes, self.frame);
                            }
                        }
                    }
                }
            }
        }

        // check for a new fan profile
        let mut next_fan_profile = self.fan_profile_current.or(Some(0));
        for (i, p) in self.fan_profiles.iter().enumerate() {
            for t in p.triggers.iter() {
                if self.check_trigger(t) {
                    next_fan_profile.replace(i);
                }
            }
        }

        if next_fan_profile != self.fan_profile_current {
            log::info!(
                "Activating fan profile: {}",
                self.fan_profiles[next_fan_profile.unwrap()].name
            );
            self.fan_profile_current = next_fan_profile;

            // apply fan profile
            for config in self.fan_profiles[next_fan_profile.unwrap()].fans.iter() {
                for device in self.devices.iter_mut() {
                    if device.name() == config.device.as_str() {
                        if let Some(fan) = device.fans().get_mut(config.channel) {
                            *fan = config.config.clone();
                        }
                    }
                }
            }
        }

        // reset all devices if the loop is somehow taking longer than expected (did the system sleep?)
        let elapsed = std::mem::replace(&mut self.last_update, Instant::now()).elapsed();
        if elapsed.as_secs_f32() > 2.0 {
            log::warn!("Recovering from system sleep");
            std::thread::sleep(std::time::Duration::from_secs(5));
            for device in self.devices.iter_mut() {
                if let Err(e) = device.initialize() {
                    log::error!("Unable to re-initialize {}: {}", device.name(), e);
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
            self.last_update = Instant::now();
        }

        self.probes.clear();
        for device in self.devices.iter_mut() {
            if let Err(e) = device.update() {
                log::error!("Unable to update {}: {}", device.name(), e);
            }
            self.probes.extend_from_slice(device.probes());
        }

        if self.last_log.elapsed() > Duration::from_secs(10) {
            self.last_log = Instant::now();
            for device in self.devices.iter() {
                if !device.is_led_only() {
                    device.report_status();
                }
            }
        }
    }

    fn check_trigger(&self, trigger: &Trigger) -> bool {
        match trigger {
            &Trigger::SensorAbove {
                sensor,
                temperature,
            } => self
                .probes
                .get(sensor)
                .unwrap_or(&None)
                .map(|val| val > temperature)
                .unwrap_or_default(),
            &Trigger::SensorBelow {
                sensor,
                temperature,
            } => self
                .probes
                .get(sensor)
                .unwrap_or(&None)
                .map(|val| val < temperature)
                .unwrap_or_default(),
            &Trigger::ProcessRunning { name: _ } => false,
        }
    }
}
