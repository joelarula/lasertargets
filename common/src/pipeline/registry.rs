use super::stage::{PathSource, PathStage};
use bevy::prelude::*;
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::collections::HashMap;

type Factory<T> = Box<dyn Fn(&Value) -> Result<Box<T>, String> + Send + Sync>;

/// Every stage and source a pipeline file may name, keyed by its YAML name. Plugins add theirs
/// in `build`; pipelines are compiled against this registry when they are bound.
#[derive(Resource, Default)]
pub struct PipelineRegistry {
    stages: HashMap<String, Factory<dyn PathStage>>,
    sources: HashMap<String, Factory<dyn PathSource>>,
}

impl PipelineRegistry {
    /// A registry with every built-in stage and source
    pub fn with_builtins() -> Self {
        let mut registry = Self::default();
        super::sources::register(&mut registry);
        super::stages::register(&mut registry);
        registry
    }

    /// Registers stage `T` under `key`; its parameters are deserialized from the YAML entry
    pub fn register_stage<T: PathStage + DeserializeOwned>(&mut self, key: &str) -> &mut Self {
        self.stages.insert(
            key.to_string(),
            Box::new(|value| {
                serde_json::from_value::<T>(value.clone())
                    .map(|stage| Box::new(stage) as Box<dyn PathStage>)
                    .map_err(|e| e.to_string())
            }),
        );
        self
    }

    /// Registers source `T` under `key`
    pub fn register_source<T: PathSource + DeserializeOwned>(&mut self, key: &str) -> &mut Self {
        self.sources.insert(
            key.to_string(),
            Box::new(|value| {
                serde_json::from_value::<T>(value.clone())
                    .map(|source| Box::new(source) as Box<dyn PathSource>)
                    .map_err(|e| e.to_string())
            }),
        );
        self
    }

    pub fn build_stage(&self, key: &str, config: &Value) -> Result<Box<dyn PathStage>, String> {
        match self.stages.get(key) {
            Some(factory) => factory(config),
            None => Err(format!("unknown stage `{key}` (known: {})", sorted_keys(&self.stages))),
        }
    }

    pub fn build_source(&self, key: &str, config: &Value) -> Result<Box<dyn PathSource>, String> {
        match self.sources.get(key) {
            Some(factory) => factory(config),
            None => Err(format!("unknown source `{key}` (known: {})", sorted_keys(&self.sources))),
        }
    }
}

fn sorted_keys<T: ?Sized>(map: &HashMap<String, Factory<T>>) -> String {
    let mut keys: Vec<&str> = map.keys().map(String::as_str).collect();
    keys.sort_unstable();
    keys.join(", ")
}
