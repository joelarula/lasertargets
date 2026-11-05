//! Every source and stage a sound file may name, and every composed sound by name.

use super::node::{SoundCtx, SoundSource, SoundStage};
use super::spec::SoundSpec;
use super::voice::{LayerVoice, SoundVoice};
use crate::audio_stream::StereoFrame;
use crate::pipeline::spec::Keyed;
use bevy::prelude::Resource;
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

type Factory<T> = Box<dyn Fn(&Value) -> Result<Box<T>, String> + Send + Sync>;

/// Nested `sound:` sources deeper than this are refused (and so are cycles)
const MAX_NESTING: usize = 8;

/// Plugins register sources and stages in `build`; sounds come from embedded files, asset files
/// (hot-reloaded) and code. Outputs ask it for a [`SoundVoice`] when they start a sound.
#[derive(Resource, Default)]
pub struct SoundCatalog {
    sources: HashMap<String, Factory<dyn SoundSource>>,
    stages: HashMap<String, Factory<dyn SoundStage>>,
    sounds: HashMap<String, Arc<SoundSpec>>,
}

impl SoundCatalog {
    /// A catalog with every built-in source and stage, and no sounds
    pub fn with_builtins() -> Self {
        let mut catalog = Self::default();
        super::sources::register(&mut catalog);
        super::stages::register(&mut catalog);
        catalog
    }

    pub fn register_source<T: SoundSource + DeserializeOwned>(&mut self, key: &str) -> &mut Self {
        self.sources.insert(
            key.to_string(),
            Box::new(|value| serde_json::from_value::<T>(value.clone()).map(|s| Box::new(s) as Box<dyn SoundSource>).map_err(|e| e.to_string())),
        );
        self
    }

    pub fn register_stage<T: SoundStage + DeserializeOwned>(&mut self, key: &str) -> &mut Self {
        self.stages.insert(
            key.to_string(),
            Box::new(|value| serde_json::from_value::<T>(value.clone()).map(|s| Box::new(s) as Box<dyn SoundStage>).map_err(|e| e.to_string())),
        );
        self
    }

    /// Adds or replaces a sound after checking every source and stage it names. Nested sounds
    /// may be added later; they are resolved when a voice starts.
    pub fn insert(&mut self, spec: SoundSpec) -> Result<(), Vec<String>> {
        self.build(&spec, 0, false)?;
        self.sounds.insert(spec.name.clone(), Arc::new(spec));
        Ok(())
    }

    pub fn get(&self, name: &str) -> Option<&Arc<SoundSpec>> {
        self.sounds.get(name)
    }

