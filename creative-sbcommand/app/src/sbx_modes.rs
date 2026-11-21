//! SBX / Scout / Direct mode semantics, taken from the shipped G6 product data.
//!
//! Data source: `reference/g6-product-data.json` -> `profiles[*].settings.speaker`
//! (and `.headphone`, which carries identical effect values for every profile).
//! Each profile stores, per effect, an `Enable` flag and a `Level` in 0..1, plus
//! `Scout.Enable` and `SBXMaster.Enable`.
//!
//! Facts this module encodes, and where they come from:
//!
//! * A profile is not just five levels — the `Enable` flags differ per profile.
//!   `Gaming` has Bass, Smart Volume and Dialog+ disabled while `Movie` has
//!   Smart Volume on at 0.40, so applying a profile has to set both.
//! * `SVM.Mode` is 0 in all 23 profiles, which matches the APO result that
//!   `CPlugAGCCompressor` has no mode curves (see `reference/g6-svm-exact-port.md`).
//! * `Bass.XOver` is 80 Hz in every profile.
//! * `SBXMaster.Enable` is on and `Scout.Enable` is off in every stored profile:
//!   the SBX/Scout exclusion is a runtime toggle, not profile data.
//! * Scout switches SBX off: `ApoScoutFeature.Enable()` saves and closes the SBX
//!   master, GraphicEQ, EAX and Reality3D, and the USB Scout command lives in a
//!   different scope from SBX (see `reference/g6-scout-geq.md`).
//! * Long-pressing Scout enters Direct, which is a full bypass.
//!
//! Scout itself is a device-side feature; the host APO has no Scout DSP. This
//! module therefore only implements the host-side exclusion/bypass semantics and
//! says so instead of pretending the Scout processing exists.

use serde_json::Value;
use std::error::Error;

/// Embedded product data, the same file the EQ presets come from.
pub const PRODUCT_JSON: &str = include_str!("../../reference/g6-product-data.json");

/// One effect as a profile stores it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProfileEffect {
    pub enabled: bool,
    /// 0..100, the shipped 0..1 level rounded to the nearest percent.
    pub level: u8,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Profile {
    pub name: String,
    pub order: u32,
    pub surround: ProfileEffect,
    pub crystalizer: ProfileEffect,
    pub bass: ProfileEffect,
    pub bass_crossover: u32,
    pub smart_volume: ProfileEffect,
    pub dialog_plus: ProfileEffect,
    pub scout: bool,
    pub sbx_master: bool,
}

/// The three mutually exclusive states of the product.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// Long-press Scout: every effect bypassed.
    Direct,
    /// SBX with one of the factory profiles.
    Sbx,
    /// Scout on; SBX off. The Scout DSP itself is not available on the host.
    Scout,
}

impl Mode {
    pub const ALL: [Mode; 3] = [Mode::Direct, Mode::Sbx, Mode::Scout];

    pub fn label(self) -> &'static str {
        match self {
            Mode::Direct => "Direct",
            Mode::Sbx => "SBX",
            Mode::Scout => "Scout",
        }
    }
}

/// Effective effect settings for the processing chain.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EffectLevels {
    pub sbx_master: bool,
    pub surround_on: bool,
    pub surround: u8,
    pub crystalizer_on: bool,
    pub crystalizer: u8,
    pub bass_on: bool,
    pub bass: u8,
    pub bass_crossover: u32,
    pub smart_volume_on: bool,
    pub smart_volume: u8,
    pub dialog_on: bool,
    pub dialog: u8,
    pub scout: bool,
    pub direct: bool,
}

impl EffectLevels {
    /// Direct: long-press Scout. Everything bypassed, including the equalizer.
    pub fn direct() -> Self {
        Self {
            sbx_master: false,
            surround_on: false,
            surround: 0,
            crystalizer_on: false,
            crystalizer: 0,
            bass_on: false,
            bass: 0,
            bass_crossover: 80,
            smart_volume_on: false,
            smart_volume: 0,
            dialog_on: false,
            dialog: 0,
            scout: false,
            direct: true,
        }
    }

