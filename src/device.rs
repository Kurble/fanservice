use anyhow::Result;
use serde::Deserialize;

use crate::color::Color;

#[derive(Clone, Deserialize, Debug)]
pub enum Fan {
    Pwm(f32),
    Rpm(u16),
    Curve(usize, [TempRpm; 6]),
}

#[derive(Clone, Deserialize, Debug)]
pub struct TempRpm {
    pub temp: f32,
    pub rpm: u16,
}

#[derive(Clone)]
pub struct Strip {
    pub colors: Vec<Color>,
}

pub trait Device {
    fn initialize(&mut self) -> Result<()>;

    fn is_led_only(&self) -> bool;

    fn name(&self) -> &str;

    fn fans(&mut self) -> &mut [Fan];

    fn strips(&mut self) -> &mut [Strip];

    fn probes(&self) -> &[Option<f32>];

    fn report_status(&self);

    fn update(&mut self) -> Result<()>;
}
