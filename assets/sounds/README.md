# LaserTargets Sounds

Every sound is a composition: `<name>.sound.yaml`, built from sources (`tone`, `chirp`, `noise`,
`silence`, nested `sound`) and stages (`envelope`, `gain`, `pan`, `tremolo`, `lowpass`, `echo`),
with parameters that can follow live signals. Both audio outputs play the same files: the
speakers at 48 kHz, the galvos at the DAC point rate. Edit a file while the server runs and the
sound changes live. Format and plugin API: [docs/SOUND_COMPOSITION.md](../../docs/SOUND_COMPOSITION.md).

The file name is the sound name used by `SoundId::name()`, `PlaySound::named(..)` and
`audio.routing` in `config/server.yaml`. Every layer and stage carries a comment saying what it
does.

## Recorded overrides (speakers only)

A recorded file next to a composition, `<name>.wav`, `.ogg` or `.flac`, replaces it on the
speakers; the galvos still play the composition.

## Shipped sounds

| Sound | `SoundId` | What it is |
| :--- | :--- | :--- |
| `laser_hit` | `LaserHit` | Down-chirp 2800 → 1100 Hz, 45 ms |
| `target_spawn` | `TargetSpawn` | C6 → E6 pip |
| `target_hit` | `TargetHit` | A6 blip |
| `target_miss` | `TargetMiss` | 160 Hz buzz |
| `countdown_tick` | `CountdownTick` | 2.2 kHz click |
| `countdown_go` | `CountdownGo` | C5 E5 G5 C6 arpeggio |
| `game_over` | `GameOver` | G5 E5 C5 G4 falling line |
| `pellet_eat` | `PelletEat` | Up-chirp 1400 → 2000 Hz |
| `snake_crash` | `SnakeCrash` | Down-sweep plus noise thump |
| `calibration_corner_acquired` | `CalibrationCornerAcquired` | C7 bell |
| `calibration_success` | `CalibrationSuccess` | C6 E6 G6 fanfare |
| `alert` | `Alert` | 1.8 / 2.4 kHz warble |
| `sci_fi_lock` | — | Crossing sweeps, left/X up while right/Y down |
| `power_chord` | — | Root + fifth on separate channels / galvos |
| `major_third_chime` | — | Root + major third |
| `beat_warning` | — | 440 vs 447 Hz, beats at 7 Hz |
| `theremin` | — | Instrument: pitch ← `gamepad.lx`, volume ← `gamepad.ly`; plays until stopped |
