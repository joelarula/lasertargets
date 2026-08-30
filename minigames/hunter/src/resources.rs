use bevy::asset::uuid::Uuid;
use bevy::prelude::*;
use common::target::HunterTarget;
use serde::{Deserialize, Serialize};

use crate::types::TargetEvent;

/// Resource tracking the reticle cursor mode and target selection
#[derive(Resource, Debug, Clone)]
pub struct HunterTargetSelection {
    pub selected_index: usize, // 0 = GunShot Mode (Default), 1 = Cyan Static Big, 2 = Yellow Balloon, 3 = Magenta Balloon, 4 = Green Balloon
    pub sizes: [f32; 5],
}

impl Default for HunterTargetSelection {
    fn default() -> Self {
        Self {
            selected_index: 0,
            sizes: [0.0, 0.45, 0.30, 0.20, 0.25],
        }
    }
}

impl HunterTargetSelection {
    pub fn get_target(&self) -> Option<HunterTarget> {
        let size = self.sizes.get(self.selected_index).copied().unwrap_or(0.25);
        match self.selected_index {
            1 => Some(HunterTarget::Basic(size, Color::srgb(0.0, 0.9, 1.0))),  // Static Big: Cyan Large Practice Circle
            2 => Some(HunterTarget::Baloon(size, Color::srgb(1.0, 0.9, 0.1))),  // Balloon 1: Yellow Rising Balloon
            3 => Some(HunterTarget::Baloon(size, Color::srgb(1.0, 0.1, 0.9))),  // Balloon 2: Magenta Small Fast Balloon
            4 => Some(HunterTarget::Baloon(size, Color::srgb(0.1, 1.0, 0.3))),  // Balloon 3: Green Medium Balloon
            _ => None, // 0 = GunShot Mode
        }
    }

    pub fn target_name(&self) -> String {
        let size = self.sizes.get(self.selected_index).copied().unwrap_or(0.25);
        match self.selected_index {
            0 => "Gun Shot Mode".to_string(),
            1 => format!("Cyan Practice Circle ({:.2}m)", size),
            2 => format!("Yellow Rising Balloon ({:.2}m)", size),
            3 => format!("Magenta Fast Balloon ({:.2}m)", size),
            4 => format!("Green Medium Balloon ({:.2}m)", size),
            _ => "Gun Shot Mode".to_string(),
        }
    }

    pub fn cycle(&mut self) {
        self.selected_index = (self.selected_index + 1) % 5;
    }

    pub fn reset_to_gunshot(&mut self) {
        self.selected_index = 0;
    }

    pub fn increase_size(&mut self) {
        if self.selected_index > 0 && self.selected_index < 5 {
            self.sizes[self.selected_index] = (self.sizes[self.selected_index] + 0.05).min(1.00);
        }
    }

    pub fn decrease_size(&mut self) {
        if self.selected_index > 0 && self.selected_index < 5 {
            self.sizes[self.selected_index] = (self.sizes[self.selected_index] - 0.05).max(0.10);
        }
    }
}

/// Resource for tracking game statistics
#[derive(Component, Resource, Debug, Clone, Serialize, Deserialize, Default)]
pub struct HunterGameStats {
    pub session_id: Uuid,
    pub targets_spawned: u32,
    pub targets_popped: u32,
    pub misses: u32,
    pub score: u32,
    pub target_events: Vec<TargetEvent>,
    pub game_start_time: f64,
}
