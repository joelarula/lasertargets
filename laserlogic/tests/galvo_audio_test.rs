use common::audio::{GalvoAudioConfig, GalvoCarrierMode, GalvoWaveform, SoundId};
use laserlogic::galvo_audio::*;
use laserlogic::LaserPoint;

#[test]
fn test_note_frequencies_and_octaves() {
    let a4 = Note::A.frequency(4);
    assert!((a4 - 440.0).abs() < 0.01, "A4 should be 440.0 Hz, got {}", a4);

    let a3 = Note::A.frequency(3);
    assert!((a3 - 220.0).abs() < 0.01, "A3 should be 220.0 Hz, got {}", a3);

    let a5 = Note::A.frequency(5);
    assert!((a5 - 880.0).abs() < 0.01, "A5 should be 880.0 Hz, got {}", a5);

    // Exact octave doubling
    assert_eq!((a5 / a4).round(), 2.0);
    assert_eq!((a4 / a3).round(), 2.0);

    let c4 = Note::C.frequency(4);
    assert!((c4 - 261.63).abs() < 0.1, "C4 should be ~261.63 Hz, got {}", c4);
}

#[test]
fn test_scale_quantization() {
    let quant = ScaleQuantization::MajorPentatonic;
    // Quantize 445 Hz -> should quantize to A4 (440 Hz)
    let q = quant.quantize(445.0);
    assert!((q - 440.0).abs() < 1.0, "Expected 440.0 Hz, got {}", q);
}

#[test]
fn test_synth_tone_and_envelope() {
    let sample_rate = 30000;
    let duration = 0.05; // 50ms
    let samples = generate_tone(GalvoWaveform::Sine, 440.0, duration, sample_rate, 1.0, None);

    assert_eq!(samples.len(), (duration * sample_rate as f32) as usize);

    // Verify all samples are within [-1.0, 1.0]
    for s in &samples {
        assert!(*s >= -1.0 && *s <= 1.0);
    }

    // Verify ADSR soft ramp: first sample should start near zero
    assert!(samples[0].abs() < 0.05, "First sample should be soft ramped near 0, got {}", samples[0]);
    // Last sample should finish near zero due to release phase
    let last = *samples.last().unwrap();
    assert!(last.abs() < 0.15, "Last sample should be soft ramped near 0, got {}", last);
}

#[test]
fn test_sound_fx_generation() {
    let sample_rate = 30000;
    let sounds = [
        SoundId::LaserHit,
        SoundId::TargetHit,
        SoundId::TargetSpawn,
        SoundId::TargetMiss,
        SoundId::CountdownTick,
        SoundId::CountdownGo,
        SoundId::Alert,
    ];

    for sound in sounds {
        let samples = generate_sound_fx(sound, sample_rate, 1.0);
        assert!(!samples.is_empty(), "SFX {:?} produced no samples", sound);
        for s in &samples {
            assert!(*s >= -1.0 && *s <= 1.0, "Sample out of bounds for {:?}", sound);
        }
    }
}

#[test]
fn test_carrier_modulator_safety_clamp() {
    let mut config = GalvoAudioConfig::default();
    config.max_amplitude_dac = 200; // Intentionally set above safe clamp
    config.carrier_mode = GalvoCarrierMode::QuadratureXY;

    let mut modulator = GalvoCarrierModulator::new(config);

    // Queue strong samples
    modulator.queue_samples(vec![1.0, -1.0, 1.0, -1.0]);

    let mut points = vec![
        LaserPoint::new(2048, 2048, 255, 255, 255, 255),
        LaserPoint::new(2048, 2048, 255, 255, 255, 255),
    ];

    modulator.modulate_laser_points(&mut points, 30000);

    for pt in &points {
        let dx = (pt.x as i32 - 2048).abs();
        let dy = (pt.y as i32 - 2048).abs();
        assert!(
            dx <= ABSOLUTE_MAX_EXCURSION_DAC as i32,
            "X excursion {} exceeded safe clamp {}",
            dx,
            ABSOLUTE_MAX_EXCURSION_DAC
        );
        assert!(
            dy <= ABSOLUTE_MAX_EXCURSION_DAC as i32,
            "Y excursion {} exceeded safe clamp {}",
            dy,
            ABSOLUTE_MAX_EXCURSION_DAC
        );
    }
}

