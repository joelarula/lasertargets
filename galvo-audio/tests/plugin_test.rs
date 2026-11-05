//! Headless tests of the galvo output and the audio router: which sounds reach the galvos,
//! signals, triggers and scopes out, and the hand-off to a DAC consumer.

use bevy::prelude::*;
use common::audio::{AudioOutput, AudioRouting, PlaySound, PlaySoundEvent, RoutedSound, SoundId, StopSound};
use common::audio_stream::LiveAudioFrame;
use common::pipeline::plugin::{BasePath, PathPipelinePlugin, ReactionTriggered, UsePipeline};
use common::pipeline::Signals;
use common::settings::ConfigSource;
use common::sound::{SoundCatalog, SoundSpec};
use galvo_audio::*;
use test_harness::TestApp;

use AudioOutput::{Galvo, Speakers};

fn app(config: GalvoAudioConfig, routing: AudioRouting) -> TestApp {
    let mut t = TestApp::new();
    t.insert_resource(config).insert_resource(routing);
    t.add_plugins((PathPipelinePlugin, GalvoAudioPlugin));
    t.capture::<ReactionTriggered>().capture::<RoutedSound>();
    // First frame has no elapsed time; start from a running clock
    t.step();
    t
}

fn on() -> GalvoAudioConfig {
    GalvoAudioConfig { enabled: true, ..Default::default() }
}

/// Every sound to the galvos
fn to_galvo() -> AudioRouting {
    AudioRouting { default: vec![Galvo], ..Default::default() }
}

fn signal(t: &TestApp, id: &str) -> f32 {
    t.resource::<Signals>().get(id).unwrap_or(0.0)
}

fn routed_to(t: &TestApp, name: &str) -> Vec<AudioOutput> {
    t.captured::<RoutedSound>().iter().filter(|r| r.name == name).map(|r| r.output).collect()
}

#[test]
fn sound_event_routed_to_galvo_plays_its_cue_and_publishes_signals() {
    let mut t = app(on(), to_galvo());
    assert_eq!(signal(&t, "galvo.level"), 0.0);

    t.send(PlaySoundEvent::new(SoundId::TargetHit)).step();
    assert!(signal(&t, "galvo.level") > 0.3, "level up in the same frame ({})", signal(&t, "galvo.level"));
    assert_eq!(signal(&t, "galvo.playing"), 1.0);
    let pitch = signal(&t, "galvo.pitch");
    assert!(pitch > 0.7 && pitch < 0.8, "1760 Hz → ~0.75 ({pitch})");

    t.step_n(30);
    assert!(t.captured::<ReactionTriggered>().iter().any(|r| r.key == "sound.target_hit"), "sound announced");
    assert!(signal(&t, "galvo.level") < 0.01, "level decays after the cue");
    assert_eq!(signal(&t, "galvo.playing"), 0.0);
}

#[test]
fn default_routing_is_speakers_only() {
    let mut t = app(on(), AudioRouting::default());
    t.send(PlaySoundEvent::new(SoundId::LaserHit)).step_n(2);
    assert_eq!(routed_to(&t, "laser_hit"), vec![Speakers]);
    assert_eq!(signal(&t, "galvo.level"), 0.0, "galvos stay quiet");
    assert!(t.captured::<ReactionTriggered>().iter().any(|r| r.key == "sound.laser_hit"), "visuals still hear it");
}

#[test]
fn per_sound_routing_splits_and_mutes() {
    let mut routing = AudioRouting::default();
    routing.sounds.insert("countdown_tick".into(), vec![Galvo]);
    routing.sounds.insert("laser_hit".into(), vec![Speakers, Galvo]);
    routing.sounds.insert("alert".into(), vec![]);
    let mut t = app(on(), routing);
    t.send(PlaySoundEvent::new(SoundId::CountdownTick))
        .send(PlaySoundEvent::new(SoundId::LaserHit))
        .send(PlaySoundEvent::new(SoundId::Alert))
        .send(PlaySoundEvent::new(SoundId::TargetHit).on([Galvo]))
        .step_n(2);
    assert_eq!(routed_to(&t, "countdown_tick"), vec![Galvo]);
    assert_eq!(routed_to(&t, "laser_hit"), vec![Speakers, Galvo]);
    assert!(routed_to(&t, "alert").is_empty(), "muted");
    assert!(!t.captured::<ReactionTriggered>().iter().any(|r| r.key == "sound.alert"), "a muted sound isn't announced");
    assert_eq!(routed_to(&t, "target_hit"), vec![Galvo], "per-event override beats the default");
}

