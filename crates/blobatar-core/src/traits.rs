use crate::hash::{seed_state, stream};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Override {
    Fixed(f64),
    Candidates(Vec<f64>),
}

pub type TraitOverrides = BTreeMap<String, Override>;

pub struct Traits<'a> {
    state: u32,
    overrides: &'a TraitOverrides,
}

impl<'a> Traits<'a> {
    pub fn new(seed: &str, normalize: bool, overrides: &'a TraitOverrides) -> Self {
        Self {
            state: seed_state(seed, normalize),
            overrides,
        }
    }

    pub fn get(&self, key: &str) -> f64 {
        let raw = stream(self.state, key);
        let fixed = match self.overrides.get(key) {
            Some(Override::Fixed(v)) => Some(*v),
            Some(Override::Candidates(v)) => {
                v.get((raw * v.len() as f64).floor() as usize).copied()
            }
            None => None,
        };
        match fixed {
            Some(v) if v > 0.0 => {
                if v < 1.0 {
                    v
                } else {
                    0.999999
                }
            }
            Some(_) => 0.0,
            None => raw,
        }
    }

    pub fn num(&self, key: &str, min: f64, max: f64) -> f64 {
        min + self.get(key) * (max - min)
    }

    pub fn int(&self, key: &str, min: i32, max: i32) -> i32 {
        min + (self.get(key) * f64::from(max - min + 1)).floor() as i32
    }

    pub fn pick<T: Clone>(&self, key: &str, options: &[T]) -> Option<T> {
        options
            .get((self.get(key) * options.len() as f64).floor() as usize)
            .cloned()
    }

    pub fn boolean(&self, key: &str, probability: f64) -> bool {
        self.get(key) < probability
    }

    pub fn jitter(&self, key: &str, amount: f64) -> f64 {
        (self.get(key) * 2.0 - 1.0) * amount
    }
}
