use bevy::prelude::*;
use std::collections::HashMap;

/// Latest value of every global signal (`audio.bass`, `gamepad.rt`, `game.combo` ...), 0..1.
#[derive(Resource, Default, Debug, Clone)]
pub struct Signals(pub HashMap<String, f32>);

impl Signals {
    pub fn get(&self, id: &str) -> Option<f32> {
        self.0.get(id).copied()
    }

    pub fn set(&mut self, id: impl Into<String>, value: f32) {
        self.0.insert(id.into(), value);
    }
}

/// Signals of one entity, read in pipelines as `self.<name>`: component bindings (`health`),
/// reaction envelopes (`hit`), the pipeline's own clock (`life`, `age_secs`) and its local
/// signals (`lfo.bob`).
#[derive(Component, Default, Debug, Clone)]
pub struct EntitySignals(pub HashMap<String, f32>);

impl EntitySignals {
    pub fn get(&self, name: &str) -> Option<f32> {
        self.0.get(name).copied()
    }

    pub fn set(&mut self, name: impl Into<String>, value: f32) {
        self.0.insert(name.into(), value);
    }
}

/// Resolves signal ids for one entity: `self.x` from the entity, anything else from the
/// entity's local signals first, then the global ones. Unknown signals read 0.
#[derive(Clone, Copy)]
pub struct SignalView<'a> {
    pub entity: &'a EntitySignals,
    pub global: &'a Signals,
}

impl<'a> SignalView<'a> {
    pub fn new(entity: &'a EntitySignals, global: &'a Signals) -> Self {
        Self { entity, global }
    }

    pub fn get(&self, id: &str) -> f32 {
        if let Some(name) = id.strip_prefix("self.") {
            return self.entity.get(name).unwrap_or(0.0);
        }
        self.entity.get(id).or_else(|| self.global.get(id)).unwrap_or(0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_self_local_then_global() {
        let mut entity = EntitySignals::default();
        entity.set("health", 0.4);
        entity.set("lfo.bob", 0.7);
        let mut global = Signals::default();
        global.set("audio.bass", 0.9);
        global.set("lfo.bob", 0.1);

        let view = SignalView::new(&entity, &global);
        assert_eq!(view.get("self.health"), 0.4);
        assert_eq!(view.get("lfo.bob"), 0.7, "local shadows global");
        assert_eq!(view.get("audio.bass"), 0.9);
        assert_eq!(view.get("self.missing"), 0.0);
        assert_eq!(view.get("health"), 0.4, "entity signals are also visible without self.");
    }
}