#[test]
fn parallel_limit_reaches_every_output() {
    let mut routing = AudioRouting { default: vec![Speakers, Galvo], max_parallel: 3, ..Default::default() };
    routing.parallel.insert("countdown_tick".into(), 1);
    let mut t = app(on(), routing);
    t.send(PlaySoundEvent::new(SoundId::LaserHit)).send(PlaySoundEvent::new(SoundId::CountdownTick)).step_n(2);
    let limit = |name: &str| t.captured::<RoutedSound>().iter().filter(|r| r.name == name).map(|r| r.max_parallel).collect::<Vec<_>>();
    assert_eq!(limit("laser_hit"), vec![3, 3], "same limit for speakers and galvo");
    assert_eq!(limit("countdown_tick"), vec![1, 1]);

    // On the galvos, five rapid hits in one frame leave three sounding
    for _ in 0..5 {
        t.send(PlaySoundEvent::new(SoundId::LaserHit));
    }
    t.step();
    assert_eq!(t.resource::<GalvoSynth>().mixer.copies_of("laser_hit"), 3);
}

#[test]
fn disabled_galvo_output_stays_silent_even_when_routed() {
    let mut t = app(GalvoAudioConfig::default(), to_galvo());
    t.send(PlaySoundEvent::new(SoundId::Alert)).step();
    assert_eq!(signal(&t, "galvo.level"), 0.0);
}

#[test]
fn pipeline_triggers_play_mapped_sounds_on_their_outputs() {
    let mut routing = to_galvo();
    routing.on_trigger.insert("snake.gem_eaten".into(), "pellet_eat".into());
    // A galvo-only cue, no speaker file needed
    routing.on_trigger.insert("hunter.lock_on".into(), "sci_fi_lock".into());
    // Announcements never loop back into sounds
    routing.on_trigger.insert("sound.pellet_eat".into(), "alert".into());
    let mut t = app(on(), routing);
    t.send(ReactionTriggered::broadcast("snake.gem_eaten")).step();
    assert!(signal(&t, "galvo.level") > 0.3);
    t.step_n(30);
    assert!(t.captured::<ReactionTriggered>().iter().any(|r| r.key == "sound.pellet_eat"));
    assert!(routed_to(&t, "alert").is_empty(), "`sound.*` triggers are not routed");

    t.send(ReactionTriggered::broadcast("hunter.lock_on")).step();
    assert!(signal(&t, "galvo.x") > 0.3 && signal(&t, "galvo.y") > 0.3, "two-voice cue on both galvos");
}

fn spec(yaml: &str) -> SoundSpec {
    SoundSpec::from_yaml(yaml).unwrap()
}

