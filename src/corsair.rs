#![allow(unused)]

use std::cell::Cell;
use std::time::Instant;

use anyhow::*;
use hidapi::{HidDevice, HidResult};

use crate::color::Color;
use crate::device::{Device, Fan, Strip};
use std::ops::AddAssign;

pub struct CorsairLighting {
    name: String,
    device: HidDevice,
    fans: Vec<Fan>,
    fans_dirty: bool,
    strips: Vec<Strip>,
    strips_dirty: bool,
    probes: Vec<Option<f32>>,
    fan_modes: Vec<FanMode>,
    rpms: Vec<u16>,
    next_sample: usize,
    backlog: Cell<usize>,
}

#[derive(Clone, Debug)]
enum FanMode {
    Off,
    Dc,
    Pwm,
}

const REPORT_LENGTH: usize = 64;
const RESPONSE_LENGTH: usize = 16;

const CMD_GET_FIRMWARE: u8 = 0x02;
const CMD_GET_BOOTLOADER: u8 = 0x06;
const CMD_GET_TEMP_CONFIG: u8 = 0x10;
const CMD_GET_TEMP: u8 = 0x11;
const CMD_GET_VOLTS: u8 = 0x12;
const CMD_GET_FAN_MODES: u8 = 0x20;
const CMD_GET_FAN_RPM: u8 = 0x21;
const CMD_SET_FAN_DUTY: u8 = 0x23;
const CMD_SET_FAN_PROFILE: u8 = 0x25;

const CMD_RESET_LED_CHANNEL: u8 = 0x37;
const CMD_BEGIN_LED_EFFECT: u8 = 0x34;
const CMD_SET_LED_CHANNEL_STATE: u8 = 0x38;
const CMD_LED_EFFECT: u8 = 0x35;
const CMD_LED_COMMIT: u8 = 0x33;
const CMD_LED_DIRECT: u8 = 0x32;

const LED_PORT_STATE_HARDWARE: u8 = 0x01;
const LED_PORT_STATE_SOFTWARE: u8 = 0x02;
const LED_SPEED_FAST: u8 = 0x00;
const LED_SPEED_MEDIUM: u8 = 0x01;
const LED_SPEED_SLOW: u8 = 0x02;

const LED_DIRECTION_FORWARD: u8 = 0x01;
const LED_DIRECTION_BACKWARD: u8 = 0x00;

const FAN_MODE_DISCONNECTED: u8 = 0x00;
const FAN_MODE_DC: u8 = 0x01;
const FAN_MODE_PWM: u8 = 0x02;

impl CorsairLighting {
    pub fn new_commander_pro(device: HidDevice) -> Self {
        Self {
            name: String::from("Commander PRO"),
            device,
            fans: vec![Fan::Pwm(0.25); 6],
            fans_dirty: true,
            strips: vec![Strip { colors: Vec::new() }, Strip { colors: Vec::new() }],
            strips_dirty: true,
            probes: vec![None; 4],
            fan_modes: vec![FanMode::Off; 6],
            rpms: vec![0; 6],
            next_sample: 0,
            backlog: Cell::new(0),
        }
    }

    pub fn new_lighting_node_core(device: HidDevice) -> Self {
        Self {
            name: String::from("Lighting Node CORE"),
            device,
            fans: vec![],
            fans_dirty: true,
            strips: vec![Strip { colors: Vec::new() }],
            strips_dirty: true,
            probes: vec![],
            fan_modes: vec![],
            rpms: vec![],
            next_sample: 0,
            backlog: Cell::new(0),
        }
    }

    fn get_temp(&self, index: usize) -> HidResult<f32> {
        let res = self.request(CMD_GET_TEMP, &[index as u8])?;
        Ok(res[1] as f32 * 2.56 + res[2] as f32 * 0.01)
    }

    fn get_rpm(&self, index: usize) -> HidResult<u16> {
        let res = self.request(CMD_GET_FAN_RPM, &[index as u8])?;
        Ok(res[1] as u16 * 256 + res[2] as u16)
    }