    /// Scout: SBX master, GraphicEQ, EAX and Reality3D are switched off while
    /// Scout runs on the device. The host keeps the settings so they can be
    /// restored when Scout is released.
    pub fn scout() -> Self {
        Self {
            scout: true,
            ..Self::direct()
        }
    }

    pub fn from_profile(profile: &Profile) -> Self {
        Self {
            sbx_master: profile.sbx_master,
            surround_on: profile.surround.enabled,
            surround: profile.surround.level,
            crystalizer_on: profile.crystalizer.enabled,
            crystalizer: profile.crystalizer.level,
            bass_on: profile.bass.enabled,
            bass: profile.bass.level,
            bass_crossover: profile.bass_crossover,
            smart_volume_on: profile.smart_volume.enabled,
            smart_volume: profile.smart_volume.level,
            dialog_on: profile.dialog_plus.enabled,
            dialog: profile.dialog_plus.level,
            scout: profile.scout,
            direct: false,
        }
    }

    /// True when the chain should pass audio through untouched.
    pub fn bypass_all(&self) -> bool {
        self.direct || self.scout || !self.sbx_master
    }

    /// True when the equalizer has to be bypassed as well.
    pub fn bypass_equalizer(&self) -> bool {
        self.bypass_all()
    }

    /// Smart Volume level in dB for `g6_svm_set_level`.
    ///
    /// The product stores 0..1 and the binary exposes no level knob
    /// (`+0x4b98` has no writer outside the constructor), so the dB mapping is
    /// not recoverable. This is a documented linear approximation over the
    /// client's existing -50..0 dB slider range, not a recovered constant.
    pub fn smart_volume_db(&self) -> f32 {
        -50.0 + 0.3 * self.smart_volume as f32
    }
}

/// The 23 factory profiles, ordered as the product orders them.
#[derive(Clone, Debug, Default)]
pub struct ProfileSet {
    profiles: Vec<Profile>,
}

impl ProfileSet {
    pub fn load() -> Result<Self, Box<dyn Error>> {
        Self::parse(PRODUCT_JSON)
    }

    pub fn parse(text: &str) -> Result<Self, Box<dyn Error>> {
        let root: Value = serde_json::from_str(text)?;
        let object = root["profiles"]
            .as_object()
            .ok_or("product data has no profiles object")?;
        let mut profiles = Vec::with_capacity(object.len());
        for (name, value) in object {
            profiles.push(parse_profile(name, value)?);
        }
        profiles.sort_by(|a, b| a.order.cmp(&b.order).then_with(|| a.name.cmp(&b.name)));
        if profiles.is_empty() {
            return Err("product data has no profiles".into());
        }
        Ok(Self { profiles })
    }

    pub fn profiles(&self) -> &[Profile] {
        &self.profiles
    }

    pub fn names(&self) -> Vec<&str> {
        self.profiles.iter().map(|p| p.name.as_str()).collect()
    }

    /// Case-insensitive lookup, matching how the CLI and the UI take names.
    pub fn find(&self, name: &str) -> Option<&Profile> {
        self.profiles
            .iter()
            .find(|profile| profile.name.eq_ignore_ascii_case(name))
    }
}

fn parse_profile(name: &str, value: &Value) -> Result<Profile, Box<dyn Error>> {
    let settings = value
        .get("settings")
        .and_then(|settings| settings.get("speaker"))
        .ok_or_else(|| format!("profile {name} has no settings.speaker"))?;
    let order = value
        .get("order")
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("profile {name} has no order"))? as u32;
    let bass = settings
        .get("Bass")
        .ok_or_else(|| format!("profile {name} has no Bass"))?;
    Ok(Profile {
        name: name.to_owned(),
        order,
        surround: parse_effect(settings, "Surround", name)?,
        crystalizer: parse_effect(settings, "Crystalizer", name)?,
        bass: ProfileEffect {
            enabled: bass["Enable"].as_bool().unwrap_or(false),
            level: level_percent(bass["Level"].as_f64().unwrap_or(0.0)),
        },
        bass_crossover: bass["XOver"].as_u64().unwrap_or(80) as u32,
        smart_volume: parse_effect(settings, "SVM", name)?,
        dialog_plus: parse_effect(settings, "DialogPlus", name)?,
        scout: settings
            .get("Scout")
            .and_then(|scout| scout["Enable"].as_bool())
            .unwrap_or(false),
        sbx_master: settings
            .get("SBXMaster")
            .and_then(|master| master["Enable"].as_bool())
            .unwrap_or(true),
    })
}

