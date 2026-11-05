# Galvo Acoustic Audio & Synthesis Plan ("Singing Galvos")

> **Schedule & Priority**: **Post-Camera Pipeline**  
> Implementation begins after camera stream capture, homography calibration, and high-speed laser hit tracking are complete and stabilized.

---

## 1. Executive Summary & Physical Principles

Galvanometer optical scanners are fundamentally **rotational voice-coil motors**. They share identical electromagnetic physics with standard loudspeaker voice coils: a moving rotor magnet surrounded by stator drive coils (or vice versa), driven by transconductance amplifiers responding to an analog ILDA voltage signal.

When driven with alternating current in the audible band ($100\text{ Hz}$ to $6\text{ kHz}$), the electromagnetic Lorentz forces vibrate the rotor, bearings, and rigid aluminum scanner block, turning the laser projector chassis itself into an acoustic transducer without requiring any external speakers.

```
                  ┌──────────────────────────────────────────┐
                  │          Bevy Game Logic / Events        │
                  │   PlaySoundEvent(LaserHit, CountdownGo)  │
                  └─────────────────────┬────────────────────┘
                                        │
                                        ▼
                  ┌──────────────────────────────────────────┐
                  │       Galvo Audio Synthesis Engine       │
                  │  * Note generator (12-TET frequencies)   │
                  │  * Sound FX chirp synthesis              │
                  │  * ADSR soft envelopes (anti-pop)        │
                  └─────────────────────┬────────────────────┘
                                        │
               ┌────────────────────────┴────────────────────────┐
               ▼                                                 ▼
┌───────────────────────────────┐               ┌────────────────────────────────┐
│   Carrier Injection Stage     │               │    Pure Acoustic / Blanked     │
│   Superimposes micro-wobble   │               │   Plays chiptune / alerts when │
│   (0.5%–1.5%) on active shape │               │   laser diode is blanked (RGB=0│
└──────────────┬────────────────┘               └────────────────┬───────────────┘
               │                                                 │
               └────────────────────────┬────────────────────────┘
                                        ▼
                        ┌────────────────────────────────┐
                        │      Helios DAC (30–40 kpps)   │
                        │    Differential Galvo Output   │
                        └───────────────┬────────────────┘
                                        ▼
                        ┌────────────────────────────────┐
                        │  Physical Galvo Scanner Block  │
                        │    Acoustic Sound Output       │
                        └────────────────────────────────┘
```

---

## 2. Core Feature Sets

### 2.1. Carrier Injection (Superimposed Audio Carrier / Micro-Wobble)
* **Concept**: Rather than distorting the shape, inject a high-frequency micro-oscillation into the DAC coordinates:
  $$X_{\text{out}}(t) = X_{\text{shape}}(t) + A \cdot \sin(2\pi f t)$$
  $$Y_{\text{out}}(t) = Y_{\text{shape}}(t) + A \cdot \cos(2\pi f t)$$
* **Visual vs. Acoustic Decoupling**:
  - Amplitude $A$ is held to **$0.5\% - 1.5\%$** of full DAC scale ($[-32768, 32767]$).
  - Visually: The projected target lines remain crisp and clear, showing at most a slight soft beam bloom or subtle thickening.
  - Acoustically: Because acoustic pressure is proportional to angular acceleration ($a = \omega^2 x$), high frequencies generate loud, clear, punchy sound even at microscopic physical deflections.
* **Selectable Modes**:
  - `SingleAxisX` / `SingleAxisY`: Vibrates one mirror coil.
  - `QuadratureXY`: Circular micro-orbit distributing heat evenly between both X and Y coil drivers.
  - `Duophonic`: True 2-voice polyphony where Galvo X reproduces Voice 1 and Galvo Y reproduces Voice 2 independently.