    fn send(&self, command: u8, data: &[u8]) -> HidResult<()> {
        let mut buf = [0u8; REPORT_LENGTH];
        buf[1] = command;
        buf[2..2 + data.len()].copy_from_slice(data);
        self.device.write(&buf)?;
        Ok(())
    }

    fn request(&self, command: u8, data: &[u8]) -> HidResult<[u8; RESPONSE_LENGTH]> {
        let mut buf = [0u8; REPORT_LENGTH];
        buf[1] = command;
        buf[2..2 + data.len()].copy_from_slice(data);
        self.device.write(&buf)?;

        let mut response = [0u8; RESPONSE_LENGTH];
        if self.device.read(&mut response)? > 0 {
            Ok(response)
        } else {
            Ok(response)
        }
    }

    fn update_fans(&self) -> HidResult<()> {
        for (i, fan) in self.fans.iter().enumerate() {
            match fan {
                Fan::Pwm(duty) => {
                    self.send(
                        CMD_SET_FAN_DUTY,
                        &[i as u8, (duty * 100.0).min(100.0).max(0.0) as u8],
                    )?;
                }
                Fan::Rpm(rpm) => {
                    let rpm_lo = rpm & 0xff;
                    let rpm_hi = (rpm & 0xff00) >> 8;

                    let mut buf = [0; 25];
                    buf[0] = i as u8;
                    buf[1] = 0; // sensor 1, doesn't matter much
                    for j in 0..6 {
                        buf[2 + j * 2] = 0;
                        buf[3 + j * 2] = 0;
                        buf[14 + j * 2] = rpm_hi as u8;
                        buf[15 + j * 2] = rpm_lo as u8;
                    }

                    self.send(CMD_SET_FAN_PROFILE, &buf)?;
                }
                Fan::Curve(sensor, curve) => {
                    let mut buf = [0; 32];
                    buf[0] = i as u8;
                    buf[1] = *sensor as u8;
                    for j in 0..6 {
                        let rpm_lo = curve[j].rpm & 0xff;
                        let rpm_hi = (curve[j].rpm & 0xff00) >> 8;

                        let temp_lo = (curve[j].temp * 100.0) as u16 & 0xff;
                        let temp_hi = ((curve[j].temp * 100.0) as u16 & 0xff00) >> 8;

                        buf[2 + j * 2] = temp_hi as u8;
                        buf[3 + j * 2] = temp_lo as u8;
                        buf[14 + j * 2] = rpm_hi as u8;
                        buf[15 + j * 2] = rpm_lo as u8;
                    }

                    self.send(CMD_SET_FAN_PROFILE, &buf)?;
                }
            }
        }

        Ok(())
    }

    fn update_strips(&self) -> HidResult<()> {
        for (channel, strip) in self.strips.iter().enumerate() {
            let channel = channel as u8;

            if strip.colors.is_empty() {
                continue;
            }

            self.send(
                CMD_SET_LED_CHANNEL_STATE,
                &[channel, LED_PORT_STATE_SOFTWARE],
            )?;

            let mut start_led = 0;
            for chunk in strip.colors.chunks(50) {
                let mut buf = [0; 54];
                buf[0] = channel;
                buf[1] = start_led;
                buf[2] = chunk.len() as u8;

                for i in 0..3 {
                    buf[3] = i as u8;
                    for j in 0..chunk.len() {
                        buf[4 + j] = (chunk[j].rgb()[i] * 255.0) as u8;
                    }
                    self.send(CMD_LED_DIRECT, &buf)?;
                }

                start_led += chunk.len() as u8;
            }

            self.send(CMD_LED_COMMIT, &[channel])?;
        }

        Ok(())
    }
}

