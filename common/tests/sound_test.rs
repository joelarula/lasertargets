//! Sound composition: built-in files, layers in time and in parallel, nesting, stages, signal
//! binding, plugin nodes, the mixer (parallel voices, stream, ducking, limiter) and hot reload.

use bevy::prelude::*;
use common::audio::SoundId;
use common::audio_stream::StereoFrame;
use common::pipeline::Signals;
use common::sound::dsp::Note;
use common::sound::plugin::builtin_catalog;
use common::sound::*;
use serde::Deserialize;
use test_harness::TestApp;

const RATE: u32 = 30_000;

fn spec(yaml: &str) -> SoundSpec {
    SoundSpec::from_yaml(yaml).unwrap()
}

/// Renders a whole voice (or `max_ms` of it) in blocks of `block`
fn render(voice: &mut SoundVoice, max_ms: f32, block: usize, signals: &Signals) -> Vec<StereoFrame> {
    let total = (max_ms * RATE as f32 / 1000.0) as usize;
    let mut out = vec![[0.0; 2]; total];
    for chunk in out.chunks_mut(block) {
        voice.render(RATE as f32, Route::Both, signals, chunk);
    }
    out
}

fn active_ms(frames: &[StereoFrame]) -> (f32, f32) {
    let first = frames.iter().position(|f| f[0].abs() > 1e-4).unwrap_or(0);
    let last = frames.iter().rposition(|f| f[0].abs() > 1e-4).unwrap_or(0);
    (first as f32 * 1000.0 / RATE as f32, last as f32 * 1000.0 / RATE as f32)
}

fn crossings_hz(frames: &[StereoFrame], axis: usize) -> f32 {
    let n = frames.windows(2).filter(|w| (w[0][axis] < 0.0) != (w[1][axis] < 0.0)).count();
    n as f32 / 2.0 / (frames.len() as f32 / RATE as f32)
}

#[test]
fn every_sound_id_has_a_valid_builtin_composition() {
    let catalog = builtin_catalog();
    let signals = Signals::default();
    for sound in SoundId::ALL {
        let mut voice = catalog.voice(sound.name()).unwrap_or_else(|e| panic!("{e}"));
        let ms = voice.duration().expect("one-shot sounds end") * 1000.0;
        assert!(ms > 0.0 && ms <= 250.0, "{}: {ms} ms", sound.name());
        let out = render(&mut voice, ms + 5.0, 512, &signals);
        assert!(out.iter().any(|f| f[0].abs() > 0.1), "{} is audible", sound.name());
        assert!(voice.is_finished());
    }
    assert!(catalog.voice("theremin").unwrap().duration().is_none(), "the theremin plays until stopped");
}

#[test]
fn layers_play_at_their_times_whatever_the_block_size() {
    let catalog = builtin_catalog();
    let signals = Signals::default();
    let mut a = catalog.voice("countdown_go").unwrap();
    let whole = render(&mut a, 170.0, 5100, &signals);
    let mut b = catalog.voice("countdown_go").unwrap();
    let blocks = render(&mut b, 170.0, 97, &signals);
    assert!(whole.iter().zip(&blocks).all(|(x, y)| (x[0] - y[0]).abs() < 1e-4), "sample-accurate across blocks");
    let (start, end) = active_ms(&whole);
    assert!(start < 1.0 && (end - 160.0).abs() < 1.0, "four 40 ms notes: {start}..{end} ms");
    let fourth = &whole[(125 * RATE / 1000) as usize..(155 * RATE / 1000) as usize];
    assert!((crossings_hz(fourth, 0) - 1046.5).abs() < 60.0, "C6 at 120 ms");
}

#[test]
fn parallel_layers_mix_and_routes_split_channels() {
    let catalog = builtin_catalog();
    let mut voice = catalog.voice("power_chord").unwrap();
    let out = render(&mut voice, 50.0, 256, &Signals::default());
    assert!((crossings_hz(&out, 0) - 440.0).abs() < 25.0, "left / X = root");
    assert!((crossings_hz(&out, 1) - 660.0).abs() < 25.0, "right / Y = fifth, at the same time");
}

