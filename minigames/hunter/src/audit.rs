//! The round's audit log. Every setup change, pause, wave, spawn, shot and escape is appended
//! with the time it happened and what the game knew at that moment: positions both in world
//! and in scene metres, the nearest target and its offset from the shot, and the scene, camera
//! and projector setup in effect. Reading it back shows whether misses come from aim (scattered
//! offsets) or from camera/projector calibration (a consistent offset, or one that changes
//! right after a setup change).

use std::fmt::{self, Write as _};
use std::path::PathBuf;

use bevy::asset::uuid::Uuid;
use bevy::prelude::*;
use common::scene::SceneSetup;
use serde::{Deserialize, Serialize};

use crate::types::HunterTarget;

/// Running totals, derived from the log as it is written
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct HunterTally {
    pub spawned: u32,
    pub popped: u32,
    /// Shots inside the scene that hit nothing
    pub misses: u32,
    /// Balloons that rose out of the scene unshot
    pub escaped: u32,
    pub score: u32,
}

impl HunterTally {
    fn count(&mut self, event: &HunterAuditEvent) {
        match event {
            HunterAuditEvent::Spawned { .. } => self.spawned += 1,
            HunterAuditEvent::Shot(Shot { outcome: ShotOutcome::Hit { reward, .. }, .. }) => {
                self.popped += 1;
                self.score += reward;
            }
            HunterAuditEvent::Shot(Shot { outcome: ShotOutcome::Miss, .. }) => self.misses += 1,
            HunterAuditEvent::Escaped { .. } => self.escaped += 1,
            _ => {}
        }
    }
}

impl fmt::Display for HunterTally {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Spawned: {} | Hits: {} | Misses: {} | Escaped: {} | Points: {}",
            self.spawned, self.popped, self.misses, self.escaped, self.score
        )
    }
}

/// Where a shot came from: the server-side gamepad reticle or a terminal's mouse (camera ray)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShotSource {
    Gamepad,
    Mouse,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "result", rename_all = "snake_case")]
pub enum ShotOutcome {
    Hit { target: Uuid, reward: u32, lifetime: f64 },
    /// Inside the scene, nothing hit
    Miss,
    /// Outside the scene: not counted, but a sign the shot mapping is off
    OffScene,
}

