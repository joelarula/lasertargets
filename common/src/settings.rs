//! Layered YAML configuration backed by [figment].
//!
//! There is no central settings schema: each config struct owns a section of the merged tree
//! through [`ConfigSection::KEY`], and plugins read their sections with
//! [`ConfigAppExt::init_config`]. Several structs can share one YAML block by using nested keys
//! (`projector`, `projector.optimize`, ...).

use bevy::prelude::*;
use figment::providers::{Env, Format, Serialized, Yaml};
use figment::Figment;
use serde::{de::DeserializeOwned, Serialize};
use std::path::{Path, PathBuf};

/// Prefix of environment overrides, e.g. `LASER_NETWORK__PORT=6001` sets `network.port`.
pub const ENV_PREFIX: &str = "LASER_";

/// Merged configuration tree, inserted before plugins are added so they can read their sections.
#[derive(Resource, Clone, Debug)]
pub struct ConfigSource(pub Figment);

impl ConfigSource {
    /// Loads a profiled YAML file (top-level keys are profiles: `default`, `rpi`, ...) and overlays
    /// `LASER_*` environment variables, which win over every profile. `profile` is applied on top
    /// of `default`.
    pub fn load(file: Option<&Path>, profile: &str) -> Self {
        let mut figment = Figment::new();
        if let Some(file) = file {
            figment = figment.merge(Yaml::file(file).nested());
        }
        let env = Env::prefixed(ENV_PREFIX)
            .ignore(&["config", "profile"])
            .split("__")
            .global();
        Self(figment.merge(env).select(profile))
    }

    /// Finds `config/<name>` in the working directory, then next to the executable.
    pub fn locate(name: &str) -> Option<PathBuf> {
        let exe_dir = std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(Path::to_path_buf));
        std::iter::once(PathBuf::from("config"))
            .chain(exe_dir.map(|dir| dir.join("config")))
            .map(|dir| dir.join(name))
            .find(|path| path.is_file())
    }

    /// Extracts the section at `key` layered over `fallback`: keys absent from the config keep
    /// the fallback's values, so partial sections are fine.
    pub fn section<T: Serialize + DeserializeOwned>(
        &self,
        key: &str,
        fallback: T,
    ) -> Result<T, figment::Error> {
        match self.0.find_value(key) {
            Ok(value) => Figment::from(Serialized::defaults(fallback))
                .merge(Serialized::defaults(value))
                .extract(),
            Err(_) => Ok(fallback),
        }
    }
}

/// A resource whose values come from one section of the [`ConfigSource`].
pub trait ConfigSection: Resource + Serialize + DeserializeOwned {
    /// Dotted path into the merged config, e.g. `"projector.optimize"`.
    const KEY: &'static str;
}

pub trait ConfigAppExt {
    /// Inserts `T` from its config section over `T::default()`.
    fn init_config<T: ConfigSection + Default>(&mut self) -> &mut Self;
    /// Inserts `T` from its config section over `fallback`.
    fn init_config_or<T: ConfigSection>(&mut self, fallback: T) -> &mut Self;
}

impl ConfigAppExt for App {
    fn init_config<T: ConfigSection + Default>(&mut self) -> &mut Self {
        self.init_config_or(T::default())
    }

    fn init_config_or<T: ConfigSection>(&mut self, fallback: T) -> &mut Self {
        // A resource inserted earlier (e.g. by a test) takes precedence over the config.
        if self.world().contains_resource::<T>() {
            return self;
        }
        let value = match self.world().get_resource::<ConfigSource>() {
            Some(source) => source
                .section(T::KEY, fallback)
                .unwrap_or_else(|e| panic!("invalid config section '{}': {e}", T::KEY)),
            None => fallback,
        };
        self.insert_resource(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Resource, Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct Outer {
        rate: u32,
        name: String,
    }

    impl ConfigSection for Outer {
        const KEY: &'static str = "outer";
    }

    #[derive(Resource, Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct Inner {
        steps: u16,
        enabled: bool,
    }

    impl ConfigSection for Inner {
        const KEY: &'static str = "outer.inner";
    }

    const YAML: &str = "
default:
  outer:
    rate: 100
    inner:
      steps: 4
rpi:
  outer:
    rate: 50
";

    fn source(yaml: &str, profile: &str) -> ConfigSource {
        ConfigSource(Figment::from(Yaml::string(yaml).nested()).select(profile))
    }

    fn outer_default() -> Outer {
        Outer { rate: 1, name: "fallback".into() }
    }

    fn inner_default() -> Inner {
        Inner { steps: 1, enabled: true }
    }

    #[test]
    fn partial_section_keeps_fallback_values() {
        let outer = source(YAML, "default").section(Outer::KEY, outer_default()).unwrap();
        assert_eq!(outer, Outer { rate: 100, name: "fallback".into() });
    }

    #[test]
    fn profile_overrides_default() {
        let outer = source(YAML, "rpi").section(Outer::KEY, outer_default()).unwrap();
        assert_eq!(outer.rate, 50);
        let inner = source(YAML, "rpi").section(Inner::KEY, inner_default()).unwrap();
        assert_eq!(inner, Inner { steps: 4, enabled: true });
    }

    #[test]
    fn missing_section_uses_fallback() {
        let inner = source("default: {}", "default").section(Inner::KEY, inner_default()).unwrap();
        assert_eq!(inner, inner_default());
    }

    #[test]
    fn invalid_value_is_an_error() {
        let yaml = "default: { outer: { rate: fast } }";
        assert!(source(yaml, "default").section(Outer::KEY, outer_default()).is_err());
    }

    #[test]
    fn init_config_reads_section_and_respects_existing_resource() {
        let mut app = App::new();
        app.insert_resource(source(YAML, "default"))
            .insert_resource(inner_default())
            .init_config_or(outer_default())
            .init_config_or(Inner { steps: 9, enabled: false });
        assert_eq!(app.world().resource::<Outer>().rate, 100);
        assert_eq!(*app.world().resource::<Inner>(), inner_default());
    }

    #[test]
    fn init_config_without_source_uses_fallback() {
        let mut app = App::new();
        app.init_config_or(outer_default());
        assert_eq!(*app.world().resource::<Outer>(), outer_default());
    }
}
