# Galvo Acoustic Audio & Synthesis Plan ("Singing Galvos")

> **Status (2026-11-05)**: the `galvo-audio` crate, its plugin and the DAC-thread hook are in
> place and tested headless. Off by default (`audio.galvo.enabled: false`) until tested on the
> Helios rig. See §6 for what's open.

---

## 1. Executive Summary & Physical Principles

Galvanometer optical scanners are fundamentally **rotational voice-coil motors**. They share identical electromagnetic physics with standard loudspeaker voice coils: a moving rotor magnet surrounded by stator drive coils (or vice versa), driven by transconductance amplifiers responding to an analog ILDA voltage signal.

When driven with alternating current in the audible band ($100\text{ Hz}$ to $6\text{ kHz}$), the electromagnetic Lorentz forces vibrate the rotor, bearings, and rigid aluminum scanner block, turning the laser projector chassis itself into an acoustic transducer without requiring any external speakers.

Two facts shape the architecture:

- **Audio has to follow the DAC clock.** The DAC plays one point per sample at `dac_pps`, and the
  DAC thread pads, truncates and repeats frames independently of the 60 Hz Bevy tick. Audio
  baked into the Bevy-side frame would be chopped and looped. So audio is rendered ahead in Bevy
  and **added per point in the DAC thread**, one sample per point.
- **The galvos are one audio output.** Speakers and galvos sit side by side behind one router
  (`audio.routing`), which decides per sound whether it plays on the speakers, on the galvos,
  on both mixed, or nowhere (§2.0).
- **Sounds are composed, not coded.** What the galvos play are the same `.sound.yaml`
  compositions the speakers play, built from plugin-registered sources and stages
  ([SOUND_COMPOSITION.md](SOUND_COMPOSITION.md)). This crate is only the galvo *output*.
- **Sound is one more reactive output.** Like visuals, sound is declared in data, started
  by world events and pipeline triggers, and what the galvos play is published back as signals,
  so visuals can react to it (see `docs/TRANSFORMER_PIPELINE.md`).

```
 WORLD                 common::audio            OUTPUTS
┌──────────────────┐   ┌──────────────────┐     ┌─ speakers (server-core) ── compositions @ 48 kHz (or wav)
│ PlaySoundEvent   │──▶│ AudioRouting     │──┬─▶│
│ pipeline triggers│──▶│ audio.routing    │  │  └─ galvo (galvo-audio) ──┐
└──────────────────┘   └────────┬─────────┘  │                           │
          ▲                     │ sound.<name> trigger (any output)      │
          └─────────────────────┘                                        ▼
 galvo-audio crate (the galvo output)                                      server-core
┌─────────────────────────────────────────────────────────────────┐   ┌────────────────────┐
│ SoundCatalog ──▶ SoundMixer @ DAC pps ──▶ BUS: Bevy → DAC queue ─┼──▶│ DAC thread         │
│ (.sound.yaml,    parallel voices,                                │   │ GUARD (per point)  │
│  common::sound)  live stream (when routed), ducking, limiter     │   │ ceiling, notch,    │
│                  params ◀── Signals                              │   │ thermal, clamp     │
│                          │                                       │   │ point.x/y += dx/dy │
└──────────────────────────┼───────────────────────────────────────┘   └────────────────────┘
                           ▼
        galvo.level / x / y / pitch / playing (signals), AudioScope → BasePath
```

---

## 2. Core Feature Sets

### 2.0. Audio Outputs and Routing
Galvo audio is one output next to the speakers. Every sound has a name (`SoundId::LaserHit` is
`laser_hit`); each output keeps its own library under those names (speakers: wav files in the
`SoundBank` when a file exists, otherwise both render the composition
`assets/sounds/<name>.sound.yaml`). The router in `common::audio` sends each sound to the outputs listed
for it, as one `RoutedSound` per output:

```yaml
audio:
  routing:
    default: [speakers]                 # sounds not listed below
    sounds:
      countdown_tick: [speakers, galvo] # both, mixed
      target_spawn: [galvo]             # scanners only
      alert: []                         # muted
    on_trigger:                         # pipeline trigger → sound
      snake.gem_eaten: pellet_eat
      hunter.lock_on: sci_fi_lock       # a composed two-voice sound
    stream: [speakers, galvo]           # live audio stream (receiver mode)
    max_parallel: 4                     # copies of one sound at once; the oldest stops
    parallel:
      laser_hit: 3
      countdown_tick: 1
```

* `PlaySoundEvent::new(SoundId::Alert).on([AudioOutput::Galvo])` overrides routing for one event.
* Every output can play every composed sound; an unknown name is skipped with a warning.
* Every routed sound raises the broadcast trigger `sound.<name>`, so pipelines react to sound
  whichever output plays it. `sound.*` keys in `on_trigger` are ignored, so nothing loops.
* Each output keeps its own master switch and volume (`audio.enabled` / `audio.volume`,
  `audio.galvo.enabled` / `audio.galvo.master_volume`).
* `PlaySound::named(..).on([AudioOutput::Galvo])` plays on the galvos only, bypassing routing;
  `StopSound` ends sounds that play until stopped (instruments).

**Parallel sounds.** Every request starts a new copy, on all its outputs at once, on top of
everything already playing, including the live stream:
* `max_parallel` (default 4, per sound in `parallel`) caps how many copies of one sound overlap
  on each output; the oldest copy stops. On the galvos it fades out over 5 ms instead of cutting.
* Galvo mixing: up to 32 tones at once, summed through a limiter (instant gain reduction, 50 ms
  recovery) rather than clipped, so stacked sounds get quieter instead of distorting.
* The live stream plays next to effects on both outputs. On the galvos it ducks to
  `audio.galvo.stream.duck` (default 40%) while other sounds play.

