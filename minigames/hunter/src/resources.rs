use bevy::prelude::*;

use crate::types::HunterTarget;

/// A target the reticle can carry and release
pub struct TargetPreset {
    pub name: &'static str,
    pub balloon: bool,
    pub color: Color,
    pub default_size: f32,
}

pub const TARGET_PRESETS: [TargetPreset; 4] = [
    TargetPreset { name: "Cyan Practice Circle", balloon: false, color: Color::srgb(0.0, 0.9, 1.0), default_size: 0.45 },
    TargetPreset { name: "Yellow Rising Balloon", balloon: true, color: Color::srgb(1.0, 0.9, 0.1), default_size: 0.30 },
    TargetPreset { name: "Magenta Fast Balloon", balloon: true, color: Color::srgb(1.0, 0.1, 0.9), default_size: 0.20 },
    TargetPreset { name: "Green Medium Balloon", balloon: true, color: Color::srgb(0.1, 1.0, 0.3), default_size: 0.25 },
];

const SIZE_STEP: f32 = 0.05;
const SIZE_RANGE: (f32, f32) = (0.10, 1.00);

/// What the reticle does on trigger: shoot, or release the carried target preset
#[derive(Resource, Debug, Clone)]
pub struct HunterTargetSelection {
    /// Index into [`TARGET_PRESETS`]; `None` is gun-shot mode
    pub carried: Option<usize>,
    /// Current radius of each preset, scene metres
    pub sizes: [f32; TARGET_PRESETS.len()],
}

impl Default for HunterTargetSelection {
    fn default() -> Self {
        Self {
            carried: None,
            sizes: TARGET_PRESETS.map(|p| p.default_size),
        }
    }
}

impl HunterTargetSelection {
    /// The carried target, at its current size
    pub fn target(&self) -> Option<HunterTarget> {
        let i = self.carried?;
        let (preset, size) = (&TARGET_PRESETS[i], self.sizes[i]);
        Some(if preset.balloon {
            HunterTarget::Balloon(size, preset.color)
        } else {
            HunterTarget::Basic(size, preset.color)
        })
    }

    pub fn name(&self) -> String {
        match self.carried {
            Some(i) => format!("{} ({:.2}m)", TARGET_PRESETS[i].name, self.sizes[i]),
            None => "Gun Shot Mode".to_string(),
        }
    }

    /// Gun shot → each preset → gun shot
    pub fn cycle(&mut self) {
        self.carried = match self.carried {
            None => Some(0),
            Some(i) if i + 1 < TARGET_PRESETS.len() => Some(i + 1),
            Some(_) => None,
        };
    }

    /// Takes the carried target, back to gun-shot mode
    pub fn release(&mut self) -> Option<HunterTarget> {
        let target = self.target();
        self.carried = None;
        target
    }

    /// Grows (+1) or shrinks (−1) the carried preset by one step
    pub fn resize(&mut self, steps: f32) {
        if let Some(i) = self.carried {
            self.sizes[i] = (self.sizes[i] + steps * SIZE_STEP).clamp(SIZE_RANGE.0, SIZE_RANGE.1);
        }
    }
}
