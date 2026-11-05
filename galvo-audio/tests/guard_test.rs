//! Plain-Rust tests of the galvo output: guard (hard limits), DAC bus and scope geometry.
//! Composition, envelopes and the mixer are tested in `common/tests/sound_test.rs`.

use common::audio_stream::StereoFrame as Frame;
use galvo_audio::bus::GalvoAudioBus;
use galvo_audio::guard::Guard;
use galvo_audio::scope::scope_path;
use galvo_audio::*;

const RATE: u32 = 30_000;

fn enabled() -> GuardSettings {
    GuardSettings { enabled: true, ..Default::default() }
}

/// Amplitude of a pure tone after the guard, once filters and the enable ramp have settled
fn guarded_amplitude(settings: GuardSettings, hz: f32) -> i16 {
    let mut guard = Guard::new(settings, RATE);
    let mut peak = 0;
    for i in 0..RATE / 5 {
        let s = (std::f32::consts::TAU * hz * i as f32 / RATE as f32).sin();
        let (dx, _) = guard.process(Some([s, 0.0]));
        if i > RATE / 10 {
            peak = peak.max(dx.abs());
        }
    }
    peak
}

#[test]
fn guard_never_exceeds_the_hard_excursion_cap() {
    let wild = GuardSettings { enabled: true, master_volume: 5.0, max_excursion_dac: 4000, ..Default::default() };
    let mut guard = Guard::new(wild, RATE);
    for i in 0..RATE {
        let s = if i % 2 == 0 { 1.0 } else { -1.0 }; // worst case: full-scale square at Nyquist
        let (dx, dy) = guard.process(Some([s * 3.0, -s]));
        assert!(dx.unsigned_abs() <= ABSOLUTE_MAX_EXCURSION_DAC && dy.unsigned_abs() <= ABSOLUTE_MAX_EXCURSION_DAC);
    }
}

#[test]
fn guard_cuts_above_the_ceiling_and_in_the_resonance_band() {
    let pass = guarded_amplitude(enabled(), 1000.0);
    let resonance = guarded_amplitude(enabled(), 3666.0);
    let above = guarded_amplitude(enabled(), 9000.0);
    assert!(pass >= 35, "1 kHz passes at ~full amplitude ({pass})");
    assert!((resonance as f32) < pass as f32 * 0.5, "resonance cut ({resonance} vs {pass})");
    assert!((above as f32) < pass as f32 * 0.4, "9 kHz is filtered ({above} vs {pass})");
}

#[test]
fn guard_tapers_long_loud_drones() {
    let mut guard = Guard::new(enabled(), RATE);
    for i in 0..RATE * 9 {
        let s = (std::f32::consts::TAU * 800.0 * i as f32 / RATE as f32).sin();
        guard.process(Some([s, 0.0]));
        if i == RATE * 4 {
            assert_eq!(guard.thermal_gain(), 1.0, "no taper within thermal_secs");
        }
    }
    assert!((guard.thermal_gain() - 0.5).abs() < 0.01, "settles at thermal_floor ({})", guard.thermal_gain());
    for _ in 0..RATE * 10 {
        guard.process(Some([0.0, 0.0]));
    }
    assert_eq!(guard.thermal_gain(), 1.0, "recovers when quiet");
}

#[test]
fn guard_fades_on_underrun_and_on_disable() {
    let mut guard = Guard::new(enabled(), RATE);
    for _ in 0..3000 {
        guard.process(Some([1.0, 1.0]));
    }
    let mut last = guard.process(None).0;
    assert!(last > 20);
    for _ in 0..3000 {
        let (dx, _) = guard.process(None);
        assert!((dx - last).abs() <= 3, "no step on underrun ({last} → {dx})");
        last = dx;
    }
    assert_eq!(last, 0, "decays to rest");

    let mut guard = Guard::new(enabled(), RATE);
    for _ in 0..3000 {
        guard.process(Some([1.0, 0.0]));
    }
    guard.configure(GuardSettings::default(), RATE); // disabled
    let first = guard.process(Some([1.0, 0.0])).0;
    assert!(first > 20, "disabling ramps instead of cutting");
    let after: Vec<i16> = (0..600).map(|_| guard.process(Some([1.0, 0.0])).0).collect();
    assert_eq!(*after.last().unwrap(), 0, "silent 20 ms after disabling");
}

#[test]
fn bus_hands_audio_to_the_dac_thread_at_its_rate() {
    let bus = GalvoAudioBus::new(enabled());
    assert!(!bus.has_consumer());
    let mut output = bus.output(40_000);
    assert!(bus.has_consumer());
    assert_eq!(bus.sample_rate(), 40_000, "the DAC owns the clock");

    bus.push(&vec![[1.0, 0.0]; 4000]);
    for _ in 0..3 {
        output.render(1000);
    }
    let offsets = output.render(1000).to_vec();
    assert_eq!(offsets.len(), 1000);
    assert!(offsets.iter().any(|o| o.0 != 0), "samples arrive as offsets");
    assert!(bus.queued() < 4000, "consumer drained the queue");

    drop(output);
    assert!(!bus.has_consumer());
    assert_eq!(apply_offset(4090, 40, 4095), 4095);
    assert_eq!(apply_offset(10, -40, 4095), 0);
}

#[test]
fn scope_draws_in_metres_within_its_size() {
    let frames: Vec<Frame> = (0..600).map(|i| {
        let a = i as f32 / 600.0 * std::f32::consts::TAU * 3.0;
        [a.sin(), a.cos()]
    }).collect();
    for mode in [ScopeMode::Xy, ScopeMode::Ring, ScopeMode::Line] {
        let path = scope_path(&frames, mode, 0.4, 200, 1.0, [0, 255, 0]);
        let points = &path.segments[0].points;
        assert!(points.len() >= 100 && points.len() <= 201, "{mode:?}: {} points", points.len());
        assert!(points.iter().all(|p| p.x.abs() <= 0.2001 && p.y.abs() <= 0.2001), "{mode:?} inside 0.4 m");
    }
    let ring = scope_path(&frames, ScopeMode::Ring, 0.4, 200, 1.0, [0, 255, 0]);
    assert_eq!(ring.segments[0].points.first(), ring.segments[0].points.last(), "ring is closed");
    assert!(scope_path(&[], ScopeMode::Xy, 0.4, 200, 1.0, [0, 0, 0]).segments.is_empty());
}
