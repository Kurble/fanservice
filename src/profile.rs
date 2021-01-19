use serde::{Deserialize, Deserializer};

use crate::device::{Fan, Strip};
use crate::effect::Effect;

#[derive(Deserialize, Clone, Debug)]
pub struct Config {
    pub color_profiles: Vec<ColorProfile>,
    pub fan_profiles: Vec<FanProfile>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct ColorProfile {
    pub name: String,
    pub triggers: Vec<Trigger>,
    #[serde(default)]
    pub transient: bool,
    pub strip_profiles: Vec<StripConfig>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct FanProfile {
    pub name: String,
    pub triggers: Vec<Trigger>,
    pub fans: Vec<FanConfig>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct FanConfig {
    pub device: String,
    pub channel: usize,
    pub config: Fan,
}

#[derive(Deserialize, Clone, Debug)]
pub struct StripConfig {
    pub device: String,
    pub channel: usize,
    pub indices: Indices,
    #[serde(deserialize_with="deserialize_from_maybe_file")]
    pub effect: Effect,
}

#[derive(Deserialize, Clone, Debug)]
pub enum Indices {
    Range(usize, usize),
    Ranges(Vec<(usize, usize)>),
    Specific(Vec<usize>),
}

#[derive(Deserialize, Clone, Debug)]
pub enum Trigger {
    SensorAbove {
        sensor: usize,
        temperature: f32,
    },
    SensorBelow {
        sensor: usize,
        temperature: f32,
    },
    ProcessRunning {
        name: String,
    },
}

impl ColorProfile {
    pub fn initialize(&mut self) {
        for p in self.strip_profiles.iter_mut() {
            p.indices.initialize();
        }
    }

    pub fn is_animated(&self) -> bool {
        self.strip_profiles.iter().any(|strip| strip.effect.is_animated())
    }
}

impl Indices {
    pub fn initialize(&mut self) {
        let indices = match self.clone() {
            Indices::Range(from, to) => {
                let mut result = Vec::with_capacity(to - from);
                for i in from..to {
                    result.push(i);
                }
                result
            },
            Indices::Ranges(ranges) => {
                let mut result = Vec::new();
                for r in ranges {
                    for i in r.0..r.1 {
                        result.push(i);
                    }
                }
                result
            }
            Indices::Specific(indices) => indices
        };
        *self = Indices::Specific(indices);
    }

    pub fn indices(&self) -> &[usize] {
        match self {
            Indices::Specific(indices) => indices.as_slice(),
            _ => unreachable!(),
        }
    }
}

impl StripConfig {
    pub fn apply(&self, strip: &mut Strip, probes: &[Option<f32>], frame: usize) {
        self.effect.apply(strip, probes, self.indices.indices(), frame);
    }
}

fn deserialize_from_maybe_file<'de, T: for<'a> Deserialize<'a>, D>(de: D) -> Result<T, D::Error> where D: Deserializer<'de> {
    let path = String::deserialize(de)?;
    let result: T = ron::from_str(std::fs::read_to_string(path).unwrap().as_str()).unwrap();
    Ok(result)
}