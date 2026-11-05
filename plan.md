# LaserTargets Development Plans

## Active & Upcoming Engineering Plans

1. [Camera Stream & Vision Tracking Plan](docs/CAMERA_STREAM_PLAN.md)
   - **Status**: **In Progress / Current Active Task**
   - Pi Camera Module v2 (Sony IMX219) MIPI-CSI2 pipeline.
   - Ultra-fast 200 FPS hit tracking mode, 81 FPS calibration mode.
   - Homography matrix alignment, thresholding, and live operator stream.

2. [Galvo Acoustic Audio & Synthesis Plan ("Singing Galvos")](docs/GALVO_AUDIO_PLAN.md)
   - **Status**: **Scheduled (After Camera Work is Finished)**
   - Utilizes galvanometer voice-coil physics to project acoustic sound directly from the laser projector scanner block.
   - **Core Features**:
     - **Carrier Injection**: Superimposing micro-wobbles (0.5%–1.5% deflection) to sing tones while maintaining razor-sharp visual laser graphics.
     - **Playing Notes & Octaves**: 12-TET musical note synthesis, soft ADSR envelopes, and blanked-beam chiptune/buzzer playback.
     - **Galvo Sound Effects Engine**: Hardware acoustic sound FX (`LaserHit` chirp, `CountdownTick`, `CountdownGo` arpeggios, `TargetMiss` buzz) hooked into Bevy `PlaySoundEvent`.
     - **Harmonic Shapes**: Tuning path point density so geometric shapes naturally sing in musical octaves ($\text{Pitch} = \frac{\text{Point Rate}}{\text{Point Count}}$).
     - **Laser Theremin**: Interactive instrument minigame mapping position (X/Y) to acoustic pitch and timbre with live projected Lissajous visual feedback.
   - **Hardware Safety**: Strict amplitude clamps ($\le 2\%$), resonance avoidance bands ($3.2\text{–}4.2\text{ kHz}$), and thermal duty-cycle management.

3. [Scene-Filling Strategies Plan](docs/FILL_STRATEGIES_PLAN.md)
   - Raster, spiral, lissajous, and hilbert fill strategies.

4. [Transformer Pipeline Architecture](docs/TRANSFORMER_PIPELINE.md)
   - Point pipeline, blanking optimization, and post-processing stages.

