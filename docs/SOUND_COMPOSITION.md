# Sound Composition

How sounds are built, played and routed. Sound follows the same architecture as the
[reactive laser pipeline](TRANSFORMER_PIPELINE.md): plugins register building blocks, data files
compose them, parameters follow live signals, and outputs render the result.

## 1. Goals

- **Composed, not coded.** A sound is a `.sound.yaml` file: layers of sources and stages. Game
  code names a sound; it never synthesises one.
- **Pluggable.** New sources and stages come from one registration line in any plugin
  (`register_sound_source`, `register_sound_stage`). No central enum or match changes.
- **Composable.** Layers play in parallel, each from its own start time; layers can repeat; a
  layer can be another whole sound (`sound: target_hit`). Stages apply per layer or to the mix.
- **Reactive.** Any numeric parameter is a pipeline `Param`: a constant or a signal mapped onto a
  range (`{ signal: gamepad.lx, range: [0, 1], smooth_ms: 20 }`). Instruments are just sounds whose
  parameters follow signals and that play until stopped. Every routed sound raises the trigger
  `sound.<name>`, so visuals react to sound.
- **Output-independent.** The speakers (48 kHz) and the galvos (DAC point rate) render the same
  files through their own mixer. Routing decides which output plays what.
- **Hot-reloadable.** Editing `assets/sounds/<name>.sound.yaml` changes the sound live; a broken
  file keeps the previous version.

## 2. Architecture

```
 PlaySoundEvent / PlaySound / StopSound / pipeline triggers
                    │
                    ▼
          AudioRouting (audio.routing) ──▶ trigger sound.<name>
                    │ RoutedSound per output
        ┌───────────┴─────────────┐
        ▼                         ▼
   speakers output           galvo output
   (server-core)             (galvo-audio)
   wav file if recorded,     SoundMixer @ DAC pps,
   else SoundMixer @ 48 kHz  default route orbit
        │                         │
        └──── SoundCatalog.voice(name) ◀── .sound.yaml (embedded + assets, hot reload)
                    │
   voice: layers (parallel, timed) ─▶ layer stages ─▶ mix ─▶ mix stages ─▶ gain ─▶ mixer
   mixer: parallel voices + live stream, per-sound max_parallel, ducking, limiter
```

| Layer | Unit | Declared in |
|---|---|---|
| Nodes | `SoundSource`, `SoundStage` | Plugins (`common::sound::sources`, `stages`, or any crate) |
| Composition | `SoundSpec` → `SoundVoice` | `assets/sounds/<name>.sound.yaml` |
| Routing | `RoutedSound` | `audio.routing` in `server.yaml` |
| Mixing | `SoundMixer`, one per output | Output plugins |

## 3. File Format

```yaml
# assets/sounds/countdown_go.sound.yaml
name: countdown_go                     # must equal the file name
description: Rising arpeggio C5 E5 G5 C6

layers:                                # all layers play in parallel, each from at_ms
  - source: { tone: { hz: 523.25, ms: 40 } }
    stages:
      - envelope: { attack_ms: 5, release_ms: 8 }
  - at_ms: 40
    source: { tone: { hz: 659.25, ms: 40 } }
  - at_ms: 80
    repeat: 2                          # two more copies…
    every_ms: 40                       # …40 ms apart
    source: { sound: target_hit }      # a whole other sound, nested

stages:                                # over the mix of all layers
  - echo: { ms: 60, feedback: 0.3 }

gain: { signal: game.intensity, range: [0.6, 1.0] }
```

Like pipeline files, every layer and stage in a shipped file has a plain-language comment
above it saying what it does and what drives it.

A sound ends when its last layer (and any stage tail, such as an echo) has died away. A layer
whose source has no `ms` plays until the sound is stopped (`StopSound`, or `stop_on_trigger`),
which fades it out over 30 ms.

### 3.1 Built-in sources

| Source | Parameters | Notes |
|---|---|---|
| `tone` | `hz` *or* `pitch` (0..1 across `range_hz`, even in pitch), `scale`, `ms`, `wave`, `gain`, `route` | Signal-driven pitch glides between blocks |
| `chirp` | `from_hz`, `to_hz`, `ms`, `curve` (`exponential`/`linear`), `wave`, `gain`, `route` | Sweeps |
| `noise` | `ms`, `gain`, `route` | White noise; shape it with `lowpass` |
| `silence` | `ms` | A rest in a sequence |
| `sound` | `<name>` | Nests another composed sound (cycles are refused) |

Waves: `sine`, `triangle`, `square` (soft), `saw` (rounded). Scales: `continuous`, `chromatic`,
`major`, `minor`, `major_pentatonic`, `minor_pentatonic`.

**Routes** place a mono source in the stereo frame: `left` (alias `x`), `right` (`y`), `both`,
`orbit` (left = sine, right = cosine). On the galvos left drives the X mirror and right the Y
mirror; `orbit` traces a micro-circle that shares coil heat. A source without `route` uses its
output's default: `orbit` on the galvos (`audio.galvo.route`), `both` on the speakers.