#[test]
fn repeat_nest_and_stages_compose() {
    let mut catalog = builtin_catalog();
    catalog
        .insert(spec(
            r#"
name: test/combo
layers:
  - source: { sound: target_hit }
  - at_ms: 100
    repeat: 2
    every_ms: 50
    source: { tone: { hz: 500, ms: 20 } }
stages:
  - echo: { ms: 30, feedback: 0.5 }
"#,
        ))
        .unwrap();
    let mut voice = catalog.voice("test/combo").unwrap();
    let duration = voice.duration().unwrap();
    assert!(duration > 0.22, "repeats (100 + 2×50 + 20 ms) plus the echo tail: {duration}");
    let out = render(&mut voice, duration * 1000.0 + 5.0, 400, &Signals::default());
    let at = |ms: f32| out[(ms * RATE as f32 / 1000.0) as usize][0].abs();
    let near = |ms: f32| (0..30).map(|i| at(ms + i as f32 * 0.3)).fold(0.0, f32::max);
    assert!(near(10.0) > 0.2, "nested target_hit at the start");
    assert!(near(205.0) > 0.2, "third repeat at 200 ms");
    assert!(near(235.0) > 0.01, "echo still sounding after the last tone");
    assert!(voice.is_finished());
}

#[test]
fn parameters_follow_signals() {
    let catalog = builtin_catalog();
    let mut signals = Signals::default();
    let mut voice = catalog.voice("theremin").unwrap();
    assert!(render(&mut voice, 50.0, 500, &signals).iter().all(|f| f[0].abs() < 1e-3), "gamepad.ly = 0 is silent");

    signals.set("gamepad.ly", 1.0);
    signals.set("gamepad.lx", 0.5);
    let out = render(&mut voice, 200.0, 500, &signals);
    assert!(out[3000..].iter().any(|f| f[0].abs() > 0.5), "loud once the gain signal is up");
    let hz = voice.pitch_hz().unwrap();
    assert!((hz - Note::C.frequency(5)).abs() < 1.0, "pitch 0.5 = C5 on the pentatonic scale ({hz})");

    voice.stop();
    render(&mut voice, 40.0, 500, &signals);
    assert!(voice.is_finished(), "stop fades out and ends");
}

#[test]
fn invalid_files_are_rejected_with_locations() {
    let mut catalog = builtin_catalog();
    let errors = catalog
        .insert(spec("name: bad\nlayers:\n  - source: { tone: { hz: 440, ms: 10, bogus: 1 } }\n    stages:\n      - wobble: {}\n"))
        .unwrap_err();
    let all = errors.join("\n");
    assert!(all.contains("layers[0].source.tone"), "{all}");
    assert!(all.contains("layers[0].stages[0].wobble") && all.contains("unknown stage"), "{all}");
    assert!(!catalog.contains("bad"));

    catalog.insert(spec("name: loop_a\nlayers:\n  - source: { sound: loop_b }\n")).unwrap();
    catalog.insert(spec("name: loop_b\nlayers:\n  - source: { sound: loop_a }\n")).unwrap();
    assert!(catalog.voice("loop_a").unwrap_err().contains("nested deeper"), "cycles are refused when played");
}

/// A plugin-provided stage: hard-inverts the signal
#[derive(Deserialize)]
struct Invert {}

impl SoundStage for Invert {
    fn process(&mut self, _ctx: &SoundCtx, buf: &mut [StereoFrame]) {
        buf.iter_mut().for_each(|f| *f = [-f[0], -f[1]]);
    }
}

#[test]
fn plugins_register_nodes_and_embed_sounds() {
    let mut t = TestApp::new();
    t.add_plugins(SoundPlugin);
    t.app_mut()
        .register_sound_stage::<Invert>("invert")
        .embed_sound("name: test/inverted\nlayers:\n  - source: { tone: { hz: 300, ms: 10 } }\n    stages:\n      - invert: {}\n");
    let catalog = t.resource::<SoundCatalog>();
    assert!(catalog.contains("laser_hit"), "built-ins embedded");
    let mut plain = catalog.voice_of(&spec("name: p\nlayers:\n  - source: { tone: { hz: 300, ms: 10 } }\n")).unwrap();
    let mut inverted = catalog.voice("test/inverted").unwrap();
    let (a, b) = (render(&mut plain, 10.0, 100, &Signals::default()), render(&mut inverted, 10.0, 100, &Signals::default()));
    assert!(a.iter().zip(&b).all(|(x, y)| (x[0] + y[0]).abs() < 1e-5), "the custom stage ran");
}

