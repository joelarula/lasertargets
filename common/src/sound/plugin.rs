//! Bevy side of sound composition: the [`SoundCatalog`] resource, `.sound.yaml` assets with hot
//! reload, and the registration API plugins use to add sources, stages and sounds.

use super::catalog::SoundCatalog;
use super::node::{SoundSource, SoundStage};
use super::spec::SoundSpec;
use bevy::asset::{io::Reader, AssetLoader, LoadContext};
use bevy::prelude::*;
use serde::de::DeserializeOwned;
use std::collections::HashMap;
use std::sync::Arc;

/// Sounds compiled into the binary. Each is also loaded from `assets/sounds/<name>.sound.yaml`
/// when an asset server is present, so editing the file changes the sound live.
const BUILTIN_SOUNDS: &[&str] = &[
    include_str!("../../../assets/sounds/laser_hit.sound.yaml"),
    include_str!("../../../assets/sounds/target_spawn.sound.yaml"),
    include_str!("../../../assets/sounds/target_hit.sound.yaml"),
    include_str!("../../../assets/sounds/target_miss.sound.yaml"),
    include_str!("../../../assets/sounds/countdown_tick.sound.yaml"),
    include_str!("../../../assets/sounds/countdown_go.sound.yaml"),
    include_str!("../../../assets/sounds/game_over.sound.yaml"),
    include_str!("../../../assets/sounds/pellet_eat.sound.yaml"),
    include_str!("../../../assets/sounds/snake_crash.sound.yaml"),
    include_str!("../../../assets/sounds/calibration_corner_acquired.sound.yaml"),
    include_str!("../../../assets/sounds/calibration_success.sound.yaml"),
    include_str!("../../../assets/sounds/alert.sound.yaml"),
    include_str!("../../../assets/sounds/sci_fi_lock.sound.yaml"),
    include_str!("../../../assets/sounds/power_chord.sound.yaml"),
    include_str!("../../../assets/sounds/major_third_chime.sound.yaml"),
    include_str!("../../../assets/sounds/beat_warning.sound.yaml"),
    include_str!("../../../assets/sounds/theremin.sound.yaml"),
];

/// The built-in nodes and sounds, without Bevy (tests, `dac-test`, tools)
pub fn builtin_catalog() -> SoundCatalog {
    let mut catalog = SoundCatalog::with_builtins();
    for yaml in BUILTIN_SOUNDS {
        let spec = SoundSpec::from_yaml(yaml).unwrap_or_else(|e| panic!("built-in sound: {e}"));
        let name = spec.name.clone();
        catalog.insert(spec).unwrap_or_else(|e| panic!("built-in sound `{name}`: {}", e.join("; ")));
    }
    catalog
}

/// A parsed `.sound.yaml`
#[derive(Asset, TypePath, Debug, Clone)]
pub struct SoundAsset(pub Arc<SoundSpec>);

/// Handles of the sound files being watched, by sound name
#[derive(Resource, Default)]
struct SoundFiles(HashMap<String, Handle<SoundAsset>>);

pub struct SoundPlugin;

impl Plugin for SoundPlugin {
    fn build(&self, app: &mut App) {
        if !app.world().contains_resource::<SoundCatalog>() {
            app.insert_resource(SoundCatalog::with_builtins());
        }
        for yaml in BUILTIN_SOUNDS {
            app.embed_sound(yaml);
        }
        app.init_resource::<SoundFiles>();
        if app.is_plugin_added::<AssetPlugin>() {
            app.init_asset::<SoundAsset>()
                .register_asset_loader(SoundAssetLoader)
                .add_systems(Startup, watch_sound_files)
                .add_systems(Update, apply_sound_files);
        }
    }
}

/// Registration API for plugins
pub trait SoundAppExt {
    /// Makes a source available to sound files under `key`
    fn register_sound_source<T: SoundSource + DeserializeOwned>(&mut self, key: &str) -> &mut Self;
    /// Makes a stage available to sound files under `key`
    fn register_sound_stage<T: SoundStage + DeserializeOwned>(&mut self, key: &str) -> &mut Self;
    /// Compiles a sound file into the binary:
    /// `app.embed_sound(include_str!("../assets/sounds/hunter/pop.sound.yaml"))`. It is also
    /// loaded (and hot-reloaded) from the asset folder. Panics on an invalid file, so a broken
    /// embedded sound fails at startup. Register custom sources and stages first.
    fn embed_sound(&mut self, yaml: &str) -> &mut Self;
}

impl SoundAppExt for App {
    fn register_sound_source<T: SoundSource + DeserializeOwned>(&mut self, key: &str) -> &mut Self {
        catalog(self).register_source::<T>(key);
        self
    }

    fn register_sound_stage<T: SoundStage + DeserializeOwned>(&mut self, key: &str) -> &mut Self {
        catalog(self).register_stage::<T>(key);
        self
    }

    fn embed_sound(&mut self, yaml: &str) -> &mut Self {
        let spec = SoundSpec::from_yaml(yaml).unwrap_or_else(|e| panic!("embedded sound: {e}"));
        let name = spec.name.clone();
        if let Err(errors) = catalog(self).insert(spec) {
            panic!("embedded sound `{name}`:\n  {}", errors.join("\n  "));
        }
        self
    }
}

fn catalog(app: &mut App) -> Mut<'_, SoundCatalog> {
    if !app.world().contains_resource::<SoundCatalog>() {
        app.insert_resource(SoundCatalog::with_builtins());
    }
    app.world_mut().resource_mut::<SoundCatalog>()
}

#[derive(Default, TypePath)]
pub struct SoundAssetLoader;

impl AssetLoader for SoundAssetLoader {
    type Asset = SoundAsset;
    type Settings = ();
    type Error = std::io::Error;

    async fn load(&self, reader: &mut dyn Reader, _settings: &(), load_context: &mut LoadContext<'_>) -> Result<SoundAsset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let text = String::from_utf8(bytes).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        SoundSpec::from_yaml(&text)
            .map(|spec| SoundAsset(Arc::new(spec)))
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, format!("{}: {e}", load_context.path().display())))
    }

    fn extensions(&self) -> &[&str] {
        &["sound.yaml"]
    }
}

/// Loads `sounds/<name>.sound.yaml` for every sound known at startup
fn watch_sound_files(server: Res<AssetServer>, catalog: Res<SoundCatalog>, mut files: ResMut<SoundFiles>) {
    for name in catalog.names() {
        files.0.entry(name.to_string()).or_insert_with(|| server.load(format!("sounds/{name}.sound.yaml")));
    }
}

/// Loaded or edited files replace the catalog entry; a broken file keeps the previous version
fn apply_sound_files(mut events: MessageReader<AssetEvent<SoundAsset>>, assets: Res<Assets<SoundAsset>>, mut catalog: ResMut<SoundCatalog>) {
    for event in events.read() {
        let (AssetEvent::Added { id } | AssetEvent::Modified { id }) = event else { continue };
        let Some(asset) = assets.get(*id) else { continue };
        let spec = (*asset.0).clone();
        let name = spec.name.clone();
        let unchanged = catalog.get(&name).is_some_and(|current| **current == spec);
        if unchanged {
            continue;
        }
        match catalog.insert(spec) {
            Ok(()) => info!("sound `{name}` reloaded"),
            Err(errors) => error!("sound `{name}` is invalid, keeping the previous version:\n  {}", errors.join("\n  ")),
        }
    }
}