impl Device for CorsairLighting {
    fn initialize(&mut self) -> Result<()> {
        let [ma, mi, p, ..] = self.request(CMD_GET_FIRMWARE, &[])?;
        let [bma, bmi, ..] = self.request(CMD_GET_BOOTLOADER, &[])?;

        if !self.probes.is_empty() {
            let probes_config = self.request(CMD_GET_TEMP_CONFIG, &[])?;
            for i in 0..self.probes.len() {
                if probes_config[i + 1] > 0 {
                    let temp = self.get_temp(i)?;
                    self.probes[i].replace(temp);
                }
            }
        }

        if !self.fans.is_empty() {
            let fan_modes = self.request(CMD_GET_FAN_MODES, &[])?;
            for i in 0..self.fans.len() {
                self.fan_modes[i] = match fan_modes[i + 1] {
                    FAN_MODE_DISCONNECTED => FanMode::Off,
                    FAN_MODE_DC => FanMode::Dc,
                    FAN_MODE_PWM => FanMode::Pwm,
                    _ => FanMode::Off,
                }
            }
        }

        for i in 0..self.strips.len() as u8 {
            self.send(CMD_RESET_LED_CHANNEL, &[i])?;
            self.send(CMD_BEGIN_LED_EFFECT, &[i])?;
            self.send(CMD_SET_LED_CHANNEL_STATE, &[i, LED_PORT_STATE_HARDWARE])?;
            self.send(
                CMD_LED_EFFECT,
                &[
                    i,
                    0,
                    204,
                    0x06,
                    LED_SPEED_MEDIUM,
                    LED_DIRECTION_FORWARD,
                    0x01,
                    0xff,
                ],
            )?;
            self.send(CMD_LED_COMMIT, &[i])?;
        }

        log::info!("{}: \n FW version {}.{}.{} \n Bootloader version {}.{} \n Temperature: {:?} \n Fan modes: {:?}", self.name, ma, mi, p, bma, bmi, self.probes, self.fan_modes);

        self.fans_dirty = true;
        self.strips_dirty = true;

        Ok(())
    }

    fn is_led_only(&self) -> bool {
        self.fans.is_empty() && self.probes.is_empty()
    }

    fn name(&self) -> &str {
        self.name.as_str()
    }

    fn fans(&mut self) -> &mut [Fan] {
        self.fans_dirty = true;
        &mut self.fans
    }

    fn strips(&mut self) -> &mut [Strip] {
        self.strips_dirty = true;
        &mut self.strips
    }

    fn probes(&self) -> &[Option<f32>] {
        &self.probes
    }

    fn report_status(&self) {
        log::info!(
            target: format!("{} status", self.name).as_str(),
            "temperatures = {:?}, fan speeds = {:?}",
            self.probes,
            self.rpms
        )
    }

    fn update(&mut self) -> Result<()> {
        let mut current_sample = 0;

        // flush the input stream
        self.device.set_blocking_mode(false);
        while self.device.read(&mut [0; RESPONSE_LENGTH])? > 0 {}
        self.device.set_blocking_mode(true);

        for i in 0..self.probes.len() {
            if self.probes[i].is_some() {
                if current_sample == self.next_sample {
                    let temp = self.get_temp(i)?;
                    if temp > 0.0 {
                        self.probes[i].replace(temp);
                    } else {
                        self.probes[i].replace(100.0);
                    }
                }
                current_sample += 1;
            }
        }

        for i in 0..self.fans.len() {
            match &self.fan_modes[i] {
                FanMode::Pwm | FanMode::Dc => {
                    if current_sample == self.next_sample {
                        self.rpms[i] = self.get_rpm(i)?;
                    }
                    current_sample += 1;
                }
                FanMode::Off => self.rpms[i] = 0,
            }
        }

        if current_sample > 0 {
            self.next_sample = (self.next_sample + 1) % current_sample;
        }

        if self.fans_dirty {
            self.update_fans()?;
            self.fans_dirty = false;
        }

        if self.strips_dirty {
            self.update_strips()?;
            self.strips_dirty = false;
        }

        Ok(())
    }
}