#[test]
fn test_blanked_audio_generation() {
    let mut modulator = GalvoCarrierModulator::default();
    modulator.queue_samples(vec![0.5; 50]);

    let frame = modulator.generate_blanked_audio_frame(50, 2048, 2048, 30000);
    assert_eq!(frame.len(), 50);

    for pt in &frame {
        assert!(pt.is_blanked(), "Blanked audio points must have laser turned off");
    }
}

#[test]
fn test_harmonic_shapes_resampling() {
    // 4 points forming a square in DAC space
    let square = vec![
        LaserPoint::new(1000, 1000, 255, 0, 0, 255),
        LaserPoint::new(3000, 1000, 255, 0, 0, 255),
        LaserPoint::new(3000, 3000, 255, 0, 0, 255),
        LaserPoint::new(1000, 3000, 255, 0, 0, 255),
    ];

    // Resample to 68 points (which corresponds to ~441 Hz at 30 kpps)
    let target_points = points_for_pitch(440.0, 30000);
    let resampled = resample_closed_loop(&square, target_points);

    assert_eq!(resampled.len(), target_points);
    let calculated_pitch = pitch_for_points(resampled.len(), 30000);
    assert!((calculated_pitch - 440.0).abs() < 5.0);

    // All points must be within original bounds
    for pt in &resampled {
        assert!(pt.x >= 999 && pt.x <= 3001);
        assert!(pt.y >= 999 && pt.y <= 3001);
    }
}

#[test]
fn test_laser_theremin() {
    let mut theremin = LaserTheremin::default();
    theremin.set_input(0.0, 0.0); // Center

    let freq = theremin.current_frequency();
    assert!(freq >= 130.0 && freq <= 2100.0);

    let vol = theremin.current_volume();
    assert!((vol - 0.5).abs() < 0.01);

    let audio = theremin.generate_audio_samples(0.01, 30000);
    assert!(!audio.is_empty());

    let visual = theremin.generate_visual_points(40);
    assert!(!visual.is_empty());
}

#[test]
fn test_duophonic_chord_generation() {
    let (samples_x, samples_y) = DuophonicSynth::generate_interval_chord(
        440.0,
        MusicalInterval::PerfectFifth,
        0.05,
        30000,
        1.0,
    );

    assert_eq!(samples_x.len(), samples_y.len());
    assert!(!samples_x.is_empty());

    for s in &samples_x {
        assert!(*s >= -1.0 && *s <= 1.0);
    }
    for s in &samples_y {
        assert!(*s >= -1.0 && *s <= 1.0);
    }
}

#[test]
fn test_duophonic_counterpoint_sfx() {
    let sfx_list = [
        DuoCounterpointSfx::SciFiLaserLock,
        DuoCounterpointSfx::PowerChordHit,
        DuoCounterpointSfx::MajorThirdChime,
        DuoCounterpointSfx::DissonantBeatWarning,
    ];

    for sfx in sfx_list {
        let (sx, sy) = DuophonicSynth::generate_counterpoint_sfx(sfx, 30000, 1.0);
        assert!(!sx.is_empty(), "Voice X empty for {:?}", sfx);
        assert_eq!(sx.len(), sy.len(), "Voice X and Y lengths mismatch for {:?}", sfx);
    }
}

#[test]
fn test_carrier_duophonic_modulation() {
    let mut config = GalvoAudioConfig::default();
    config.carrier_mode = GalvoCarrierMode::Duophonic;
    config.max_amplitude_dac = 40;

    let mut modulator = GalvoCarrierModulator::new(config);

    // Queue different signals for Voice X and Voice Y
    let (sx, sy) = DuophonicSynth::generate_chord(
        440.0,
        GalvoWaveform::Sine,
        880.0,
        GalvoWaveform::Triangle,
        0.02,
        30000,
        1.0,
    );
    modulator.queue_duo_samples(sx, sy);

    let mut points = vec![LaserPoint::new(2048, 2048, 255, 255, 255, 255); 50];
    modulator.modulate_laser_points(&mut points, 30000);

    // Confirm both X and Y received distinct micro-deflections
    let has_x_deflection = points.iter().any(|p| p.x != 2048);
    let has_y_deflection = points.iter().any(|p| p.y != 2048);
    assert!(has_x_deflection, "Galvo X should be modulated by Voice 1");
    assert!(has_y_deflection, "Galvo Y should be modulated by Voice 2");
}


