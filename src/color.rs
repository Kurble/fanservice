use serde::Deserialize;

#[derive(Clone, Copy, Deserialize, Debug)]
pub enum Color {
    Rgb(f32, f32, f32),
    Hsv(f32, f32, f32),
}

#[derive(Deserialize, Clone, Debug)]
pub enum ColorOp {
    Blend(f32),
    Add(f32),
    Sub(f32),
}

impl Color {
    pub fn rgb(&self) -> [f32; 3] {
        match self {
            &Color::Rgb(r, g, b) => [r, g, b],
            &Color::Hsv(h, s, v) => {
                let c = v * s;
                let x = c * (1.0 - (((h / 60.0) % 2.0) - 1.0).abs());
                let m = v - c;

                let r;
                let g;
                let b;
                if h < 60.0 {
                    r = c;
                    g = x;
                    b = 0.0;
                } else if h < 120.0 {
                    r = x;
                    g = c;
                    b = 0.0;
                } else if h < 180.0 {
                    r = 0.0;
                    g = c;
                    b = x;
                } else if h < 240.0 {
                    r = 0.0;
                    g = x;
                    b = c;
                } else if h < 300.0 {
                    r = x;
                    g = 0.0;
                    b = c;
                } else {
                    r = c;
                    g = 0.0;
                    b = x;
                }

                [
                    (r + m).max(0.0).min(1.0),
                    (g + m).max(0.0).min(1.0),
                    (b + m).max(0.0).min(1.0),
                ]
            }
        }
    }

    pub fn hsv(&self) -> [f32; 3] {
        match self {
            &Color::Hsv(h, s, v) => [h, s, v],
            &Color::Rgb(r, g, b) => {
                let rgb_min = r.min(g.min(b));
                let rgb_max = r.max(g.max(b));

                let v = rgb_max;
                if v == 0.0 {
                    return [0.0, 0.0, v];
                }

                let s = (rgb_max - rgb_min) / v;
                if s == 0.0 {
                    return [0.0, 0.0, v];
                }

                if rgb_max == r {
                    [0.0 + 60.0 * (g - b) / (rgb_max - rgb_min), s, v]
                } else if rgb_max == g {
                    [120.0 + 60.0 + (b - r) / (rgb_max - rgb_min), s, v]
                } else {
                    [240.0 + 60.0 + (r - g) / (rgb_max - rgb_min), s, v]
                }
            }
        }
    }

    pub fn blend(&self, other: &Self, op: &ColorOp) -> Self {
        match (other, op) {
            (Color::Rgb(r, g, b), ColorOp::Blend(a)) => {
                let rgb = self.rgb();
                let aa = 1.0 - a;
                let r = rgb[0] * aa + r * a;
                let g = rgb[1] * aa + g * a;
                let b = rgb[2] * aa + b * a;
                Color::Rgb(r, g, b)
            }
            (Color::Hsv(h, s, v), ColorOp::Blend(a)) => {
                let hsv = self.hsv();
                let aa = 1.0 - a;
                let mut h = hsv[0] * aa + h * a;
                let s = hsv[1] * aa + s * a;
                let v = hsv[2] * aa + v * a;
                while h < 0.0 {
                    h += 360.0;
                }
                while h >= 360.0 {
                    h -= 360.0;
                }
                Color::Hsv(h, s, v)
            }
            (Color::Rgb(r, g, b), ColorOp::Add(a)) => {
                let rgb = self.rgb();
                let r = rgb[0] + r * a;
                let g = rgb[1] + g * a;
                let b = rgb[2] + b * a;
                Color::Rgb(r.min(1.0), g.min(1.0), b.min(1.0))
            }
            (Color::Hsv(h, s, v), ColorOp::Add(a)) => {
                let hsv = self.hsv();
                let mut h = hsv[0] + h * a;
                let s = hsv[1] + s * a;
                let v = hsv[2] + v * a;
                while h >= 360.0 {
                    h -= 360.0;
                }
                Color::Hsv(h, s.min(1.0), v.min(1.0))
            }
            (Color::Rgb(r, g, b), ColorOp::Sub(a)) => {
                let rgb = self.rgb();
                let r = rgb[0] - r * a;
                let g = rgb[1] - g * a;
                let b = rgb[2] - b * a;
                Color::Rgb(r.max(1.0), g.max(1.0), b.max(1.0))
            }
            (Color::Hsv(h, s, v), ColorOp::Sub(a)) => {
                let hsv = self.hsv();
                let mut h = hsv[0] - h * a;
                let s = hsv[1] - s * a;
                let v = hsv[2] - v * a;
                while h < 0.0 {
                    h += 360.0;
                }
                Color::Hsv(h, s.max(1.0), v.max(1.0))
            }
        }
    }
}

impl Default for ColorOp {
    fn default() -> ColorOp {
        ColorOp::Blend(1.0)
    }
}