/// The target closest to a shot, whether hit or not
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct NearestTarget {
    pub target: Uuid,
    /// Target centre, scene metres
    pub local: Vec3,
    /// Shot minus target centre, scene metres
    pub offset: Vec2,
    pub radius: f32,
    /// Still under spawn immunity, so it could not be hit
    pub immune: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Shot {
    pub source: ShotSource,
    pub world: Vec3,
    /// The shot in scene metres, through the scene transform in effect at the time
    pub local: Vec3,
    pub outcome: ShotOutcome,
    pub nearest: Option<NearestTarget>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum HunterAuditEvent {
    /// Scene, camera and projector in effect from here on (logged at the start and on every change)
    Setup(SceneSetup),
    Paused,
    Resumed,
    /// The workflow released a wave
    Wave { wave: u32, targets: u32 },
    Spawned { target: Uuid, kind: HunterTarget, world: Vec3, local: Vec3 },
    Shot(Shot),
    Escaped { target: Uuid, local: Vec3 },
    RoundFinished,
}

impl HunterAuditEvent {
    fn label(&self) -> &'static str {
        match self {
            Self::Setup(_) => "setup",
            Self::Paused => "paused",
            Self::Resumed => "resumed",
            Self::Wave { .. } => "wave",
            Self::Spawned { .. } => "spawned",
            Self::Shot(Shot { outcome: ShotOutcome::Hit { .. }, .. }) => "hit",
            Self::Shot(Shot { outcome: ShotOutcome::Miss, .. }) => "miss",
            Self::Shot(Shot { outcome: ShotOutcome::OffScene, .. }) => "off-scene",
            Self::Escaped { .. } => "escaped",
            Self::RoundFinished => "round finished",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HunterAuditEntry {
    /// Seconds since the session started
    pub t: f64,
    #[serde(flatten)]
    pub event: HunterAuditEvent,
}

/// One session's tally and audit log. The server keeps the full log; terminals only the tally.
#[derive(Resource, Debug, Clone, Default, Serialize, Deserialize)]
pub struct HunterGameStats {
    pub session_id: Uuid,
    /// Wall clock at the session start, seconds since the Unix epoch (to line the log up with
    /// camera and projector logs)
    pub started_unix: f64,
    /// `Time::elapsed_secs_f64` at the session start; entry times count from here
    pub started_at: f64,
    pub tally: HunterTally,
    pub log: Vec<HunterAuditEntry>,
}

impl HunterGameStats {
    pub fn new(session_id: Uuid, now: f64) -> Self {
        let started_unix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0.0, |d| d.as_secs_f64());
        Self { session_id, started_unix, started_at: now, ..default() }
    }

    /// Terminal side: the totals the server sent
    pub fn from_tally(session_id: Uuid, tally: HunterTally) -> Self {
        Self { session_id, tally, ..default() }
    }

    /// Seconds since the session started
    pub fn since_start(&self, now: f64) -> f64 {
        now - self.started_at
    }

    /// Appends `event` at `now` and counts it
    pub fn record(&mut self, now: f64, event: HunterAuditEvent) {
        self.tally.count(&event);
        self.log.push(HunterAuditEntry { t: self.since_start(now), event });
    }

    /// Logs `setup` if it differs from the one in effect
    pub fn note_setup(&mut self, now: f64, setup: &SceneSetup) {
        if self.setup_in_effect() != Some(setup) {
            self.record(now, HunterAuditEvent::Setup(setup.clone()));
        }
    }

    pub fn setup_in_effect(&self) -> Option<&SceneSetup> {
        self.log.iter().rev().find_map(|entry| match &entry.event {
            HunterAuditEvent::Setup(setup) => Some(setup),
            _ => None,
        })
    }

    pub fn shots(&self) -> impl Iterator<Item = &Shot> {
        self.log.iter().filter_map(|entry| match &entry.event {
            HunterAuditEvent::Shot(shot) => Some(shot),
            _ => None,
        })
    }
}

/// Post-game summary plus the full log
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameReport {
    pub session_id: Uuid,
    pub started_unix: f64,
    pub duration: f64,
    pub tally: HunterTally,
    /// Mean time between spawns, from the first to the last
    pub avg_spawn_interval: f64,
    /// Mean time from spawn to pop over popped targets
    pub avg_target_lifetime: f64,
    /// Mean shot − nearest-target offset over in-scene shots, scene metres. A consistent offset
    /// points at calibration rather than aim.
    pub mean_aim_offset: Option<Vec2>,
    /// Setups logged after the first one
    pub setup_changes: usize,
    pub log: Vec<HunterAuditEntry>,
}

fn mean<T: std::iter::Sum<T> + std::ops::Div<f32, Output = T>>(values: impl Iterator<Item = T>) -> Option<T> {
    let values: Vec<T> = values.collect();
    let n = values.len() as f32;
    (n > 0.0).then(|| values.into_iter().sum::<T>() / n)
}

impl GameReport {
    pub fn new(stats: &HunterGameStats, now: f64) -> Self {
        let spawns: Vec<f64> = stats
            .log
            .iter()
            .filter(|e| matches!(e.event, HunterAuditEvent::Spawned { .. }))
            .map(|e| e.t)
            .collect();
        let avg_spawn_interval = match spawns.as_slice() {
            [first, .., last] => (last - first) / (spawns.len() - 1) as f64,
            _ => 0.0,
        };
        let lifetimes = stats.shots().filter_map(|shot| match shot.outcome {
            ShotOutcome::Hit { lifetime, .. } => Some(lifetime as f32),
            _ => None,
        });
        let in_scene_offsets = stats
            .shots()
            .filter(|shot| shot.outcome != ShotOutcome::OffScene)
            .filter_map(|shot| shot.nearest.map(|n| n.offset));
        let setups = stats.log.iter().filter(|e| matches!(e.event, HunterAuditEvent::Setup(_))).count();

        Self {
            session_id: stats.session_id,
            started_unix: stats.started_unix,
            duration: stats.since_start(now),
            tally: stats.tally,
            avg_spawn_interval,
            avg_target_lifetime: mean(lifetimes).unwrap_or(0.0) as f64,
            mean_aim_offset: mean(in_scene_offsets),
            setup_changes: setups.saturating_sub(1),
            log: stats.log.clone(),
        }
    }

    /// Writes `report.md` and `report.json` to `stats/hunter/<session>/`
    pub fn save(&self) -> std::io::Result<PathBuf> {
        let dir = PathBuf::from(format!("stats/hunter/{}", self.session_id));
        std::fs::create_dir_all(&dir)?;
        std::fs::write(dir.join("report.md"), self.to_markdown())?;
        std::fs::write(dir.join("report.json"), serde_json::to_string_pretty(self)?)?;
        Ok(dir)
    }

    pub fn to_markdown(&self) -> String {
        let mut s = String::new();
        let t = &self.tally;
        let _ = writeln!(s, "# Hunter Game Report\n");
        let _ = writeln!(s, "Session `{}`, started at Unix {:.3}\n", self.session_id, self.started_unix);
        let _ = writeln!(s, "## Summary");
        let _ = writeln!(s, "- **Duration**: {:.2}s", self.duration);
        let _ = writeln!(s, "- **Spawned / Popped / Escaped**: {} / {} / {}", t.spawned, t.popped, t.escaped);
        let _ = writeln!(s, "- **Misses**: {}", t.misses);
        let _ = writeln!(s, "- **Score**: {}", t.score);
        let _ = writeln!(s, "- **Avg Spawn Interval**: {:.2}s", self.avg_spawn_interval);
        let _ = writeln!(s, "- **Avg Target Lifetime**: {:.2}s", self.avg_target_lifetime);
        match self.mean_aim_offset {
            Some(o) => { let _ = writeln!(s, "- **Mean Aim Offset**: ({:+.3}, {:+.3}) m", o.x, o.y); }
            None => { let _ = writeln!(s, "- **Mean Aim Offset**: no shots"); }
        }
        let _ = writeln!(s, "- **Setup Changes During Round**: {}", self.setup_changes);

        let _ = writeln!(s, "\n## Timeline\n");
        let _ = writeln!(s, "Positions in scene metres unless marked world; offset = shot − nearest target.\n");
        let _ = writeln!(s, "| t | Event | Details |");
        let _ = writeln!(s, "|---|-------|---------|");
        for entry in &self.log {
            let _ = writeln!(s, "| {:.3}s | {} | {} |", entry.t, entry.event.label(), details(&entry.event));
        }
        s
    }
}

fn v3(v: Vec3) -> String {
    format!("({:.3}, {:.3}, {:.3})", v.x, v.y, v.z)
}

fn details(event: &HunterAuditEvent) -> String {
    match event {
        HunterAuditEvent::Setup(setup) => {
            let (scene, cam, proj) = (&setup.scene, &setup.camera, &setup.projector);
            format!(
                "scene {}×{} at {} rot {:?} scale {}, y-diff {:.3}; camera {}×{} at {} fov {:.1}° locked {}; \
                 projector {}×{} at {} angle {:.1}° on {} connected {} locked {}",
                scene.scene_dimension.x, scene.scene_dimension.y, v3(scene.origin.translation),
                scene.origin.rotation, v3(scene.origin.scale), scene.y_difference,
                cam.resolution.x, cam.resolution.y, v3(cam.origin.translation), cam.angle, cam.locked_to_scene,
                proj.resolution.x, proj.resolution.y, v3(proj.origin.translation), proj.angle,
                proj.switched_on, proj.connected, proj.locked_to_scene,
            )
        }
        HunterAuditEvent::Paused | HunterAuditEvent::Resumed | HunterAuditEvent::RoundFinished => String::new(),
        HunterAuditEvent::Wave { wave, targets } => format!("wave {wave}, {targets} targets"),
        HunterAuditEvent::Spawned { target, kind, world, local } => {
            format!("{target} {kind:?} at {} (world {})", v3(*local), v3(*world))
        }
        HunterAuditEvent::Shot(shot) => {
            let mut d = format!("{:?} at {} (world {})", shot.source, v3(shot.local), v3(shot.world));
            if let ShotOutcome::Hit { target, reward, lifetime } = shot.outcome {
                let _ = write!(d, "; popped {target} +{reward} after {lifetime:.2}s");
            }
            if let Some(n) = shot.nearest {
                let _ = write!(
                    d,
                    "; nearest {} at {} offset ({:+.3}, {:+.3}) |{:.3}| r {:.3}{}",
                    n.target, v3(n.local), n.offset.x, n.offset.y, n.offset.length(), n.radius,
                    if n.immune { " immune" } else { "" },
                );
            }
            d
        }
        HunterAuditEvent::Escaped { target, local } => format!("{target} at {}", v3(*local)),
    }
}
