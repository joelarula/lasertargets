# LaserTargets Sound Assets

Place sound effect assets in this directory (`assets/sounds/`). The server's `LaserAudioPlugin` preloads these files into `SoundBank` on startup.

Supported formats: `.wav`, `.ogg` (Vorbis), `.flac`.

### Registered Cue Mapping

| Sound ID | Default File Path | Description |
| :--- | :--- | :--- |
| `SoundId::LaserHit` | `assets/sounds/laser_hit.wav` | Immediate ping/blip on optical camera laser hit detection |
| `SoundId::TargetSpawn` | `assets/sounds/target_spawn.wav` | Chime when a minigame target appears |
| `SoundId::TargetHit` | `assets/sounds/target_hit.wav` | Minigame target successfully hit / scored |
| `SoundId::TargetMiss` | `assets/sounds/target_miss.wav` | Target expired without being hit |
| `SoundId::CountdownTick` | `assets/sounds/countdown_tick.wav` | Countdown tick (3... 2... 1...) |
| `SoundId::CountdownGo` | `assets/sounds/countdown_go.wav` | Match start chime |
| `SoundId::GameOver` | `assets/sounds/game_over.wav` | Round end fanfare / game over tone |
| `SoundId::PelletEat` | `assets/sounds/pellet_eat.wav` | Snake minigame pellet pickup |
| `SoundId::SnakeCrash` | `assets/sounds/snake_crash.wav` | Snake minigame crash / wall hit |
| `SoundId::CalibrationCornerAcquired` | `assets/sounds/calibration_tone.wav` | Operator feedback when corner target is locked |
| `SoundId::Alert` | `assets/sounds/alert.wav` | General warning or error tone |

