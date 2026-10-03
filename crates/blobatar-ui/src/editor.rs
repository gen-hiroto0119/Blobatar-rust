use std::collections::BTreeMap;

use blobatar_core::{
    Avatar,
    layout::Layout,
    traits::{Override, Traits},
};
use blobatar_export::Settings;

use crate::axes::{self, AXES, Axis, Choice};

#[derive(Clone, Default)]
pub struct EditorState {
    pub settings: Settings,
}

impl EditorState {
    pub fn seed(&self) -> &str {
        if self.settings.name.is_empty() {
            "blobatar"
        } else {
            &self.settings.name
        }
    }

    pub fn avatar(&self) -> Avatar {
        Avatar::new(self.seed(), &self.settings.options)
    }

    pub fn neutral_layout(&self) -> Layout {
        let mut options = self.settings.options.clone();
        options.expression = None;
        Avatar::new(self.seed(), &options).layout
    }

    pub fn reader(&self) -> Traits<'_> {
        Traits::new(
            &self.settings.name,
            self.settings.options.normalize,
            &self.settings.options.traits,
        )
    }

    pub fn pin(&mut self, key: &str, value: f64) {
        if value.is_finite() {
            self.set_pin(key, Some(Override::Fixed(axes::round3(value))));
        }
    }

    pub fn set_pin(&mut self, key: &str, value: Option<Override>) {
        match value {
            Some(value) => {
                if !self.settings.trait_order.iter().any(|entry| entry == key) {
                    self.settings.trait_order.push(key.into());
                }
                self.settings.options.traits.insert(key.into(), value);
            }
            None => {
                self.settings.options.traits.remove(key);
                self.settings.trait_order.retain(|entry| entry != key);
            }
        }
    }

    pub fn toggle_lock(&mut self, key: &str) {
        if self.settings.options.traits.contains_key(key) {
            self.set_pin(key, None);
        } else {
            self.pin(key, self.reader().get(key));
        }
    }

    pub fn chosen(&self, key: &str) -> Vec<f64> {
        match self.settings.options.traits.get(key) {
            Some(Override::Fixed(value)) => vec![*value],
            Some(Override::Candidates(values)) => values.clone(),
            None => Vec::new(),
        }
    }

    pub fn toggle_choice(&mut self, key: &str, order: &[Choice], at: f64) {
        let values = axes::toggle_choice(order, &self.chosen(key), at);
        self.set_pin(key, axes::narrow_pin(values));
    }

    pub fn reset(&mut self) {
        self.settings.options.traits.clear();
        self.settings.trait_order.clear();
    }

    pub fn shuffle_to(&mut self, name: String) {
        self.settings.name = name;
    }

    pub fn applicable_axes(&self) -> Vec<&'static Axis> {
        let layout = self.neutral_layout();
        let shapes = axes::candidates(self.settings.options.traits.get("shape"), &layout.shape);
        AXES.iter().filter(|axis| axis.applies(&shapes)).collect()
    }

    pub fn ordered_pins(&self) -> Vec<(&str, &Override)> {
        let pins = &self.settings.options.traits;
        let mut keys = AXES
            .iter()
            .map(|axis| axis.key)
            .filter(|key| pins.contains_key(*key))
            .collect::<Vec<_>>();
        for key in self.settings.trait_order.iter().chain(pins.keys()) {
            if pins.contains_key(key) && !keys.contains(&key.as_str()) {
                keys.push(key);
            }
        }
        keys.into_iter().map(|key| (key, &pins[key])).collect()
    }

    pub fn apply_traits_json(&mut self, json: &str) -> Result<(), String> {
        let values: serde_json::Map<String, serde_json::Value> =
            serde_json::from_str(json).map_err(|error| error.to_string())?;
        let pins = serde_json::from_value(serde_json::Value::Object(values.clone()))
            .map_err(|error| error.to_string())?;
        let mut next = self.settings.clone();
        next.options.traits = pins;
        next.trait_order = values.into_iter().map(|(key, _)| key).collect();
        next.validate()?;
        self.settings = next;
        Ok(())
    }

    pub fn fit_readback(&self) -> BTreeMap<String, f64> {
        resolved(&self.neutral_layout(), &self.reader())
    }
}

pub fn resolved(layout: &Layout, traits: &Traits<'_>) -> BTreeMap<String, f64> {
    let rx = layout.body.rx;
    let er = layout.eyes[0].rx;
    let asked = (0.075 + traits.get("eye.rx") * 0.03) * rx;
    let fit = if asked > 0.0 { er / asked } else { 1.0 };
    if fit > 0.999 {
        return BTreeMap::new();
    }
    let scale = 0.78 + traits.get("eye.scale") * 0.46;
    let gap = (layout.eyes[1].cx - layout.eyes[0].cx) / 2.0;
    let clearance = gap - er * scale.max(1.0) - rx * 0.03;
    BTreeMap::from([
        ("eye.rx".into(), ((er / rx - 0.075) / 0.03).clamp(0.0, 1.0)),
        (
            "eye.gap".into(),
            ((clearance / rx - 0.1) / 0.14).clamp(0.0, 1.0),
        ),
    ])
}