**Live stream.** `common::audio_stream::LiveAudioFrame` (from `NetworkMessage::AudioStreamFrame`)
reaches every output; each plays it when `audio.routing.stream` lists it:
* speakers: an endless Bevy audio source at 48 kHz stereo, fed from a 200 ms queue (older audio
  is dropped so latency can't grow), volume `audio.volume × audio.stream_volume`;
* galvo: resampled to the DAC point rate and mixed in, level `audio.galvo.stream.gain`.

### 2.1. Carrier Injection (Superimposed Audio Carrier / Micro-Wobble)
* **Concept**: Rather than distorting the shape, inject a high-frequency micro-oscillation into the DAC coordinates:
  $$X_{\text{out}}(t) = X_{\text{shape}}(t) + A \cdot x(t) \qquad Y_{\text{out}}(t) = Y_{\text{shape}}(t) + A \cdot y(t)$$
* **Visual vs. Acoustic Decoupling**:
  - Amplitude $A$ (`max_excursion_dac`, default 45) is at most 80 of the 4096 DAC units (≈2%).
  - Visually: lines stay crisp, at most a slight bloom.
  - Acoustically: sound pressure follows angular acceleration ($a = \omega^2 x$), so high
    frequencies are loud even at microscopic deflection.
* **Routes** (per tone, default `audio.galvo.route`): `x`, `y`, `both` (in phase), `orbit`
  (X = sine, Y = cosine: a micro-circle that shares heat between both coils). Two tones on `x`
  and `y` give true two-voice polyphony.

### 2.2. Two-Voice Duophony & Counterpoint (Galvo X + Galvo Y)
* **Mechanics**: Galvo X and Galvo Y are separate voice-coil motors with independent drivers,
  clamped into one aluminium block that mixes their sound.
* **As compositions**: chords and counterpoint are layers with `route: x` / `route: y`:
  - **Chords**: `power_chord` (440 + 660 Hz, 2:3), `major_third_chime` (5:4).
    `PlaySound::inline(spec)` plays one built in code.
  - **Counterpoint**: `sci_fi_lock`, X sweeps 800 → 2400 Hz while Y sweeps 2400 → 800 Hz.
  - **Beats**: `beat_warning`, 440 Hz on X against 447 Hz on Y throbs at 7 Hz.
  - **Visual Lissajous**: the beam traces the chord's Lissajous figure; an `AudioScope` in `xy`
    mode draws it large.

### 2.3. Notes, Octaves and Envelopes
* 12-TET notes (`Note::A.frequency(4)` = 440 Hz), MIDI conversion, just intervals, scale
  quantisation (`Scale::MajorPentatonic`).
* Waveforms: sine, triangle, soft square, rounded saw. Oscillators are phase-continuous across
  render blocks and sweeps, so tones never jump.
* Every tone runs through an ADSR envelope (default 5 ms attack, 8 ms release). Steps in coil
  current are what click.
* **Blanked audio**: with `play_when_blank: true` the galvos also sing on blank frames (between
  rounds, laser switched off), e.g. countdown ticks without light.

### 2.4. Sound Effects
Every `SoundId` has a composition in `assets/sounds/<name>.sound.yaml`
(`SoundId::LaserHit` → `laser_hit`), one commented layer per note; edit a file and the sound
changes live. See [SOUND_COMPOSITION.md](SOUND_COMPOSITION.md) for the format:

| Sound | Waveform | Frequencies | Duration |
| :--- | :--- | :--- | :--- |
| `laser_hit` | Exponential down-chirp | 2800 → 1100 Hz | 45 ms |
| `target_spawn` | Soft dual pip | C6 → E6 | 60 ms |
| `target_hit` | Crisp blip | A6 (1760 Hz) | 35 ms |
| `target_miss` | Low buzz (saw) | 160 Hz | 80 ms |
| `countdown_tick` | Percussive click | 2200 Hz | 15 ms |
| `countdown_go` | Rising arpeggio | C5 E5 G5 C6 | 160 ms |
| `game_over` | Falling line (triangle) | G5 E5 C5 G4 | 240 ms |
| `pellet_eat` / `snake_crash` | Chirps | 1400 → 2000 / 400 → 90 Hz | 25 / 120 ms |
| `calibration_*` | Bell / fanfare | C7 / C6 E6 G6 | 40 / 150 ms |
| `alert` | Warble (square) | 1800 ↔ 2400 Hz | 160 ms |

**Sources**: sounds routed to `galvo` (§2.0: `PlaySoundEvent`, `PlaySound`, and pipeline
triggers in `audio.routing.on_trigger`, so games get galvo sound without any code).

### 2.5. Harmonic Shapes (Geometric Pitch)
* A closed loop scanned repeatedly hums at $\text{pps} / \text{points}$
  (`loop_pitch`, `loop_points_for_pitch`: 68 points at 30 kpps ≈ 440 Hz).
* **Deferred to the frame tier.** The frame composer adds dwell and travel points and the DAC
  thread fixes the frame length at `dac_min_points`, so a per-entity resample doesn't set the
  pitch that's heard. Pitch-locking needs a loop-stable frame in `LaserFrameSet::Kinematics`
  (TRANSFORMER_PIPELINE P8). Carrier audio already plays any pitch on any shape.

### 2.6. Laser Theremin (Instrument)
* An instrument is just a sound whose layer plays until stopped and whose parameters follow
  signals: `assets/sounds/theremin.sound.yaml` (pitch ← `gamepad.lx` on the major pentatonic
  scale, volume ← `gamepad.ly`, shimmer ← `gamepad.rt`). Start it with
  `PlaySound::named("theremin")`, end it with `StopSound::named("theremin")`, or map triggers in
  `audio.routing.on_trigger` / `stop_on_trigger`.
* **Visuals as data**: `assets/pipelines/galvo/theremin.pipeline.yaml` is a ring that grows with
  `galvo.level` and ripples, spins and whitens with `galvo.pitch`. No drawing code.
* Inputs: any signal. Gamepad signals are still open in the pipeline (TRANSFORMER_PIPELINE P3).

### 2.7. Receiver Mode Audio Visualizer
* The server maps `NetworkMessage::AudioStreamFrame { samples, channels, sample_rate }` onto
  `common::audio_stream::LiveAudioFrame` (shared by all outputs, §2.0). On the galvo side it is
  resampled to the DAC rate and:
  * publishes `audio.level` and `audio.peak` for any pipeline,
  * fills the stream window for scopes,
  * with `audio.routing.stream: [galvo]`, plays through the galvos (the projector reproduces
    the soundtrack; `audio.galvo.stream.gain` sets its level).
* **Scopes**: an entity with `AudioScope { mode, input, size_m, window_ms, .. }` gets the figure
  written to its `BasePath` every frame, in metres in its local frame, so it goes through the
  scene transform, culling and frame safety. Modes: `xy` (L → X, R → Y vector scope), `ring`
  (radial waveform iris), `line` (horizontal ribbon). Inputs: `galvo` (what the galvos are
  playing) or `stream`. Style it with a pipeline, e.g. `UsePipeline::new("galvo/scope")`.

---

## 3. Hardware Safety & Galvo Health Guardrails

Enforced by `galvo_audio::guard::Guard` on every DAC point, after everything else. No config
value or sound can disable these:

| Rule | Implementation |
| :--- | :--- |
| **Amplitude clamp** | Offsets never exceed `ABSOLUTE_MAX_EXCURSION_DAC` = 80 DAC units (≈2%), whatever `max_excursion_dac` and `master_volume` say |
| **Frequency ceiling** | 2nd-order low-pass at `ceiling_hz`, never above `ABSOLUTE_MAX_HZ` = 5.5 kHz |
| **Resonance band** | Peaking cut of `notch_db` (default −10 dB) over `notch_min_hz..notch_max_hz` (default 3.2–4.2 kHz) |
| **Duty-cycle limiter** | After `thermal_secs` (5 s) of continuous loud drive the gain slides to `thermal_floor` (50%) over 2 s, and recovers when quiet |
| **No clicks** | Underruns decay the last sample (≈7 ms) instead of stepping to 0; enable/disable ramps over 10 ms |
| **Never blocks the DAC** | The DAC thread `try_lock`s the shared queue at most once per frame and keeps a one-frame spare |

---

## 4. Architecture

### 4.1 Crates

Galvo audio is one crate, `galvo-audio`. `laserlogic` (geometry and DAC frames) and `common`
(shared game types) know nothing about it. Games keep using `common::audio::PlaySoundEvent` or
pipeline triggers and need no new dependency.

| Crate | Module | Bevy? | Contents |
|---|---|---|---|
| `common` | `audio.rs` | yes | `SoundId::name`, `AudioOutput`, `AudioRouting` (`audio.routing`), `RoutedSound`, `AudioRoutingPlugin` |
| | `audio_stream.rs` | message | `LiveAudioFrame`, `Resampler` |
| | `sound/` | yes (plugin) | Composition: nodes, catalog, voices, mixer ([SOUND_COMPOSITION.md](SOUND_COMPOSITION.md)) |
| `galvo-audio` | `plugin.rs` | yes | `GalvoAudioPlugin`: a `SoundMixer` at the DAC rate, `galvo.*` signals, `AudioScope` |
| | `guard.rs` | no | Hard limits (§3) |
| | `bus.rs` | resource | `GalvoAudioBus` (Bevy side), `GalvoAudioOutput` (DAC thread side) |
| | `stream.rs`, `scope.rs` | no | `AudioWindow`, `Levels`; scope geometry |
| | `config.rs` | resource | `GalvoAudioConfig` (`audio.galvo`) |
| `server-core` | `lib.rs` | | Adds `GalvoAudioPlugin` |
| | `plugins/audio.rs` | | Speakers output: `RoutedSound`s for `speakers` from the wav `SoundBank` (parallel, capped), and `LiveStreamAudio`, an endless source playing the stream |
| | `plugins/projector.rs` | | DAC thread owns a `GalvoAudioOutput` and adds its offsets to every point |
| | `plugins/network.rs` | | `AudioStreamFrame` → `LiveAudioFrame` |
| `assets/pipelines/galvo/` | | | `scope`, `theremin` pipelines (embedded as fallbacks) |

The DAC hook is format-agnostic: the output returns `(dx, dy)` per point and server-core adds
them to `HeliosPoint`s with `apply_offset`.

### 4.2 Schedule

`AudioRoutingSet` runs after `PathPipelineSet::Signals`; `GalvoAudioSet::{Config, Cues, Render}`
run chained after it, all before `PathPipelineSet::React`:

- Trigger messages written by game plugins in `Signals` are heard in the same frame.
- `sound.<name>` triggers and `galvo.*` signals reach the same frame's reactions and stages.
- Audio uses `Time<Real>`, so sound and scopes keep going while the game is paused.

### 4.3 Timing

- The DAC owns the clock: `bus.output(dac_pps)` publishes the real point rate, and the
  galvo mixer renders at it.
- With a DAC attached the plugin keeps `latency_ms` (80 ms) of audio queued. Without one it
  renders in real time and discards the result, so signals, scopes and tests behave the same.
- When a DAC write doesn't go out (`Ok(false)`), the retry reuses the same offsets, so no audio
  is skipped.

### 4.4 Reactivity

| Direction | Signal / trigger | Meaning |
|---|---|---|
| out | `galvo.level` | Loudness of what the galvos play, 0..1 (fast attack, 80 ms release) |
| out | `galvo.x`, `galvo.y` | Per-galvo loudness |
| out | `galvo.pitch` | Pitch of the loudest voice, 50 Hz → 0 … 5.5 kHz → 1 (log), held after it ends |
| out | `galvo.playing` | 1 while any voice sounds |
| out | trigger `sound.<name>` | Raised by the router for every routed sound, any output; strength = volume |
| out | `audio.level`, `audio.peak` | Live stream input |
| in | `audio.routing.on_trigger` | Pipeline trigger → sound (`sound.*` triggers are ignored, so nothing loops) |
| in | sound parameters (`Param`) | Any signal, e.g. the theremin's pitch and volume |

---

## 5. Configuration

`audio.galvo` in `config/server.yaml` (every key optional, unknown keys are a startup error):

| Key | Default | |
|---|---|---|
| `enabled` | `false` | Master switch |
| `master_volume` | 0.8 | |
| `max_excursion_dac` | 45 | Capped at 80 |
| `route` | `orbit` | Route for sources that don't name one |
| `latency_ms` | 80 | Render-ahead |
| `play_when_blank` | `true` | |
| `ceiling_hz` | 5500 | Capped at 5500 |
| `notch_min_hz` / `notch_max_hz` / `notch_db` | 3200 / 4200 / −10 | |
| `thermal_secs` / `thermal_floor` | 5 / 0.5 | |
| `stream.gain` | 0.8 | Level of the live stream when routed to `galvo` |
| `stream.duck` | 0.4 | Stream level while other sounds play |

Which sounds reach the galvos is `audio.routing` (§2.0), not this section.

---

## 6. Roadmap

* [x] **Phase 1: DSP + compositions**: notes, envelope, oscillators; sounds as `.sound.yaml`
  compositions of plugin-registered sources and stages (shared with the speakers).
* [x] **Phase 2: DAC injection**: bus, guard, per-point offsets in the DAC thread.
* [x] **Phase 3: Bevy plugin**: routed sounds, signals out, config section.
* [x] **Routing**: speakers and galvos as outputs behind `audio.routing`, `sound.<name>` triggers.
* [x] **Live stream on every output** and **parallel sounds** (per-sound `max_parallel`,
  galvo limiter, stream ducking).
* [x] **Phase 5a: Instruments**: signal-bound sounds that play until stopped; theremin pipeline.
* [x] **Receiver mode**: stream resampling, `audio.*` signals, scopes, optional singing.
* [ ] **Bench on hardware**: enable on the Helios rig; tune `max_excursion_dac`, notch band and
  latency; check that beam quality stays sharp on the wall.
* [ ] **Gamepad signals** in the pipeline (TRANSFORMER_PIPELINE P3) so the theremin has inputs;
  then a theremin minigame that spawns the `galvo/theremin` entity and plays the `theremin` sound.
* [ ] **Phase 4: Harmonic shapes** in the frame tier (§2.5), after TRANSFORMER_PIPELINE P8.
* [ ] **Audio bands** (`audio.bass`, `audio.beat` …) from the stream, in an `AudioAnalysisPlugin`.
