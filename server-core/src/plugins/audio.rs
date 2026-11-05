use std::collections::HashMap;
use bevy::prelude::*;
use common::audio::{AudioConfiguration, PlaySoundEvent, SoundId};
use common::settings::ConfigAppExt;
use common::vision::VisionDetection;

/// Resource storing loaded handles for sound cues
#[derive(Resource, Default)]
pub struct SoundBank {
    pub handles: HashMap<SoundId, Handle<AudioSource>>,
}

/// Standalone plugin for game audio, sound effects, and acoustic laser feedback.
pub struct LaserAudioPlugin;

impl Plugin for LaserAudioPlugin {
    fn build(&self, app: &mut App) {
        app.init_config::<AudioConfiguration>();
        app.add_plugins(bevy::audio::AudioPlugin::default());
        app.add_message::<PlaySoundEvent>();
        app.init_resource::<SoundBank>();
        app.add_systems(Startup, setup_sound_bank);
        app.add_systems(Update, (handle_play_sound_events, handle_vision_hit_audio));
    }
}

/// Preloads sound effect assets into the SoundBank resource on startup
fn setup_sound_bank(
    asset_server: Res<AssetServer>,
    mut sound_bank: ResMut<SoundBank>,
) {
    let sounds = [
        (SoundId::LaserHit, "sounds/laser_hit.wav"),
        (SoundId::TargetSpawn, "sounds/target_spawn.wav"),
        (SoundId::TargetHit, "sounds/target_hit.wav"),
        (SoundId::TargetMiss, "sounds/target_miss.wav"),
        (SoundId::CountdownTick, "sounds/countdown_tick.wav"),
        (SoundId::CountdownGo, "sounds/countdown_go.wav"),
        (SoundId::GameOver, "sounds/game_over.wav"),
        (SoundId::PelletEat, "sounds/pellet_eat.wav"),
        (SoundId::SnakeCrash, "sounds/snake_crash.wav"),
        (SoundId::CalibrationCornerAcquired, "sounds/calibration_tone.wav"),
        (SoundId::Alert, "sounds/alert.wav"),
    ];

    for (id, path) in sounds {
        let handle: Handle<AudioSource> = asset_server.load(path);
        sound_bank.handles.insert(id, handle);
    }

    info!("SoundBank initialized with {} audio cue definitions", sound_bank.handles.len());
}

/// Plays audio when a PlaySoundEvent is received
fn handle_play_sound_events(
    mut commands: Commands,
    mut sound_events: MessageReader<PlaySoundEvent>,
    config: Res<AudioConfiguration>,
    sound_bank: Res<SoundBank>,
) {
    if !config.enabled {
        // Drain events without playing if audio is disabled
        for _ in sound_events.read() {}
        return;
    }

    for event in sound_events.read() {
        if let Some(handle) = sound_bank.handles.get(&event.sound) {
            let volume = (config.volume * event.volume).clamp(0.0, 1.0);
            let mut settings = PlaybackSettings::DESPAWN;
            settings.volume = bevy::audio::Volume::Linear(volume);

            commands.spawn((
                AudioPlayer::new(handle.clone()),
                settings,
            ));
        } else {
            warn!("Sound cue {:?} requested but not found in SoundBank", event.sound);
        }
    }
}

/// Automatically triggers a hit sound cue when the camera detects a laser spot appear on target
fn handle_vision_hit_audio(
    mut detections: MessageReader<VisionDetection>,
    mut sound_events: MessageWriter<PlaySoundEvent>,
    config: Res<AudioConfiguration>,
) {
    if !config.enabled {
        return;
    }

    for detection in detections.read() {
        if detection.appeared && (detection.label == "laser" || detection.label == "spot") {
            let mut event = PlaySoundEvent::new(SoundId::LaserHit);
            if config.spatial_panning {
                event = event.with_pan(detection.galvo_pos.x);
            }
            sound_events.write(event);
        }
    }
}
