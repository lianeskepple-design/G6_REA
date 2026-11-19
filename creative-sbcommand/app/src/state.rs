use aurora_audio_command::eq_core;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// ---------------------------------------------------------------- constants

/// Ten graphic-EQ bands, read from EqBandViewModel and ProfileConstants.
pub const EQ_FREQS: [u32; 10] = [
    31, 62, 125, 250, 500, 1000, 2000, 4000, 8000, 16000,
];
pub const EQ_MIN_DB: f32 = -12.0;
pub const EQ_MAX_DB: f32 = 12.0;
pub const EQ_STEP_DB: f32 = 0.1;

pub const SPEAKER_CONFIGS: [&str; 6] = ["Stereo", "2.1", "4.0", "4.1", "5.1", "7.1"];
pub const SPEAKER_TYPES: [&str; 5] = ["Builtin", "Desktop", "Bookshelf", "Tower", "Custom"];
pub const HEADPHONE_LOADS: [&str; 3] = [
    "Low (16 - 31 ohm)",
    "Normal (32 - 149 ohm)",
    "High (150 - 600 ohm)",
];
pub const AUDIO_QUALITIES: [&str; 4] = [
    "32-bit / 384 kHz",
    "32-bit / 192 kHz",
    "24-bit / 96 kHz",
    "16-bit / 48 kHz",
];
pub const SVM_MODES: [&str; 3] = ["Normal", "Night", "Loud"];
pub const SURROUND_MODES: [&str; 3] = ["Normal", "Wide", "Ultra Wide"];
pub const DIALOG_MODES: [&str; 3] = ["Normal", "Balanced", "Dialog Focus"];
/// Pattern names from Creative.Platform.Lighting.Pattern/Type.
pub const LIGHT_PATTERNS: [&str; 8] = [
    "Solo",
    "Aurora",
    "Wave",
    "Pulsate",
    "Mood",
    "Cycle",
    "Peak Meter",
    "Personal",
];
/// Direction names from Creative.Platform.Lighting.Direction/Type.
pub const LIGHT_DIRECTIONS: [&str; 4] = [
    "Left to Right",
    "Right to Left",
    "Left Bounce",
    "Right Bounce",
];
/// Microphone boost is read from the mixer line at run time; the installed
/// product package does not state the maximum. This is an estimate.
pub const MIC_BOOST_MAX: f32 = 30.0;
pub const LIGHT_COLORS: [(&str, [u8; 3]); 8] = [
    ("Aurora Teal", [0, 200, 230]),
    ("Spring", [38, 214, 164]),
    ("Deep Blue", [54, 126, 255]),
    ("Violet", [164, 96, 255]),
    ("Rose", [255, 86, 128]),
    ("Amber", [255, 176, 48]),
    ("Ice", [160, 225, 245]),
    ("White", [236, 240, 248]),
];

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Page {
    Sbx,
    Equalizer,
    Playback,
    Recording,
    Scout,
    Encoder,
    Mixer,
    Lighting,
    Settings,
}