### 2.2. Two-Voice Duophony & Counterpoint (Galvo X + Galvo Y)
* **Mechanics**: Galvo X and Galvo Y are two separate physical voice-coil motors with independent driver amplifiers, clamped into a shared aluminum block that acoustically mixes their sound pressure waves.
* **Duophonic Modes**:
  - **Harmonic Chords**: Power chords (Root + 5th in $2:3$ ratio), Major/Minor Thirds ($5:4$, $6:5$), and Octaves ($1:2$).
  - **Counterpoint Sound Effects**: Cross-frequency sweeps (Galvo X chirps up while Galvo Y chirps down for intense sci-fi effects).
  - **Acoustic Beat Pulses**: Microtonal detuning (e.g., $440\text{ Hz}$ on X and $447\text{ Hz}$ on Y) produces a $7\text{ Hz}$ physical throbbing warning rumble without software LFOs.
  - **Visual Lissajous Harmonics**: Visual beam traces the exact geometric Lissajous shape of the acoustic chord.

### 2.3. Playing Notes & Octaves (Galvo Chiptune Synth)
* **Standard Equal Temperament (12-TET)**:
  - Frequency table spanning Octaves 2 through 7 ($65.4\text{ Hz}$ to $2093\text{ Hz}$).
  - Clean tone generation: Sine, Triangle, and Softened Square waves.
* **Thermal & Inductive Soft Envelope (ADSR)**:
  - Instantaneous step changes cause galvo drivers to spike current and produce harsh, mechanical clicks.
  - Smooth Attack ($2\text{–}5\text{ ms}$) and Release envelopes ramp current smoothly, yielding pure musical tones.
* **Blanked Audio Mode (Silent Projector / Piezo Buzzer Emulation)**:
  - When the laser diodes are blanked ($R=0, G=0, B=0$) between game rounds or during standby, the projector can still play countdown chimes, standby clicks, or celebratory victory fanfare without firing laser light.

### 2.3. Galvo Sound Effects Engine (Bevy ECS Integration)
Plugs directly into the existing `common::audio::PlaySoundEvent` dispatch pipeline:

| Sound Effect | Galvo Acoustic Waveform | Frequency Profile | Duration |
| :--- | :--- | :--- | :--- |
| `LaserHit` | Fast logarithmic down-chirp | $2800\text{ Hz} \to 1200\text{ Hz}$ | $45\text{ ms}$ |
| `TargetSpawn` | Soft dual-pip | $1046\text{ Hz} \text{ (C6)} \to 1318\text{ Hz} \text{ (E6)}$ | $60\text{ ms}$ |
| `TargetHit` | Crisp confirmed blip | $1760\text{ Hz} \text{ (A6)}$ | $40\text{ ms}$ |
| `TargetMiss` | Low dissonance buzz | $160\text{ Hz} \text{ sawtooth}$ | $90\text{ ms}$ |
| `CountdownTick` | High percussive click | $2200\text{ Hz} \text{ pulse}$ | $15\text{ ms}$ |
| `CountdownGo` | Ascending arpeggio | $523\text{ Hz} \to 659\text{ Hz} \to 784\text{ Hz} \to 1046\text{ Hz}$ | $180\text{ ms}$ |
| `Alert` / `Warning` | Alternating two-tone warble | $1800\text{ Hz} \leftrightarrow 2400\text{ Hz}$ | $200\text{ ms}$ |

### 2.4. Harmonic Shapes (Geometric Pitch Modulation)
* **Mechanism**: When projecting a closed shape (circle, square, target crosshair) in a repeating scan loop:
  $$\text{Pitch (Hz)} = \frac{\text{DAC Sample Rate (pps)}}{\text{Total Points in Loop}}$$
* **Pitch-Locked Polyline Generator**:
  - Given a desired musical pitch (e.g., $A_4 = 440\text{ Hz}$) and a sample rate of $30\text{ kpps}$:
    $$\text{Target Points} = \frac{30000}{440} \approx 68 \text{ points}$$
  - The path optimizer automatically samples or resamples the target polygon to exactly 68 equidistant points.
  - As different target states activate (normal target vs. bonus target vs. boss target), their physical shapes naturally hum in harmonious chords (e.g. Root, Major Third, Fifth, Octave).

