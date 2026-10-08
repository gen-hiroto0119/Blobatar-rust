#[cfg(not(target_family = "wasm"))]
use std::{io::Write, path::Path};

use blobatar_core::{Avatar, Generation, Options, traits::Override};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Motion {
    Off,
    #[default]
    Hover,
    Always,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    pub schema_version: u32,
    pub generation: u8,
    pub name: String,
    pub options: Options,
    pub motion: Motion,
    pub reduced_motion: bool,
    #[serde(default)]
    pub trait_order: Vec<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            schema_version: 1,
            generation: 2,
            name: "alain00".into(),
            options: Options::default(),
            motion: Motion::Hover,
            reduced_motion: false,
            trait_order: Vec::new(),
        }
    }
}

impl Settings {
    pub fn generation(&self) -> Generation {
        if self.generation == 1 {
            Generation::One
        } else {
            Generation::Two
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != 1 {
            return Err("Unsupported settings schema".into());
        }
        if !matches!(self.generation, 1 | 2) {
            return Err("Generation must be 1 or 2".into());
        }
        if self.name.len() > 16_384
            || self
                .options
                .title
                .as_ref()
                .is_some_and(|title| title.len() > 16_384)
        {
            return Err("Name or title is too long".into());
        }
        for value in [self.options.size, self.options.hue, self.options.tone]
            .into_iter()
            .flatten()
        {
            if !value.is_finite() {
                return Err("Options must contain finite numbers".into());
            }
        }
        if self.options.size.is_some_and(|size| size <= 0.0) {
            return Err("Size must be positive".into());
        }
        if self.options.traits.len() > 256 || self.trait_order.len() > 256 {
            return Err("Too many traits".into());
        }
        for (key, value) in &self.options.traits {
            let values = match value {
                Override::Fixed(value) => std::slice::from_ref(value),
                Override::Candidates(values) => values.as_slice(),
            };
            if key.len() > 256
                || values.len() > 256
                || values.iter().any(|value| !value.is_finite())
            {
                return Err(format!("Invalid trait: {key}"));
            }
        }
        Ok(())
    }

    pub fn from_json(json: &str) -> Result<Self, String> {
        if json.len() > 1_048_576 {
            return Err("Settings exceed 1 MiB".into());
        }
        let settings: Self = serde_json::from_str(json).map_err(|error| error.to_string())?;
        settings.validate()?;
        Ok(settings)
    }

    pub fn to_json(&self) -> Result<String, String> {
        self.validate()?;
        serde_json::to_string_pretty(self).map_err(|error| error.to_string())
    }

    pub fn svg(&self) -> Result<String, String> {
        self.validate()?;
        let seed = if self.name.is_empty() {
            "blobatar"
        } else {
            &self.name
        };
        Ok(Avatar::with_generation(seed, &self.options, self.generation()).svg(&self.options))
    }

    pub fn png(&self, pixels: u32) -> Result<Vec<u8>, String> {
        if !(1..=4096).contains(&pixels) {
            return Err("PNG size must be between 1 and 4096 pixels".into());
        }
        let svg = self.svg()?;
        let tree = resvg::usvg::Tree::from_str(&svg, &resvg::usvg::Options::default())
            .map_err(|error| error.to_string())?;
        let mut pixmap =
            resvg::tiny_skia::Pixmap::new(pixels, pixels).ok_or("Could not allocate PNG")?;
        let scale = resvg::tiny_skia::Transform::from_scale(
            pixels as f32 / tree.size().width(),
            pixels as f32 / tree.size().height(),
        );
        resvg::render(&tree, scale, &mut pixmap.as_mut());
        pixmap.encode_png().map_err(|error| error.to_string())
    }
}

/// Write in the destination directory, then atomically replace the destination.
#[cfg(not(target_family = "wasm"))]
pub fn save_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut file = tempfile::NamedTempFile::new_in(parent)?;
    file.write_all(bytes)?;
    file.as_file().sync_all()?;
    file.persist(path).map_err(|error| error.error)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use blobatar_core::{Background, Expression};

    #[test]
    fn settings_roundtrip_preserves_all_expressions_and_overrides() {
        for expression in Expression::ALL {
            let mut settings = Settings {
                name: "ひろと 🦀".into(),
                motion: Motion::Always,
                reduced_motion: true,
                ..Settings::default()
            };
            settings.options.expression = Some(expression);
            settings.options.background = Some(Background::Kind("squircle".into()));
            settings
                .options
                .traits
                .insert("shape".into(), Override::Candidates(vec![0.11, 0.965]));
            settings.trait_order.push("shape".into());
            let restored = Settings::from_json(&settings.to_json().unwrap()).unwrap();
            assert_eq!(restored.to_json().unwrap(), settings.to_json().unwrap());
            assert_eq!(restored.svg().unwrap(), settings.svg().unwrap());
        }
    }

    #[test]
    fn generation_one_roundtrips_and_exports_its_frozen_geometry() {
        let mut settings = Settings {
            generation: 1,
            ..Settings::default()
        };
        settings
            .options
            .traits
            .insert("shape".into(), Override::Fixed(0.65));
        settings.options.expression = Some(Expression::Thinking);
        let restored = Settings::from_json(&settings.to_json().unwrap()).unwrap();
        assert_eq!(restored.generation, 1);
        let expected = Avatar::with_generation(&settings.name, &settings.options, Generation::One);
        assert_eq!(expected.layout.shape, "boxy");
        assert_eq!(restored.svg().unwrap(), expected.svg(&settings.options));
        assert_ne!(
            restored.svg().unwrap(),
            Avatar::new(&settings.name, &settings.options).svg(&settings.options)
        );
        let image = resvg::tiny_skia::Pixmap::decode_png(&restored.png(512).unwrap()).unwrap();
        assert_eq!((image.width(), image.height()), (512, 512));
        assert_eq!(image.pixel(0, 0).unwrap().alpha(), 0);
    }

    #[test]
    fn png_defaults_to_transparency_and_requested_dimensions() {
        let settings = Settings::default();
        let png = settings.png(512).unwrap();
        let image = resvg::tiny_skia::Pixmap::decode_png(&png).unwrap();
        assert_eq!((image.width(), image.height()), (512, 512));
        assert_eq!(image.pixel(0, 0).unwrap().alpha(), 0);
        assert!(image.pixels().iter().any(|pixel| pixel.alpha() == 255));
        assert!(settings.png(0).is_err());
        assert!(settings.png(4097).is_err());
    }

    #[test]
    fn atomic_save_replaces_complete_files_and_reports_failure() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("avatar.svg");
        save_atomic(&path, b"first").unwrap();
        save_atomic(&path, b"second").unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"second");
        assert!(save_atomic(&directory.path().join("missing/avatar.svg"), b"bad").is_err());
        assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 1);
    }

    #[test]
    fn unsupported_or_invalid_settings_are_rejected() {
        let mut settings = Settings {
            generation: 3,
            ..Settings::default()
        };
        assert!(settings.validate().is_err());
        settings.generation = 2;
        settings.options.hue = Some(f64::NAN);
        assert!(settings.to_json().is_err());
        assert!(Settings::from_json("{}").is_err());
    }
}