impl Page {
    pub const ALL: [Page; 9] = [
        Page::Sbx,
        Page::Equalizer,
        Page::Playback,
        Page::Recording,
        Page::Scout,
        Page::Encoder,
        Page::Mixer,
        Page::Lighting,
        Page::Settings,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Page::Sbx => "SBX Profile",
            Page::Equalizer => "Equalizer",
            Page::Playback => "Playback",
            Page::Recording => "Recording",
            Page::Scout => "Scout Mode",
            Page::Encoder => "Encoder",
            Page::Mixer => "Mixer",
            Page::Lighting => "Lighting",
            Page::Settings => "Settings",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputMode {
    Speakers,
    Headphones,
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtKind {
    Cinema,
    Gaming,
    Concert,
    Night,
    Fps,
    Rpg,
    Racing,
    Strategy,
    Personal,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ProfileDef {
    pub name: &'static str,
    pub tagline: &'static str,
    pub description: &'static str,
    pub accent: [u8; 3],
    pub art: ArtKind,
    pub order: u32,
    pub levels: [f32; 5],
    pub svm_mode: u8,
    pub dialog_on: bool,
    pub scout_on: bool,
}

pub const PROFILES: [ProfileDef; 8] = [
    ProfileDef {
        name: "Gaming",
        tagline: "Positional and punchy",
        description: "Lifts positional cues and keeps explosions tight and controlled.",
        accent: [0, 200, 230],
        art: ArtKind::Gaming,
        order: 10,
        levels: [67.0, 30.0, 10.0, 74.0, 50.0],
        svm_mode: 0,
        dialog_on: false,
        scout_on: false,
    },
    ProfileDef {
        name: "Cinema",
        tagline: "Wide and cinematic",
        description: "Pristine dialogue, deep impacts and a wide, room-filling stage.",
        accent: [198, 74, 72],
        art: ArtKind::Cinema,
        order: 20,
        levels: [67.0, 55.0, 20.0, 74.0, 50.0],
        svm_mode: 0,
        dialog_on: false,
        scout_on: false,
    },
    ProfileDef {
        name: "Concert",
        tagline: "Live and open",
        description: "Opens the midrange and adds air for live recordings and acoustic sets.",
        accent: [164, 96, 255],
        art: ArtKind::Concert,
        order: 30,
        levels: [67.0, 50.0, 40.0, 60.0, 50.0],
        svm_mode: 0,
        dialog_on: false,
        scout_on: false,
    },
    ProfileDef {
        name: "Night",
        tagline: "Quiet-hours listening",
        description: "Keeps late-night listening quiet without losing detail or dialogue.",
        accent: [62, 128, 210],
        art: ArtKind::Night,
        order: 40,
        levels: [67.0, 55.0, 20.0, 74.0, 30.0],
        svm_mode: 1,
        dialog_on: true,
        scout_on: false,
    },
    ProfileDef {
        name: "FPS",
        tagline: "Footsteps up front",
        description: "Sharpens footsteps and distant cues for competitive play.",
        accent: [255, 126, 48],
        art: ArtKind::Fps,
        order: 100,
        levels: [12.0, 65.0, 20.0, 20.0, 50.0],
        svm_mode: 0,
        dialog_on: false,
        scout_on: false,
    },
    ProfileDef {
        name: "RPG",
        tagline: "Score and narration",
        description: "A warm, spacious score with narration kept clean and up front.",
        accent: [160, 90, 220],
        art: ArtKind::Rpg,
        order: 100,
        levels: [35.0, 50.0, 20.0, 30.0, 15.0],
        svm_mode: 0,
        dialog_on: true,
        scout_on: false,
    },
    ProfileDef {
        name: "Racing",
        tagline: "Engine and road",
        description: "Engine detail and road texture with a firm, controlled low end.",
        accent: [40, 190, 136],
        art: ArtKind::Racing,
        order: 100,
        levels: [30.0, 50.0, 50.0, 40.0, 30.0],
        svm_mode: 0,
        dialog_on: true,
        scout_on: false,
    },
    ProfileDef {
        name: "Strategy",
        tagline: "Broad and clear",
        description: "Clear orders and ambience across a broad, readable soundstage.",
        accent: [84, 156, 220],
        art: ArtKind::Strategy,
        order: 100,
        levels: [67.0, 30.0, 50.0, 50.0, 50.0],
        svm_mode: 0,
        dialog_on: false,
        scout_on: false,
    },
];

#[derive(Clone)]
pub struct EqPreset {
    pub name: String,
    pub speaker: [f32; 10],
    pub headphone: [f32; 10],
}

impl EqPreset {
    pub fn curve(&self, mode: OutputMode) -> [f32; 10] {
        match mode {
            OutputMode::Speakers => self.speaker,
            OutputMode::Headphones => self.headphone,
        }
    }
}

/// The 43 audited G6 factory EQ curves, embedded from
/// `reference/g6-eq-presets-current.json` and ordered like the original preset list.
static EQ_PRESETS: std::sync::LazyLock<Vec<EqPreset>> = std::sync::LazyLock::new(|| {
    eq_core::presets()
        .expect("embedded G6 factory EQ preset data is valid")
        .into_iter()
        .map(|p| EqPreset {
            name: p.name,
            speaker: gains_to_f32(&p.speaker),
            headphone: gains_to_f32(&p.headphone),
        })
        .collect()
});

pub fn eq_presets() -> &'static [EqPreset] {
    &EQ_PRESETS
}

fn gains_to_f32(curve: &eq_core::Curve) -> [f32; 10] {
    let gains = curve.gains();
    let mut out = [0.0; 10];
    for (slot, gain) in out.iter_mut().zip(gains.iter()) {
        *slot = *gain as f32;
    }
    out
}

// ---------------------------------------------------------------- state

#[derive(Clone, Serialize, Deserialize)]
pub struct Effects {
    pub sbx_on: bool,
    pub surround: f32,
    pub crystalizer: f32,
    pub bass: f32,
    pub smart_vol: f32,
    pub dialog: f32,
    pub surround_on: bool,
    pub crystalizer_on: bool,
    pub bass_on: bool,
    pub smart_on: bool,
    pub dialog_on: bool,
    pub surround_mode: u8,
    pub svm_mode: u8,
    pub dialog_mode: u8,
    pub bass_crossover: f32,
    pub subwoofer_gain: bool,
    pub active_profile: String,
}

impl Default for Effects {
    fn default() -> Self {
        Self::from_profile("Cinema")
    }
}

impl Effects {
    pub fn from_profile(name: &str) -> Self {
        let def = PROFILES
            .iter()
            .find(|p| p.name.eq_ignore_ascii_case(name))
            .unwrap_or(&PROFILES[1]);
        Self {
            sbx_on: true,
            surround: def.levels[0],
            crystalizer: def.levels[1],
            bass: def.levels[2],
            smart_vol: def.levels[3],
            dialog: def.levels[4],
            surround_on: true,
            crystalizer_on: true,
            bass_on: true,
            smart_on: true,
            dialog_on: def.dialog_on,
            surround_mode: 0,
            svm_mode: def.svm_mode,
            dialog_mode: 0,
            bass_crossover: 80.0,
            subwoofer_gain: false,
            active_profile: def.name.to_string(),
        }
    }

    pub fn apply_profile(&mut self, name: &str) {
        let saved_bass_x = self.bass_crossover;
        let saved_sub = self.subwoofer_gain;
        *self = Self::from_profile(name);
        self.bass_crossover = saved_bass_x;
        self.subwoofer_gain = saved_sub;
    }

    pub fn mark_personal(&mut self) {
        if self.active_profile != "Personal" {
            self.active_profile = "Personal".to_string();
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct CustomEqPreset {
    pub name: String,
    pub bands: [f32; 10],
}

#[derive(Clone, Serialize, Deserialize)]
pub struct EqState {
    pub enabled: bool,
    pub bands: [f32; 10],
    pub preamp: f32,
    pub preset: String,
    pub sync_outputs: bool,
    pub custom: Vec<CustomEqPreset>,
}

impl Default for EqState {
    fn default() -> Self {
        Self {
            enabled: true,
            bands: [0.0; 10],
            preamp: 0.0,
            preset: "Flat".to_string(),
            sync_outputs: true,
            custom: Vec::new(),
        }
    }
}

impl EqState {
    /// Loads a factory curve for the given output mode, or a personal/custom one.
    pub fn apply_preset(&mut self, name: &str, mode: OutputMode) {
        if let Some(p) = eq_presets().iter().find(|p| p.name.eq_ignore_ascii_case(name)) {
            self.bands = p.curve(mode);
            self.preset = p.name.clone();
        } else if name == "Personal" {
            self.preset = "Personal".to_string();
        } else if let Some(p) = self.custom.iter().find(|p| p.name == name) {
            self.bands = p.bands;
            self.preset = p.name.clone();
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct MixerChannel {
    pub name: String,
    pub volume: f32,
    pub muted: bool,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct MixerState {
    pub playback: Vec<MixerChannel>,
    pub recording: Vec<MixerChannel>,
    pub master: f32,
    pub master_muted: bool,
    pub mic_volume: f32,
    pub mic_muted: bool,
    pub mic_boost: f32,
    pub monitor: bool,
    pub agc: bool,
    pub balance_l: f32,
    pub balance_r: f32,
    pub selected_input: usize,
}

impl Default for MixerState {
    fn default() -> Self {
        Self {
            playback: vec![
                MixerChannel { name: "Speakers".into(), volume: 74.0, muted: false },
                MixerChannel { name: "Digital Out".into(), volume: 92.0, muted: false },
            ],
            recording: vec![
                MixerChannel { name: "Microphone".into(), volume: 63.0, muted: false },
                MixerChannel { name: "What U Hear".into(), volume: 50.0, muted: false },
                MixerChannel { name: "Line In".into(), volume: 42.0, muted: false },
            ],
            master: 78.0,
            master_muted: false,
            mic_volume: 64.0,
            mic_muted: false,
            mic_boost: 12.0,
            monitor: false,
            agc: true,
            balance_l: 0.0,
            balance_r: 0.0,
            selected_input: 0,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct PlaybackState {
    pub output: OutputMode,
    pub speaker_config: usize,
    pub speaker_type: usize,
    pub bass_redirection: bool,
    pub crossover: f32,
    pub subwoofer_gain: bool,
    pub full_range_front: bool,
    pub full_range_surround: bool,
    pub headphone_gain: usize,
    pub headphone_load: usize,
    pub hp_virtualization: bool,
    pub retain_gain: bool,
    pub audio_quality: usize,
    pub set_default: bool,
    pub direct: bool,
}

impl Default for PlaybackState {
    fn default() -> Self {
        Self {
            output: OutputMode::Speakers,
            speaker_config: 4,
            speaker_type: 1,
            bass_redirection: true,
            crossover: 80.0,
            subwoofer_gain: false,
            full_range_front: false,
            full_range_surround: false,
            headphone_gain: 0,
            headphone_load: 1,
            hp_virtualization: false,
            retain_gain: false,
            audio_quality: 0,
            set_default: true,
            direct: false,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct RecordingState {
    pub selected_input: usize,
    pub smart_volume: bool,
    pub noise_reduction: bool,
    pub aec: bool,
    pub voice_focus: bool,
    pub mic_eq: usize,
    pub monitor: bool,
    /// Voice Morph preset index (Neutral first), from the VoiceFx preset list.
    #[serde(default)]
    pub voice_fx: usize,
}

/// Voice Morph presets found in the AE9 PresetConfig.json.
pub const VOICE_FX: [&str; 6] = ["Neutral", "Male", "Female", "Kid", "Elderly", "Deep Voice"];

impl Default for RecordingState {
    fn default() -> Self {
        Self {
            selected_input: 0,
            smart_volume: true,
            noise_reduction: true,
            aec: false,
            voice_focus: true,
            mic_eq: 0,
            monitor: false,
            voice_fx: 0,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ScoutState {
    pub on: bool,
    pub hotkey_enabled: bool,
    pub hotkey_text: String,
    pub radar: bool,
    pub capturing: bool,
}

impl Default for ScoutState {
    fn default() -> Self {
        Self {
            on: false,
            hotkey_enabled: false,
            hotkey_text: "Ctrl + Shift + S".into(),
            radar: true,
            capturing: false,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct EncoderState {
    pub mode: usize,
    pub neo_pc: bool,
    pub neo_mode: usize,
    pub width: f32,
}

impl Default for EncoderState {
    fn default() -> Self {
        Self { mode: 0, neo_pc: false, neo_mode: 1, width: 50.0 }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct LightingState {
    pub on: bool,
    pub pattern: usize,
    pub color: usize,
    pub brightness: f32,
    pub speed: f32,
    pub direction: usize,
    pub all_zones: bool,
}

impl Default for LightingState {
    fn default() -> Self {
        Self {
            on: true,
            pattern: 2,
            color: 0,
            brightness: 80.0,
            speed: 50.0,
            direction: 0,
            all_zones: true,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct SettingsState {
    pub language: usize,
    pub startup: bool,
    pub start_minimized: bool,
    pub last_check: String,
    pub tab: usize,
}

impl Default for SettingsState {
    fn default() -> Self {
        Self {
            language: 0,
            startup: true,
            start_minimized: false,
            last_check: "Not checked yet".into(),
            tab: 0,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct AppState {
    pub page: Page,
    pub effects: Effects,
    pub eq: EqState,
    pub mixer: MixerState,
    pub playback: PlaybackState,
    pub recording: RecordingState,
    pub scout: ScoutState,
    pub encoder: EncoderState,
    pub lighting: LightingState,
    pub settings: SettingsState,
    #[serde(default)]
    pub custom_profiles: Vec<(String, Effects)>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            page: Page::Sbx,
            effects: Effects::default(),
            eq: EqState::default(),
            mixer: MixerState::default(),
            playback: PlaybackState::default(),
            recording: RecordingState::default(),
            scout: ScoutState::default(),
            encoder: EncoderState::default(),
            lighting: LightingState::default(),
            settings: SettingsState::default(),
            custom_profiles: Vec::new(),
        }
    }
}

impl AppState {
    pub fn is_direct(&self) -> bool {
        self.playback.direct
    }

    pub fn processing_allowed(&self) -> bool {
        !self.playback.direct
    }

    pub fn sbx_active(&self) -> bool {
        self.effects.sbx_on && self.processing_allowed()
    }

    pub fn eq_active(&self) -> bool {
        self.eq.enabled && self.processing_allowed()
    }

    pub fn scout_active(&self) -> bool {
        self.scout.on && self.processing_allowed()
    }

    pub fn toggle_direct(&mut self) {
        let next = !self.playback.direct;
        self.playback.direct = next;
        if next {
            // Direct Mode and Scout Mode are mutually exclusive in AE9ViewModel.
            self.scout.on = false;
        }
    }

    pub fn toggle_scout(&mut self) {
        let next = !self.scout.on;
        self.scout.on = next;
        if next {
            self.playback.direct = false;
        }
    }

    pub fn save(&self) {
        let Some(path) = state_path() else { return };
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(text) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(path, text);
        }
    }

    pub fn load() -> Self {
        let Some(path) = state_path() else { return Self::default() };
        let Ok(text) = std::fs::read_to_string(path) else { return Self::default() };
        serde_json::from_str(&text).unwrap_or_default()
    }
}

fn state_path() -> Option<PathBuf> {
    let appdata = std::env::var_os("APPDATA")?;
    Some(PathBuf::from(appdata).join("AuroraAudioCommand").join("state.json"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn factory_presets_are_the_audited_43() {
        assert_eq!(eq_presets().len(), 43);
        assert_eq!(eq_presets()[0].name, "Acoustic");
    }

    #[test]
    fn apply_preset_uses_the_curve_for_the_output_mode() {
        let mut eq = EqState::default();
        let preset = eq_presets()
            .iter()
            .find(|p| p.name == "Fortnite")
            .expect("Fortnite is a factory preset");
        eq.apply_preset("Fortnite", OutputMode::Speakers);
        assert_eq!(eq.bands, preset.speaker);
        assert_eq!(eq.preset, "Fortnite");
        eq.apply_preset("Fortnite", OutputMode::Headphones);
        assert_eq!(eq.bands, preset.headphone);
        assert_ne!(
            preset.speaker, preset.headphone,
            "Fortnite has distinct speaker/headphone factory curves"
        );
    }

    #[test]
    fn personal_and_custom_presets_still_work() {
        let mut eq = EqState::default();
        eq.custom.push(CustomEqPreset { name: "Mine".into(), bands: [1.0; 10] });
        eq.apply_preset("Mine", OutputMode::Speakers);
        assert_eq!(eq.bands, [1.0; 10]);
        assert_eq!(eq.preset, "Mine");
        eq.apply_preset("Personal", OutputMode::Speakers);
        assert_eq!(eq.preset, "Personal");
    }

    #[test]
    fn unknown_preset_leaves_the_curve_alone() {
        let mut eq = EqState::default();
        let before = eq.bands;
        eq.apply_preset("Cinema", OutputMode::Speakers);
        assert_eq!(eq.bands, before);
        assert_eq!(eq.preset, "Flat");
    }
}