### 3.2 Built-in stages

| Stage | Parameters | |
|---|---|---|
| `envelope` | `attack_ms`, `decay_ms`, `sustain`, `release_ms` | Over the layer's length (or the sound's, as a mix stage) |
| `gain` | `amount` | Ramped per block, no zipper noise |
| `pan` | `position` -1..1 | Constant power |
| `tremolo` | `rate_hz`, `depth` | |
| `lowpass` | `cutoff_hz` | Keeps its state when the cutoff moves |
| `echo` | `ms`, `feedback`, `mix` | Keeps the voice alive until the repeats fade (−60 dB) |

### 3.3 Instruments

An instrument is a sound with an endless layer and signal-bound parameters:

```yaml
# assets/sounds/theremin.sound.yaml
name: theremin
layers:
  - source:
      tone:
        pitch: { signal: gamepad.lx, smooth_ms: 20 }
        range_hz: [130.81, 2093]
        scale: major_pentatonic
gain: { signal: gamepad.ly, range: [0.0, 0.8], smooth_ms: 20 }
```

Start it with `PlaySound::named("theremin")` (or a trigger in `audio.routing.on_trigger`), stop
it with `StopSound::named("theremin")` (or `stop_on_trigger`).

## 4. Plugin API

```rust
use common::sound::{SoundAppExt, SoundCtx, SoundStage};

#[derive(Deserialize)]
struct Bitcrush { bits: u8 }

impl SoundStage for Bitcrush {
    fn process(&mut self, _ctx: &SoundCtx, buf: &mut [[f32; 2]]) {
        let steps = (1u32 << self.bits) as f32;
        for f in buf { *f = f.map(|v| (v * steps).round() / steps); }
    }
}

app.register_sound_stage::<Bitcrush>("bitcrush")
   .embed_sound(include_str!("../../../assets/sounds/hunter/pop.sound.yaml"));
```

- `SoundSource::render` overwrites its block; `SoundStage::process` changes it in place. Both are
  pure (no ECS access) and stateful per voice; they get time, block length, the layer's duration,
  the default route and the global `Signals` through `SoundCtx`. `ctx.param(&mut p)` reads a
  `Param` once per block.
- `duration_secs` (sources) and `tail_secs` (stages) tell the voice when it is done.
- `embed_sound` compiles a file into the binary as a fallback, and also loads and watches
  `assets/sounds/<name>.sound.yaml`. It panics on an invalid file, so register custom nodes first.
- Unknown sources, stages and fields are errors that name the file and location
  (`laser_hit: layers[0].stages[1].ech0: unknown stage (known: echo, envelope, …)`).

## 5. Playing, Routing and Parallel Sounds

| Message | Purpose |
|---|---|
| `PlaySoundEvent::new(SoundId::LaserHit)` | Typed built-in sounds |
| `PlaySound::named("sci_fi_lock")`, `PlaySound::inline(spec)` | Any sound by name, or one built in code |
| `.on([AudioOutput::Galvo])` | Override routing for one request |
| `StopSound::named("theremin")` | Fade out every copy (on every output unless `outputs` is set) |

`audio.routing` (`common::audio::AudioRouting`) decides outputs per sound, maps pipeline
triggers to sounds (`on_trigger`, `stop_on_trigger`), routes the live stream (`stream`), and caps
overlapping copies (`max_parallel`, per sound in `parallel`). Sounds overlap freely, on all their
outputs at once and on top of the live stream. Each output's `SoundMixer`:

- plays up to 32 voices in parallel; past a sound's `max_parallel` the oldest copy fades out;
- ducks the live stream under sounds (`audio.galvo.stream.duck` on the galvos);
- limits the sum instead of clipping it.

The speakers play a recorded file instead of the composition when
`assets/sounds/<name>.wav|ogg|flac` exists.

## 6. Code Layout

| Crate | Module | Contents |
|---|---|---|
| `common` | `sound/node.rs` | `SoundSource`, `SoundStage`, `SoundCtx`, `Route` |
| | `sound/spec.rs` | `SoundSpec`, `LayerSpec` |
| | `sound/catalog.rs` | `SoundCatalog`: node factories, sounds by name, voice building, nesting |
| | `sound/voice.rs` | `SoundVoice`: sample-accurate layers, stages, gain, stop fade |
| | `sound/mixer.rs` | `SoundMixer`: parallel voices, stream, ducking, limiter |
| | `sound/sources.rs`, `sound/stages.rs` | Built-in nodes |
| | `sound/dsp/` | Notes, scales, envelope, oscillator, biquad |
| | `sound/plugin.rs` | `SoundPlugin`, `SoundAppExt`, `.sound.yaml` loader, hot reload |
| | `audio.rs`, `audio_stream.rs` | Routing, messages, live stream |
| `server-core` | `plugins/audio.rs` | Speakers output |
| `galvo-audio` | | Galvo output: mixer at DAC rate, bus, guard, scope, `galvo.*` signals |
| `assets/sounds/` | `*.sound.yaml` | Built-in sounds (all `SoundId`s, two-voice sounds, theremin) |
