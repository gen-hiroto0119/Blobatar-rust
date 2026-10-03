use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub struct Oklch {
    pub l: f64,
    pub c: f64,
    pub h: f64,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Palette {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bg: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub head: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eye: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub struct Ramp {
    pub bg: Oklch,
    pub head: Oklch,
    pub eye: Oklch,
}

fn to_linear(color: Oklch) -> [f64; 3] {
    let radians = color.h * std::f64::consts::PI / 180.0;
    let a = color.c * radians.cos();
    let b = color.c * radians.sin();

    let l = color.l + 0.3963377774 * a + 0.2158037573 * b;
    let m = color.l - 0.1055613458 * a - 0.0638541728 * b;
    let s = color.l - 0.0894841775 * a - 1.291485548 * b;

    let l = l * l * l;
    let m = m * m * m;
    let s = s * s * s;

    [
        4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s,
        -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s,
        -0.0041960863 * l - 0.7034186147 * m + 1.707614701 * s,
    ]
}

fn in_gamut(rgb: [f64; 3]) -> bool {
    rgb.iter().all(|value| (-1e-4..=1.0 + 1e-4).contains(value))
}

fn resolve(color: Oklch) -> [f64; 3] {
    let mut rgb = to_linear(color);
    if !in_gamut(rgb) {
        let mut low = 0.0;
        let mut high = color.c;
        for _ in 0..12 {
            let middle = (low + high) / 2.0;
            if in_gamut(to_linear(Oklch { c: middle, ..color })) {
                low = middle;
            } else {
                high = middle;
            }
        }
        rgb = to_linear(Oklch { c: low, ..color });
    }
    rgb.map(|value| value.clamp(0.0, 1.0))
}

fn luminance(color: Oklch) -> f64 {
    let [red, green, blue] = resolve(color);
    0.2126 * red + 0.7152 * green + 0.0722 * blue
}

fn contrast(a: Oklch, b: Oklch) -> f64 {
    let a = luminance(a);
    let b = luminance(b);
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}

fn ensure_contrast(foreground: Oklch, background: Oklch, minimum: f64) -> Oklch {
    if contrast(foreground, background) >= minimum {
        return foreground;
    }

    let direction = if foreground.l >= background.l {
        1.0
    } else {
        -1.0
    };
    for direction in [direction, -direction] {
        let mut probe = foreground;
        for _ in 0..60 {
            probe.l = (probe.l + direction * 0.02).clamp(0.0, 1.0);
            if contrast(probe, background) >= minimum {
                return probe;
            }
            if probe.l == 0.0 || probe.l == 1.0 {
                break;
            }
        }
    }

    let black = Oklch {
        l: 0.0,
        c: 0.0,
        ..foreground
    };
    let white = Oklch {
        l: 1.0,
        c: 0.0,
        ..foreground
    };
    if contrast(black, background) >= contrast(white, background) {
        black
    } else {
        white
    }
}

fn to_hex(color: Oklch) -> String {
    let rgb = resolve(color);
    let bytes = rgb.map(|value| {
        let srgb = if value <= 0.0031308 {
            12.92 * value
        } else {
            1.055 * value.powf(1.0 / 2.4) - 0.055
        };
        (srgb * 255.0).round() as u8
    });
    format!("#{:02x}{:02x}{:02x}", bytes[0], bytes[1], bytes[2])
}

fn tone(tone: f64) -> Oklch {
    let (l, c) = if tone < 0.2 {
        (0.86, 0.085)
    } else if tone < 0.36 {
        (0.9, 0.028)
    } else if tone < 0.62 {
        (0.73, 0.135)
    } else if tone < 0.8 {
        (0.62, 0.165)
    } else if tone < 0.93 {
        (0.87, 0.16)
    } else if tone < 1.0 {
        (0.34, 0.035)
    } else {
        (0.86, 0.085)
    };
    Oklch { l, c, h: 0.0 }
}

fn dark_surface() -> Oklch {
    Oklch {
        l: 0.145,
        c: 0.0,
        h: 0.0,
    }
}

pub fn ramp(hue: f64, enforce: bool, tone_value: f64) -> Ramp {
    let selected_tone = tone(tone_value);
    let head = ensure_contrast(
        Oklch {
            h: hue,
            ..selected_tone
        },
        dark_surface(),
        1.5,
    );
    let mut ramp = Ramp {
        bg: Oklch {
            l: 0.965,
            c: 0.01,
            h: hue,
        },
        head,
        eye: if head.l >= 0.5 {
            Oklch {
                l: 0.17,
                c: 0.02,
                h: hue,
            }
        } else {
            Oklch {
                l: 0.97,
                c: 0.012,
                h: hue,
            }
        },
    };

    if enforce {
        ramp.head = ensure_contrast(ramp.head, ramp.bg, 1.25);
        ramp.eye = ensure_contrast(ramp.eye, ramp.head, 4.5);
    }
    ramp
}

pub fn palette(hue: f64, enforce: bool, tone_value: f64) -> Palette {
    let ramp = ramp(hue, enforce, tone_value);
    Palette {
        bg: Some(to_hex(ramp.bg)),
        head: Some(to_hex(ramp.head)),
        eye: Some(to_hex(ramp.eye)),
    }
}