#[test]
fn plays_edited_and_inline_compositions() {
    let mut t = app(on(), to_galvo());
    // What hot reload does when assets/sounds/laser_hit.sound.yaml is edited
    t.resource_mut::<SoundCatalog>()
        .insert(spec("name: laser_hit
layers:
  - source: { tone: { hz: 300, ms: 40 } }
"))
        .unwrap();
    t.send(PlaySoundEvent::new(SoundId::LaserHit)).step();
    assert!(signal(&t, "galvo.level") > 0.3);
    let pitch = signal(&t, "galvo.pitch");
    assert!(pitch < 0.45, "the edited 300 Hz composition replaced the chirp ({pitch})");

    // A chord built in code, sent to the galvos only (routing alone would send nothing)
    let mut t = app(on(), AudioRouting::default());
    let chord = spec("name: test/chord
layers:
  - source: { tone: { hz: 440, ms: 50, route: x } }
  - source: { tone: { hz: 660, ms: 50, route: y } }
");
    t.send(PlaySound::inline(chord).on([Galvo])).step();
    assert!(signal(&t, "galvo.x") > 0.3 && signal(&t, "galvo.y") > 0.3, "both voices play");
}

#[test]
fn instruments_are_sounds_that_follow_signals_until_stopped() {
    let mut t = app(on(), AudioRouting::default());
    t.send(PlaySound::named("theremin").on([Galvo])).step_n(5);
    assert!(signal(&t, "galvo.level") < 0.01, "gamepad.ly = 0 is silent");
    t.resource_mut::<Signals>().set("gamepad.ly", 1.0);
    t.resource_mut::<Signals>().set("gamepad.lx", 0.5);
    t.step_n(10);
    assert!(signal(&t, "galvo.level") > 0.6, "up with its signal ({})", signal(&t, "galvo.level"));
    let pitch = signal(&t, "galvo.pitch");
    assert!((pitch - 0.50).abs() < 0.03, "C5 → ~0.50 ({pitch})");
    t.send(StopSound::named("theremin")).step_n(30);
    assert!(signal(&t, "galvo.level") < 0.01, "stopped");
    assert_eq!(t.resource::<GalvoSynth>().mixer.active_voices(), 0);
}

#[test]
fn renders_ahead_for_a_dac_consumer() {
    let mut t = app(on(), to_galvo());
    let bus = t.resource::<GalvoAudioBus>().clone();
    let mut output = bus.output(30_000);
    t.send(PlaySoundEvent::new(SoundId::CountdownGo)).step();
    let queued = bus.queued();
    assert!((2300..=2500).contains(&queued), "80 ms ahead at 30 kpps ({queued})");
    let offsets = output.render(1000).to_vec();
    assert!(offsets.iter().any(|o| o.0 != 0 || o.1 != 0), "the DAC side hears it");
    t.step();
    assert!(bus.queued() >= 2300, "topped up again");
}

fn stream_tone() -> LiveAudioFrame {
    let samples = (0..800).flat_map(|i| {
        let s = (i as f32 * 0.1).sin() * 0.8;
        [s, -s]
    }).collect();
    LiveAudioFrame { samples, channels: 2, sample_rate: 48_000 }
}

#[test]
fn stream_feeds_signals_and_scopes_and_reaches_the_galvos_only_when_routed() {
    let mut t = app(on(), AudioRouting::default());
    let scope = t.world_mut().spawn((AudioScope { input: ScopeInput::Stream, ..Default::default() }, UsePipeline::new("galvo/scope"))).id();
    t.send(stream_tone()).step();
    assert!(signal(&t, "audio.level") > 0.7);
    assert_eq!(signal(&t, "galvo.level"), 0.0, "not routed to the galvos");
    let base = t.world().get::<BasePath>(scope).unwrap();
    assert!(base.segments.first().is_some_and(|s| s.points.len() > 50), "scope drew the stream");

    let mut t = app(on(), AudioRouting { stream: vec![Galvo], ..Default::default() });
    t.send(stream_tone()).step();
    assert!(signal(&t, "galvo.level") > 0.3, "stream sung by the galvos");
}

#[test]
fn config_sections_parse_dotted_keys() {
    let yaml = "default:\n  audio:\n    routing:\n      default: [speakers]\n      sounds:\n        countdown_tick: [galvo]\n      on_trigger:\n        snake.gem_eaten: pellet_eat\n    galvo:\n      enabled: true\n      route: x\n";
    let dir = std::env::temp_dir().join(format!("galvo_cfg_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("server.yaml");
    std::fs::write(&file, yaml).unwrap();
    let source = ConfigSource::load(Some(&file), "default");
    let routing: AudioRouting = source.section("audio.routing", AudioRouting::default()).unwrap();
    assert_eq!(routing.on_trigger.get("snake.gem_eaten").map(String::as_str), Some("pellet_eat"));
    assert_eq!(routing.outputs_for("countdown_tick"), &[Galvo]);
    let config: GalvoAudioConfig = source.section("audio.galvo", GalvoAudioConfig::default()).unwrap();
    assert!(config.enabled);
    assert_eq!(config.route, common::sound::Route::Left, "x is an alias for left");
    assert_eq!(config.master_volume, 0.8, "unset keys keep defaults");
    let _ = std::fs::remove_dir_all(dir);
}
