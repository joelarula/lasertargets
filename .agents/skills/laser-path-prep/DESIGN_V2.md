# Shape Language v2 & Laser Pipeline — Design (draft for review)

Status: **design agreed in discussion, not implemented yet.** Text rendering is out of scope; it keeps
its own optimisation pipeline and will be analysed separately.

## Principles

1. **Shapes are abstract.** A shape describes geometry, colour and intent. It never contains blank
   points, stroke lead-ins or point counts.
2. **Laser physics lives in one stage.** Galvo motion and diode timing are added by the laser
   transform, nowhere else.
3. **Physical units everywhere.** Lengths in metres (on the scene plane), times in milliseconds,
   speeds in metres per second. Point counts only exist inside the laser stage, derived from PPS.

## Pipeline

```
1. Shape            v2 template / procedural shape: strokes in metres
       │
2. Placement        shape → scene → world → projector → DAC coordinates      (geometry, metres)
       │            - place the shape (position, rotation) in the scene
       │            - expand line styles (dashes/dots become separate strokes)
       │            - fillet corners that have corner_radius_m > 0
       │            - project with SceneSetup::projector_ndc, clip to the scene
       │
3. Laser transform  the only stage that knows the physical laser             (laserlogic)
       │   3a Galvo model   XY motion: speed along lines, slow-down and speed-up around
       │                    direction changes, corner holds, blank jumps and settling
       │   3b Diode model   light timing: laser-on and laser-off delays shift the light
       │                    signal against the XY signal; blank = all channels 0
       │   3c Composer      stroke order, minimum frame rate, wrap back to frame start
       │
4. DAC transport    delivers finished frames on time; Windows / Raspberry Pi differences
                    (timer resolution, thread priority, USB pacing) are handled only here
```

The XY stream is the "galvo dance": a mechanical signal limited by mirror inertia.
The light stream is the diode: an electrical/optical signal with warm-up and dimming times.
Stage 3 shapes the first and time-shifts the second against it.

## Shape language v2

Files: `assets/shapes/templates/v2/*.json`. All lengths in metres.

```json
{
  "name": "box",
  "description": "0.5 m square outline",
  "tags": ["geometry", "calibration"],
  "version": 2,
  "strokes": [
    {
      "closed": true,
      "color": [0, 255, 255],
      "points": [
        { "x": -0.25, "y":  0.25 },
        { "x":  0.25, "y":  0.25 },
        { "x":  0.25, "y": -0.25 },
        { "x": -0.25, "y": -0.25 }
      ]
    }
  ]
}
```

### Stroke

One continuous line drawn without switching the laser off.

| Field | Type | Default | Meaning |
|---|---|---|---|
| `points` | array | required | Polyline vertices. One point = a **dot**. |
| `closed` | bool | `false` | Joins the last point back to the first; do not repeat the first point. |
| `color` | `[r, g, b]` 0–255 | white | Stroke colour. |
| `style` | see below | `"solid"` | Line style. |

`style`:
- `"solid"`
- `{ "dashed": { "dash_m": 0.10, "gap_m": 0.05 } }`
- `{ "dotted": { "spacing_m": 0.05 } }`

Each dash or dot becomes its own stroke in stage 2, so it gets the same clean laser on/off handling
as any other stroke.

### Point

| Field | Type | Default | Meaning |
|---|---|---|---|
| `x`, `y` | metres | required | Position in the shape's own frame. |
| `color` | `[r, g, b]` | stroke colour | Colour of the edge that **starts** at this point. |
| `corner_radius_m` | metres | not set | Not set: the turn angle decides (auto). `0`: forced sharp corner with full hold. `> 0`: corner replaced by an arc of this radius in stage 2, drawn without stopping. |
| `dwell_ms` | ms | not set | Optional override: replaces whatever hold the laser stage would compute here. |

### Conversion of v1 templates

v1 files (`templates/*.json`, `unsorted/`) stay readable. The loader converts them:
- blank points (r = g = b = 0) split the point list into strokes
- a stroke whose last point equals its first becomes `closed: true`
- `line_style` becomes `style`
- v1 `dwell` (point count) becomes `dwell_ms` using the reference rate of 30000 PPS
- normalised v1 coordinates (−1..1) are scaled by the size given at placement

## Laser transform parameters (stage 3)

All set in physical units; converted to point counts with the current PPS.

### 3a Galvo model

| Parameter | Unit | Meaning |
|---|---|---|
| `lit_speed_m_s` | m/s | Speed along lit lines on the scene plane. Sets the spacing of intermediate points on long lines, so the galvo moves linearly at constant speed. |
| `max_step_dac` | DAC units | Hard cap per point from the galvo rating; guards against too-large steps when the scene is far away. |
| `corner_ramp_ms` | ms | Slow-down before and speed-up after a direction change: points get denser towards the corner and sparser after it, matching mirror inertia. |
| `corner_hold_ms` | ms | Hold at a 90° turn; scales with turn angle. Gentle turns (curves) get no hold. |
| `blank_speed_m_s` | m/s | Travel speed of laser-off jumps between strokes. |
| `settle_ms` + `settle_ms_per_m` | ms | Hold after a jump before the laser turns on; grows with jump length. |

### 3b Diode model

| Parameter | Unit | Meaning |
|---|---|---|
| `laser_on_delay_ms` | ms | Time from "on" command to visible light. Light is commanded this much earlier. |
| `laser_off_delay_ms` | ms | Time from "off" command to dark. Light is cut this much earlier. |

Blank means r = g = b = 0 **and** intensity 0. There is no other blank convention.

### 3c Composer

| Parameter | Unit | Meaning |
|---|---|---|
| `min_frame_hz` | Hz | Lowest acceptable refresh rate; warns when a frame gets too long. |

Frames are never truncated. Padding, if the transport needs it, is added as blank points at the
frame start position.

## Changes from the current code

| Today | v2 |
|---|---|
| Templates contain blank lead points and dwell counts | Templates contain geometry and intent only |
| `update_point_buffer` expands `dwell` into repeated points (breaks corner detection) | Dwell overrides travel as `dwell_ms` into stage 3 |
| `optimize` works in point counts; colour delay is a fixed 2-point shift that does nothing at on/off edges | Galvo and diode models in ms and m/s, separate on and off delays |
| Points with r = g = b = 0 are sent with intensity 255 | One blank rule |
| DAC thread truncates and pads frames to 1024 points | Composer builds the frame; transport only delivers it |
| Dashed/dotted expansion inside `PathSegment::expand_line_style` with fixed metre values | `style` in the shape, expanded in stage 2 with values from the shape |

## Open points

- How `lit_speed_m_s` relates to galvo angular speed when the scene distance changes; `max_step_dac`
  is the safety net until this is measured.
- Starting values for every parameter: to be found with the calibration patterns (crosshair, square,
  line with end ticks, dot grid) and a slow-motion video, on both Windows and the Pi.