### 2.5. Laser Theremin (Interactive Instrument Minigame)
* **Game Mode Concept**: An expressive audio-visual laser instrument controlled by gamepad thumbsticks, mouse, or tracked retroreflective wand/gun:
  * **X Axis**: Continuous pitch glide (or scale-quantized pentatonic / chromatic notes) from $150\text{ Hz}$ to $3000\text{ Hz}$.
  * **Y Axis**: Waveform timbre / harmonic richness or volume amplitude ($0.2\% \to 2.0\%$ carrier excursion).
  * **Visual Feedback**: The laser draws concentric rippling acoustic rings or an animated Lissajous figure whose loop count dynamically reflects the oscillating pitch.

---

## 3. Hardware Safety & Galvo Health Guardrails

Because galvanometer coils have low DC resistance and high inductive reactances, aggressive high-frequency signals must be bounded:

1. **Amplitude Clamping**:
   - Carrier amplitude $A$ strictly clamped to $\le 2.0\%$ full scale ($[-655, +655]$ DAC counts).
   - Prevents driver overheating and thermal tripping.
2. **Frequency Ceiling**:
   - Hard upper cutoff at $5.5\text{ kHz}$. Frequencies above $6\text{ kHz}$ dissipate excessive power in the scanner damping circuits without providing audible benefit.
3. **Resonance Notch Avoidance**:
   - Standard 20k/30k scanners have mechanical armature resonances around $3.2\text{–}4.2\text{ kHz}$. A configurable notch filter or resonance attenuation band will prevent driving unattenuated tones at the scanner's exact resonance frequency.
4. **Duty Cycle Limiter**:
   - Long continuous drone tones (>5 seconds) automatically taper amplitude to $50\%$ to ensure driver thermal headroom.

---

## 4. Architecture & Implementation Plan

```
lasertargets/
├── common/
│   └── src/
│       └── audio.rs              # Add GalvoAudioSettings & GalvoToneEvent
├── laserlogic/
│   └── src/
│       ├── galvo_audio/          # Audio synthesis, ADSR, carrier modulator
│       │   ├── mod.rs
│       │   ├── synth.rs          # Pure tone & chirp sample generator
│       │   ├── carrier.rs        # Point stream superimposition
│       │   └── harmonic.rs       # Path resampling for geometric pitch
│       └── lib.rs
└── server-core/
    └── src/
        └── plugins/
            ├── galvo_audio.rs    # Bevy plugin listening to PlaySoundEvent
            └── projector.rs      # Integrates carrier injection into DAC buffer
```

### Phased Roadmap

* [ ] **Prerequisite**: Complete Camera & Vision Tracking pipeline (IMX219 capture, calibration, hit detection).
* [ ] **Phase 1: Math & Waveform Generator (`laserlogic::galvo_audio`)**:
  - Implement 12-TET tone generator and sample-rate downsampler for DAC point rates ($20\text{k} - 40\text{kpps}$).
  - Implement ADSR soft envelope to eliminate start/stop click transients.
  - Implement thermal amplitude clamp ($\le 2\%$ excursion).
* [ ] **Phase 2: Superimposed Carrier Transformer**:
  - Create DAC point transformer that injects carrier sine/orbit onto existing polyline frames.
  - Benchmark point rate jitter and verify visual beam quality remains sharp on test wall.
* [ ] **Phase 3: Galvo Sound FX Bevy Plugin**:
  - Wire `PlaySoundEvent` listener to galvo chirp synth.
  - Add YAML configuration toggles under `audio.galvo_audio: { enabled: true, volume: 0.8, mode: "both" }`.
  - Test blanked countdown beeps and target hit chirps directly on Helios hardware.
* [ ] **Phase 4: Harmonic Geometric Shapes**:
  - Add point-count pitch quantizer to `laserlogic` path generator.
  - Test resonant pitch intervals on minigame targets (`Hunter` / `Snake`).
* [ ] **Phase 5: Laser Theremin Mode**:
  - Implement standalone interactive mode with gamepad / laser pointer tracking.
  - Project dynamic acoustic Lissajous waveforms responding in real time.

