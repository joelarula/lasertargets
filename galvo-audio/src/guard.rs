//! Last stage before the galvos. It always runs, and no config value or cue can push past
//! its hard limits:
//!
//! 1. Frequency ceiling: low-pass at `ceiling_hz`, never above [`ABSOLUTE_MAX_HZ`].
//! 2. Resonance cut: the scanner's mechanical resonance band is attenuated by `notch_db`.
//! 3. Thermal taper: after `thermal_secs` of continuous loud drive the gain slides to
//!    `thermal_floor` and recovers once things go quiet.
//! 4. Excursion clamp: offsets never exceed [`ABSOLUTE_MAX_EXCURSION_DAC`].
//! 5. Underrun fade: when no sample arrives the last one decays instead of stepping to 0, and
//!    enabling or disabling ramps over 10 ms. Steps in coil current are what click.

use common::sound::dsp::Biquad;

/// Hard cap on the audio offset of any DAC point, in 12-bit DAC units (~2% of full scale).
/// Protects the coils and driver amplifiers from overheating.
pub const ABSOLUTE_MAX_EXCURSION_DAC: u16 = 80;

/// Hard frequency ceiling. Above ~6 kHz the drive heats the damping circuit without being heard.
pub const ABSOLUTE_MAX_HZ: f32 = 5500.0;

/// Drive level (0..1 RMS) that counts as "loud" for the thermal taper
const THERMAL_LEVEL: f32 = 0.2;

/// Guard settings; built from `audio.galvo` by the plugin and sent to the DAC thread.
#[derive(Debug, Clone, PartialEq)]
pub struct GuardSettings {
    pub enabled: bool,
    pub master_volume: f32,
    pub max_excursion_dac: u16,
    pub ceiling_hz: f32,
    pub notch_min_hz: f32,
    pub notch_max_hz: f32,
    pub notch_db: f32,
    pub thermal_secs: f32,
    pub thermal_floor: f32,
    pub play_when_blank: bool,
}

impl Default for GuardSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            master_volume: 0.8,
            max_excursion_dac: 45,
            ceiling_hz: ABSOLUTE_MAX_HZ,
            notch_min_hz: 3200.0,
            notch_max_hz: 4200.0,
            notch_db: -10.0,
            thermal_secs: 5.0,
            thermal_floor: 0.5,
            play_when_blank: true,
        }
    }
}

impl GuardSettings {
    /// Offset in DAC units for a full-scale (1.0) sample, before the thermal taper
    pub fn amplitude_dac(&self) -> f32 {
        self.max_excursion_dac.min(ABSOLUTE_MAX_EXCURSION_DAC) as f32 * self.master_volume.clamp(0.0, 1.0)
    }
}

pub struct Guard {
    settings: GuardSettings,
    sample_rate: f32,
    lowpass: [Biquad; 2],
    notch: [Biquad; 2],
    last_input: [f32; 2],
    /// Mean square of the filtered drive, ~50 ms window
    mean_square: f32,
    hot_secs: f32,
    /// 0..1 ramp for enable/disable
    enable_gain: f32,
}

impl Guard {
    pub fn new(settings: GuardSettings, sample_rate: u32) -> Self {
        let mut guard = Self {
            settings: settings.clone(),
            sample_rate: 1.0,
            lowpass: [Biquad::identity(); 2],
            notch: [Biquad::identity(); 2],
            last_input: [0.0; 2],
            mean_square: 0.0,
            hot_secs: 0.0,
            enable_gain: 0.0,
        };
        guard.configure(settings, sample_rate);
        guard
    }

    pub fn settings(&self) -> &GuardSettings {
        &self.settings
    }

    /// Applies new settings or a new sample rate; filters are rebuilt only when needed
    pub fn configure(&mut self, settings: GuardSettings, sample_rate: u32) {
        let rate = sample_rate.max(1) as f32;
        let filters_changed = rate != self.sample_rate
            || settings.ceiling_hz != self.settings.ceiling_hz
            || settings.notch_min_hz != self.settings.notch_min_hz
            || settings.notch_max_hz != self.settings.notch_max_hz
            || settings.notch_db != self.settings.notch_db;
        self.sample_rate = rate;
        if filters_changed {
            let ceiling = settings.ceiling_hz.clamp(20.0, ABSOLUTE_MAX_HZ);
            self.lowpass = [Biquad::lowpass(ceiling, rate); 2];
            let cut = Biquad::band_cut(settings.notch_min_hz, settings.notch_max_hz, settings.notch_db.min(0.0), rate);
            self.notch = [cut; 2];
        }
        self.settings = settings;
    }

    /// Current thermal gain, 1 (cool) down to `thermal_floor`
    pub fn thermal_gain(&self) -> f32 {
        let floor = self.settings.thermal_floor.clamp(0.0, 1.0);
        let over = ((self.hot_secs - self.settings.thermal_secs.max(0.0)) / 2.0).clamp(0.0, 1.0);
        1.0 - (1.0 - floor) * over
    }

    /// One DAC point: the next sample (or `None` on underrun) → offset in DAC units
    pub fn process(&mut self, input: Option<[f32; 2]>) -> (i16, i16) {
        let dt = 1.0 / self.sample_rate;
        // ~7 ms decay at 30 kpps
        let raw = input.unwrap_or([self.last_input[0] * 0.995, self.last_input[1] * 0.995]);
        self.last_input = raw;

        let target = if self.settings.enabled { 1.0 } else { 0.0 };
        let ramp = dt / 0.010;
        self.enable_gain = if self.enable_gain < target {
            (self.enable_gain + ramp).min(target)
        } else {
            (self.enable_gain - ramp).max(target)
        };

        let mut out = [0.0_f32; 2];
        for axis in 0..2 {
            let x = raw[axis].clamp(-1.0, 1.0);
            out[axis] = self.notch[axis].process(self.lowpass[axis].process(x));
        }

        let level = out[0] * out[0] + out[1] * out[1];
        self.mean_square += (level - self.mean_square) * (dt / 0.050).min(1.0);
        if self.mean_square.sqrt() > THERMAL_LEVEL {
            self.hot_secs += dt;
        } else {
            self.hot_secs = (self.hot_secs - dt).max(0.0);
        }

        let amplitude = self.settings.amplitude_dac() * self.thermal_gain() * self.enable_gain;
        let cap = ABSOLUTE_MAX_EXCURSION_DAC as f32;
        let to_dac = |v: f32| (v * amplitude).round().clamp(-cap, cap) as i16;
        (to_dac(out[0]), to_dac(out[1]))
    }
}
