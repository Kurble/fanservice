use rand::random;
use serde::Deserialize;

use crate::color::{Color, ColorOp};
use crate::device::Strip;

#[derive(Deserialize, Clone, Debug)]
pub enum Effect {
    Static {
        color: Color,
        #[serde(default)]
        op: ColorOp,
    },
    Gradient {
        from: Color,
        to: Color,
        #[serde(default)]
        op: ColorOp,
    },
    Noise(ColorOp),
    Temperature {
        sensor: usize,
        min_temperature: f32,
        max_temperature: f32,
        min_color: Color,
        max_color: Color,
        #[serde(default)]
        op: ColorOp,
    },
    Wave {
        frames_per_led: usize,
        length: usize,
        colors: Vec<Color>,
        #[serde(default)]
        op: ColorOp,
    },
    Rotation {
        duration: usize,
        colors: Vec<Color>,
        #[serde(default)]
        reverse: bool,
        #[serde(default)]
        op: ColorOp,
    },
    Pattern {
        #[serde(default)]
        frames_per_led: Option<usize>,
        colors: Vec<Color>,
        #[serde(default)]
        reverse: bool,
        #[serde(default)]
        op: ColorOp,
    },
}

impl Effect {
    pub fn is_animated(&self) -> bool {
        match self {
            Effect::Noise(_) => true,
            Effect::Temperature { .. } => true,
            Effect::Wave { .. } => true,
            Effect::Rotation { .. } => true,
            _ => false,
        }
    }

    pub fn apply(
        &self,
        strip: &mut Strip,
        probes: &[Option<f32>],
        indices: &[usize],
        frame: usize,
    ) {
        if let Some(required_len) = indices.iter().cloned().max() {
            if strip.colors.len() < required_len + 1 {
                strip
                    .colors
                    .resize(required_len + 1, Color::Rgb(0.0, 0.0, 0.0));
            }
        }

        match self {
            Effect::Static { color, op } => {
                for &led in indices.iter() {
                    strip.colors[led] = strip.colors[led].blend(color, op);
                }
            }
            Effect::Gradient { from, to, op } => {
                for (i, &led) in indices.iter().enumerate() {
                    let t = i as f32 / (indices.len() - 1) as f32;
                    let c = from.blend(to, &ColorOp::Blend(t));
                    strip.colors[led] = strip.colors[led].blend(&c, op);
                }
            }
            Effect::Noise(op) => {
                for &led in indices.iter() {
                    let noise = Color::Hsv(random(), random(), random());
                    strip.colors[led] = strip.colors[led].blend(&noise, op);
                }
            }
            &Effect::Temperature {
                sensor,
                min_temperature,
                max_temperature,
                ref min_color,
                ref max_color,
                ref op,
            } => {
                let temp = probes
                    .get(sensor)
                    .cloned()
                    .unwrap_or_default()
                    .unwrap_or(max_temperature);
                let range = max_temperature - min_temperature;
                let x = (temp.min(max_temperature) - min_temperature).max(0.0) / range;
                let color = min_color.blend(max_color, &ColorOp::Blend(x));
                for &led in indices.iter() {
                    strip.colors[led] = strip.colors[led].blend(&color, op);
                }
            }
            &Effect::Wave {
                frames_per_led,
                length,
                ref colors,
                ref op,
            } => {
                let progress = (frame / frames_per_led) % (indices.len() + length * 2);
                for i in 0..length {
                    if progress + i >= length {
                        if let Some(&led) = indices.get(progress + i - length) {
                            let x = (i as f32 / length as f32) * colors.len() as f32;
                            let lower = (x.floor() as usize).min(colors.len() - 1);
                            let upper = (x.ceil() as usize).min(colors.len() - 1);
                            let color =
                                colors[lower].blend(&colors[upper], &ColorOp::Blend(x.fract()));
                            strip.colors[led] = strip.colors[led].blend(&color, op);
                        }
                    }
                }
            }
            &Effect::Rotation {
                duration,
                ref colors,
                reverse,
                ref op,
            } => {
                let progress = if reverse {
                    (duration - frame % duration) as f32 / duration as f32
                } else {
                    (frame % duration) as f32 / duration as f32
                };
                for i in 0..indices.len() {
                    let led = indices[i % indices.len()];

                    let x = (progress + (i as f32 / indices.len() as f32)).fract()
                        * (colors.len() as f32 - 1.0);
                    let lower = (x.floor() as usize) % colors.len();
                    let upper = (x.ceil() as usize) % colors.len();
                    let color = colors[lower].blend(&colors[upper], &ColorOp::Blend(x.fract()));

                    strip.colors[led] = strip.colors[led].blend(&color, op);
                }
            }
            &Effect::Pattern {
                frames_per_led,
                ref colors,
                reverse,
                ref op,
            } => {
                let progress = if let Some(frames_per_led) = frames_per_led {
                    (frame / frames_per_led) % colors.len()
                } else {
                    0
                };
                for i in 0..indices.len() {
                    let led = if reverse {
                        indices[i]
                    } else {
                        indices[indices.len() - 1 - i]
                    };
                    let color = &colors[(progress + i) % colors.len()];

                    strip.colors[led] = strip.colors[led].blend(&color, op);
                }
            }
        }
    }
}
