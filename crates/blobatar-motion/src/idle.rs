use crate::ease;
use blobatar_core::traits::Traits;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IdleSeeds {
    pub phase: f64,
    pub bob: f64,
    pub blink: f64,
    pub blink_phase: f64,
    pub saccade: f64,
    pub saccade_phase: f64,
    #[serde(rename = "lookX")]
    pub look_x: f64,
    #[serde(rename = "lookY")]
    pub look_y: f64,
    #[serde(rename = "lookMX")]
    pub look_magnitude_x: f64,
    #[serde(rename = "lookMY")]
    pub look_magnitude_y: f64,
}

impl IdleSeeds {
    pub fn new(traits: &Traits<'_>) -> Self {
        // These ranges are positive, so Rust round agrees with Math.round here.
        let blink = traits.num("motion.blink", 3500.0, 6500.0).round();
        let saccade = traits.num("motion.saccade", 4200.0, 7600.0).round();
        let look_magnitude_x = (traits.num("motion.lookX", 1.0, 2.2) * 100.0).round() / 100.0;
        let look_magnitude_y = (traits.num("motion.lookY", 0.8, 1.7) * 100.0).round() / 100.0;
        Self {
            phase: traits.num("motion.phase", 0.0, 2800.0).round(),
            bob: traits.num("motion.bob", 0.0, 3400.0).round(),
            blink,
            blink_phase: traits.num("motion.blinkPhase", 0.0, blink).round(),
            saccade,
            saccade_phase: traits.num("motion.saccadePhase", 0.0, saccade).round(),
            look_x: look_magnitude_x
                * if traits.boolean("motion.lookXFlip", 0.5) {
                    -1.0
                } else {
                    1.0
                },
            look_y: look_magnitude_y
                * if traits.boolean("motion.lookYFlip", 0.5) {
                    -1.0
                } else {
                    1.0
                },
            look_magnitude_x,
            look_magnitude_y,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct GlanceWrap {
    pub mx: f64,
    pub side: f64,
    pub sy: f64,
    pub rot: f64,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct IdleFrame {
    pub shake: [f64; 2],
    pub breathe: [f64; 2],
    pub bob: f64,
    pub saccade: [f64; 2],
    pub rockp: f64,
    pub blink: f64,
    pub wrap: GlanceWrap,
}

fn cycle(time: f64, phase: f64, period: f64) -> f64 {
    let position = (time + phase) / period;
    position - position.floor()
}

fn alternate(time: f64, phase: f64, period: f64) -> f64 {
    let position = (time + phase) / period;
    let iteration = position.floor();
    let fraction = position - iteration;
    if iteration % 2.0 != 0.0 {
        1.0 - fraction
    } else {
        fraction
    }
}

fn stops<const N: usize>(position: f64, table: &[[f64; N]], column: usize) -> f64 {
    for (index, row) in table.iter().enumerate().rev() {
        if position < row[0] {
            continue;
        }
        let Some(next) = table.get(index + 1) else {
            return row[column];
        };
        let span = next[0] - row[0];
        return if span <= 0.0 {
            row[column]
        } else {
            row[column] + (next[column] - row[column]) * ((position - row[0]) / span)
        };
    }
    table[0][column]
}

const SACCADE: [[f64; 3]; 13] = [
    [0.0, 0.0, 0.0],
    [0.15, 0.0, 0.0],
    [0.165, -0.8, -0.9],
    [0.31, -0.8, -0.9],
    [0.325, 1.0, 0.1],
    [0.47, 1.0, 0.1],
    [0.485, -0.15, 0.85],
    [0.63, -0.15, 0.85],
    [0.645, 0.75, -0.8],
    [0.79, 0.75, -0.8],
    [0.805, -1.0, -0.15],
    [0.985, -1.0, -0.15],
    [1.0, 0.0, 0.0],
];

const WRAP: [[f64; 5]; 13] = [
    [0.0, 0.0, 0.0, 0.0, 0.0],
    [0.15, 0.0, 0.0, 0.0, 0.0],
    [0.165, -0.0176, 0.008, -0.027, 0.648],
    [0.31, -0.0176, 0.008, -0.027, 0.648],
    [0.325, -0.022, -0.01, -0.003, 0.09],
    [0.47, -0.022, -0.01, -0.003, 0.09],
    [0.485, -0.0033, 0.0015, -0.0255, -0.115],
    [0.63, -0.0033, 0.0015, -0.0255, -0.115],
    [0.645, -0.0165, -0.0075, -0.024, -0.54],
    [0.79, -0.0165, -0.0075, -0.024, -0.54],
    [0.805, -0.022, 0.01, -0.0045, 0.135],
    [0.985, -0.022, 0.01, -0.0045, 0.135],
    [1.0, 0.0, 0.0, 0.0, 0.0],
];

const SHAKE: [[f64; 3]; 5] = [
    [0.0, 0.62, -0.34],
    [0.25, -0.7, 0.22],
    [0.5, 0.38, 0.66],
    [0.75, -0.44, -0.6],
    [1.0, 0.62, -0.34],
];

pub fn idle_at(seeds: IdleSeeds, time_ms: f64, amplitude: f64, shake: f64) -> IdleFrame {
    let breathe = ease::in_out(alternate(time_ms, seeds.phase, 2800.0));
    let bob = ease::in_out(alternate(time_ms, seeds.bob, 3400.0));
    let saccade = cycle(time_ms, seeds.saccade_phase, seeds.saccade);
    let shake_phase = cycle(time_ms, 0.0, 112.0);
    let rock_phase = cycle(time_ms, 0.0, 900.0);
    let rockp = if rock_phase < 0.5 {
        1.0 - 2.0 * ease::in_out(rock_phase * 2.0)
    } else {
        -1.0 + 2.0 * ease::in_out(rock_phase * 2.0 - 1.0)
    };
    let blink_phase = cycle(time_ms, seeds.blink_phase, seeds.blink);
    let blink = if blink_phase < 0.972 {
        1.0
    } else if blink_phase < 0.986 {
        1.0 - 0.92 * amplitude * ease::ease_in((blink_phase - 0.972) / 0.014)
    } else {
        1.0 - 0.92 * amplitude * (1.0 - ease::ease_out((blink_phase - 0.986) / 0.014))
    };
    IdleFrame {
        shake: [
            stops(shake_phase, &SHAKE, 1) * shake,
            stops(shake_phase, &SHAKE, 2) * shake,
        ],
        breathe: [
            1.0 + 0.022 * amplitude * breathe,
            1.0 - 0.018 * amplitude * breathe,
        ],
        bob: -1.1 * amplitude * bob,
        saccade: [
            stops(saccade, &SACCADE, 1) * seeds.look_x * amplitude,
            stops(saccade, &SACCADE, 2) * seeds.look_y * amplitude,
        ],
        rockp,
        blink,
        wrap: GlanceWrap {
            mx: stops(saccade, &WRAP, 1) * seeds.look_magnitude_x * amplitude,
            side: stops(saccade, &WRAP, 2) * seeds.look_x * amplitude,
            sy: stops(saccade, &WRAP, 3) * seeds.look_magnitude_y * amplitude,
            rot: stops(saccade, &WRAP, 4) * seeds.look_x * seeds.look_y * amplitude,
        },
    }
}