    pub fn contains(&self, name: &str) -> bool {
        self.sounds.contains_key(name)
    }

    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.sounds.keys().map(String::as_str)
    }

    /// A fresh voice of sound `name`
    pub fn voice(&self, name: &str) -> Result<SoundVoice, String> {
        let spec = self.sounds.get(name).ok_or_else(|| format!("unknown sound `{name}`"))?;
        self.build(spec, 0, true).map_err(|e| e.join("; "))
    }

    /// A fresh voice of a sound built in code, not in the catalog
    pub fn voice_of(&self, spec: &SoundSpec) -> Result<SoundVoice, String> {
        self.build(spec, 0, true).map_err(|e| e.join("; "))
    }

    fn build(&self, spec: &SoundSpec, depth: usize, strict: bool) -> Result<SoundVoice, Vec<String>> {
        let mut errors = Vec::new();
        let at = |path: String, e: String| format!("{}: {path}: {e}", spec.name);
        let mut layers = Vec::new();
        for (i, layer) in spec.layers.iter().enumerate() {
            let source = match self.build_source(&layer.source, depth, strict) {
                Ok(source) => source,
                Err(e) => {
                    errors.push(at(format!("layers[{i}].source.{}", layer.source.key), e));
                    continue;
                }
            };
            let stages = self.build_stages(&layer.stages, &mut errors, |j, key| at(format!("layers[{i}].stages[{j}].{key}"), String::new()));
            let copies = layer.repeat + 1;
            // Repeats need their own stateful nodes; build them again from the same spec
            let mut first = Some((source, stages));
            for copy in 0..copies {
                let (source, stages) = match first.take() {
                    Some(nodes) => nodes,
                    None => {
                        let Ok(source) = self.build_source(&layer.source, depth, strict) else { break };
                        let stages = self.build_stages(&layer.stages, &mut Vec::new(), |_, _| String::new());
                        (source, stages)
                    }
                };
                let start = (layer.at_ms + copy as f32 * layer.every_ms) / 1000.0;
                layers.push(LayerVoice::new(start, source, stages));
            }
        }
        let stages = self.build_stages(&spec.stages, &mut errors, |j, key| at(format!("stages[{j}].{key}"), String::new()));
        if spec.layers.is_empty() {
            errors.push(format!("{}: a sound needs at least one layer", spec.name));
        }
        if !errors.is_empty() {
            return Err(errors);
        }
        Ok(SoundVoice::new(layers, stages, spec.gain.clone()))
    }

    fn build_source(&self, entry: &Keyed, depth: usize, strict: bool) -> Result<Box<dyn SoundSource>, String> {
        if entry.key == "sound" {
            let name = match &entry.config {
                Value::String(name) => name.clone(),
                Value::Object(map) => map.get("name").and_then(Value::as_str).ok_or("expected `sound: <name>`")?.to_string(),
                _ => return Err("expected `sound: <name>`".into()),
            };
            if depth >= MAX_NESTING {
                return Err(format!("sounds nested deeper than {MAX_NESTING} (a cycle through `{name}`?)"));
            }
            return match self.sounds.get(&name) {
                Some(spec) => self.build(spec, depth + 1, strict).map(|voice| Box::new(Nested(voice)) as Box<dyn SoundSource>).map_err(|e| e.join("; ")),
                None if strict => Err(format!("unknown sound `{name}`")),
                // Validation: the nested sound may be added later
                None => Ok(Box::new(Nested(SoundVoice::new(Vec::new(), Vec::new(), 1.0.into())))),
            };
        }
        match self.sources.get(&entry.key) {
            Some(factory) => factory(&entry.config),
            None => Err(format!("unknown source (known: {}, sound)", sorted(&self.sources))),
        }
    }

    fn build_stages(&self, entries: &[Keyed], errors: &mut Vec<String>, at: impl Fn(usize, &str) -> String) -> Vec<Box<dyn SoundStage>> {
        let mut stages = Vec::new();
        for (j, entry) in entries.iter().enumerate() {
            match self.stages.get(&entry.key) {
                Some(factory) => match factory(&entry.config) {
                    Ok(stage) => stages.push(stage),
                    Err(e) => errors.push(format!("{}{e}", at(j, &entry.key))),
                },
                None => errors.push(format!("{}unknown stage (known: {})", at(j, &entry.key), sorted(&self.stages))),
            }
        }
        stages
    }
}

fn sorted<T: ?Sized>(map: &HashMap<String, Factory<T>>) -> String {
    let mut keys: Vec<&str> = map.keys().map(String::as_str).collect();
    keys.sort_unstable();
    keys.join(", ")
}

/// `sound: <name>` as a source: a whole composed sound inside another one
struct Nested(SoundVoice);

impl SoundSource for Nested {
    fn duration_secs(&self) -> Option<f32> {
        self.0.duration()
    }

    fn render(&mut self, ctx: &SoundCtx, out: &mut [StereoFrame]) {
        out.fill([0.0; 2]);
        self.0.render(ctx.sample_rate, ctx.default_route, ctx.signals, out);
    }

    fn pitch_hz(&self) -> Option<f32> {
        self.0.pitch_hz()
    }
}