fn parse_effect(settings: &Value, key: &str, profile: &str) -> Result<ProfileEffect, Box<dyn Error>> {
    let effect = settings
        .get(key)
        .ok_or_else(|| format!("profile {profile} has no {key}"))?;
    Ok(ProfileEffect {
        enabled: effect["Enable"].as_bool().unwrap_or(false),
        level: level_percent(effect["Level"].as_f64().unwrap_or(0.0)),
    })
}

fn level_percent(level: f64) -> u8 {
    let scaled = (level * 100.0).round();
    scaled.clamp(0.0, 100.0) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set() -> ProfileSet {
        ProfileSet::load().expect("product data should parse")
    }

    #[test]
    fn loads_every_factory_profile_in_order() {
        let set = set();
        assert_eq!(set.profiles().len(), 23);
        assert_eq!(set.profiles()[0].name, "BaseProfile");
        assert_eq!(set.profiles()[0].order, 0);
        assert!(set.names().contains(&"FPS"));
    }

    #[test]
    fn gaming_matches_the_stored_flags() {
        let set = set();
        let levels = EffectLevels::from_profile(set.find("Gaming").unwrap());
        assert!(levels.surround_on && levels.crystalizer_on);
        assert_eq!(levels.surround, 12);
        assert_eq!(levels.crystalizer, 50);
        // Gaming keeps Bass, Smart Volume and Dialog+ switched off.
        assert!(!levels.bass_on);
        assert_eq!(levels.bass, 50);
        assert_eq!(levels.bass_crossover, 80);
        assert!(!levels.smart_volume_on);
        assert_eq!(levels.smart_volume, 74);
        assert!(!levels.dialog_on);
        assert!(!levels.bypass_all());
    }

    #[test]
    fn movie_and_rpg_enable_more_effects() {
        let set = set();
        let movie = EffectLevels::from_profile(set.find("Movie").unwrap());
        assert!(movie.smart_volume_on);
        assert_eq!(movie.smart_volume, 40);
        let rpg = EffectLevels::from_profile(set.find("RPG").unwrap());
        assert!(rpg.dialog_on);
        assert_eq!(rpg.dialog, 15);
        assert!(rpg.bass_on);
        assert_eq!(rpg.bass, 20);
        let dota = EffectLevels::from_profile(set.find("Dota2").unwrap());
        assert!(!dota.surround_on);
    }

    #[test]
    fn lookup_ignores_case() {
        let set = set();
        assert!(set.find("fps").is_some());
        assert!(set.find("FPS").is_some());
        assert!(set.find("not-a-profile").is_none());
    }

    #[test]
    fn direct_and_scout_bypass_everything() {
        for levels in [EffectLevels::direct(), EffectLevels::scout()] {
            assert!(levels.bypass_all());
            assert!(levels.bypass_equalizer());
            assert!(!levels.crystalizer_on);
            assert!(!levels.smart_volume_on);
        }
        // Scout keeps its own flag so the UI can explain what is missing.
        assert!(EffectLevels::scout().scout);
        assert!(EffectLevels::direct().direct);
    }

    #[test]
    fn smart_volume_db_is_the_documented_approximation() {
        let mut levels = EffectLevels::direct();
        levels.smart_volume = 0;
        assert!((levels.smart_volume_db() - -50.0).abs() < 1e-4);
        levels.smart_volume = 100;
        assert!((levels.smart_volume_db() - -20.0).abs() < 1e-4);
        levels.smart_volume = 74;
        assert!((levels.smart_volume_db() - -27.8).abs() < 1e-4);
    }

    #[test]
    fn every_profile_has_a_sbx_master_and_no_scout() {
        let set = set();
        for profile in set.profiles() {
            assert!(profile.sbx_master, "{} should have SBX on", profile.name);
            assert!(!profile.scout, "{} should not force Scout", profile.name);
            assert_eq!(profile.bass_crossover, 80, "{} crossover", profile.name);
        }
    }
}