#[test]
fn mixer_plays_in_parallel_caps_copies_and_limits() {
    let catalog = builtin_catalog();
    let signals = Signals::default();
    let mut mixer = SoundMixer::new(RATE, Route::Left);
    for _ in 0..5 {
        mixer.play(&catalog, "target_hit", 1.0, 2).unwrap();
    }
    mixer.play(&catalog, "countdown_go", 1.0, 2).unwrap();
    assert_eq!(mixer.copies_of("target_hit"), 2, "newest two of five");
    assert_eq!(mixer.copies_of("countdown_go"), 1, "limits are per sound");

    let mut out = Vec::new();
    mixer.render(RATE as usize / 10, &signals, &mut out);
    assert!(out.iter().all(|f| f[0].abs() <= 1.0));
    let flat_runs = out.windows(3).filter(|w| w.iter().all(|f| f[0].abs() > 0.999)).count();
    assert_eq!(flat_runs, 0, "stacked sounds are limited, not clipped");
    let steps = out.windows(2).map(|w| (w[1][0] - w[0][0]).abs()).fold(0.0, f32::max);
    assert!(steps < 0.6, "replaced copies fade instead of cutting ({steps})");
}

#[test]
fn mixer_ducks_the_stream_under_sounds() {
    let catalog = builtin_catalog();
    let signals = Signals::default();
    let mut mixer = SoundMixer::new(RATE, Route::Left);
    mixer.set_stream_duck(0.25);
    mixer.push_stream(&vec![[0.5, 0.5]; RATE as usize / 4]);
    let mut alone = Vec::new();
    mixer.render(900, &signals, &mut alone);
    assert!((alone[800][1] - 0.5).abs() < 0.01, "stream at full level alone");

    let mut quiet = builtin_catalog();
    quiet.insert(spec("name: hum\nlayers:\n  - source: { tone: { hz: 1000, ms: 150, gain: 0.5, route: left } }\n")).unwrap();
    mixer.play(&quiet, "hum", 1.0, 4).unwrap();
    let mut together = Vec::new();
    mixer.render(3000, &signals, &mut together);
    assert!((together[2900][1] - 0.125).abs() < 0.02, "right carries only the ducked stream ({})", together[2900][1]);
    assert!(together[..2900].iter().any(|f| f[0].abs() > 0.4), "the sound plays on the left meanwhile");
    let _ = catalog;
}

#[test]
fn notes_scales_and_orbit_route() {
    use common::sound::dsp::{loop_points_for_pitch, Interval, Scale};
    assert!((Note::A.frequency(4) - 440.0).abs() < 0.01);
    assert!((Note::C.frequency(4) - 261.63).abs() < 0.1);
    assert!((Scale::MajorPentatonic.quantize(445.0) - 440.0).abs() < 1.0);
    // B4 is one semitone from C5 but two from A4: it snaps up across the octave
    assert!((Scale::MajorPentatonic.quantize(493.9) - Note::C.frequency(5)).abs() < 0.5);
    assert!((Interval::PerfectFifth.ratio() - 1.5).abs() < 1e-6);
    assert_eq!(loop_points_for_pitch(440.0, RATE), 68);

    // Orbit: left and right in quadrature, so on the galvos the figure is a circle
    let catalog = builtin_catalog();
    let mut voice = catalog.voice_of(&spec("name: o\nlayers:\n  - source: { tone: { hz: 500, ms: 100, route: orbit } }\n")).unwrap();
    let out = render(&mut voice, 100.0, 300, &Signals::default());
    let r2: Vec<f32> = out[1000..2000].iter().map(|f| f[0] * f[0] + f[1] * f[1]).collect();
    let (lo, hi) = r2.iter().fold((f32::MAX, 0.0_f32), |(lo, hi), &v| (lo.min(v), hi.max(v)));
    assert!(hi - lo < 0.01, "orbit radius steady ({lo}..{hi})");
}

#[test]
fn shipped_sound_files_compile_and_match_their_names() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../assets/sounds");
    let mut catalog = builtin_catalog();
    let mut count = 0;
    for entry in std::fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        let Some(file) = path.file_name().and_then(|f| f.to_str()) else { continue };
        let Some(stem) = file.strip_suffix(".sound.yaml") else { continue };
        let spec = SoundSpec::from_yaml(&std::fs::read_to_string(&path).unwrap()).unwrap_or_else(|e| panic!("{file}: {e}"));
        assert_eq!(spec.name, stem, "{file}: `name` must equal the file name");
        catalog.insert(spec).unwrap_or_else(|e| panic!("{file}: {}", e.join("; ")));
        count += 1;
    }
    assert!(count >= SoundId::ALL.len(), "found {count} sound files");
}
