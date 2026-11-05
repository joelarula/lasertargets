# Scene-Filling Strategies Plan

## 1. Objective

Fill an area of the scene (a target, a shape's interior, a rectangle) with laser light using five
strategies. Each one is a **fill source** in the transformer pipeline
([TRANSFORMER_PIPELINE.md](TRANSFORMER_PIPELINE.md) §5). Each is selectable and tunable live
through a pipeline asset, and verified on the real projector through the Helios DAC at 30 kPPS.

Fills are also a main channel for **reactivity** (TRANSFORMER_PIPELINE §4). An entity's fill can
show its state: a target fills up as it charges, drains as it loses health, densifies when the
reticle comes near, and scatters into stipple when it pops.

The five strategies cover the real trade-offs of a galvo scanner:

| # | Key | Strategy | Character | Tests on hardware |
|---|---|---|---|---|
| 1 | `lissajous` | Resonant Lissajous pendulum | Harmonic sines, continuous drift, near-zero jerk | Mirror bandwidth |
| 2 | `raster` | Interlaced CRT raster | Scanlines with blanked turnaround loops in the overscan | Turnaround vs. flyback ringing |
| 3 | `spiral` | Double Archimedean spiral | Outward and inward interleaved sweep, no jumps | Continuous curvature |
| 4 | `hilbert` | Filleted Hilbert curve | Order 3–4 space-filling curve with arc fillets | Corner speed |
| 5 | `stipple` | TSP stipple matrix | Dots on a 2-opt tour, blank transits, micro-dwell | Dot sharpness, jump settle |

## 2. Fill Source Contract

```rust
/// Area to fill, in the entity's local frame (metres)
pub struct FillRegion {
    /// Bounding box the pattern is laid out in
    pub bounds: Rect,
    /// Optional closed contours to clip to (outer + holes, even-odd); empty = bounds only
    pub contours: Vec<Vec<Vec2>>,
    /// Extra margin outside the visible area where turnarounds may run blanked
    pub overscan_m: f32,
}

pub trait FillStrategy: Send + Sync + 'static {
    fn generate(&self, region: &FillRegion, ctx: &StageCtx, out: &mut UniversalPath);
}
```

- `region: self` takes the closed strokes of the entity's Shape v2 as contours. `region: rect`
  takes explicit bounds. A target hit-zone can provide its own region later.
- Clipping to contours turns crossings into blanked moves inside the same stroke (§3.1), not new
  strokes.
- Every numeric parameter is a `Param`, so density, phase, frequency ratios, etc. can follow
  entity signals (`self.health`, `self.hit`, `self.near`) or global ones (`audio.bass`)
  (TRANSFORMER_PIPELINE §4.1–4.2).
- Bound parameters make the fill regenerate every frame, so generators must be cheap enough for
  that. Each strategy states its cost per frame at the default density, and a golden test enforces
  it.
- Strategies are pure and deterministic for the same inputs and time. Golden-file tests rely on
  this.

### 2.1 Parameters per strategy

| Strategy | Parameters |
|---|---|
| `lissajous` | `fx`, `fy`, `phase`, `drift_hz` (phase drift over time), `steps`, `color` |
| `raster` | `lines`, `interlace` (fields, 1 = progressive), `overscan_m`, `turnaround: teardrop \| arc`, `bidirectional`, `color` |
| `spiral` | `turns`, `double` (out + in interleaved), `steps_per_turn`, `color` |
| `hilbert` | `order` (3–4), `fillet_radius_m`, `color` |
| `stipple` | `count` or `spacing_m`, `jitter`, `seed`, `dwell_ms`, `two_opt_iters`, `color` |

Common to all strategies, chosen so reactive bindings have meaningful targets:

| Parameter | Range | Meaning | Typical binding |
|---|---|---|---|
| `coverage` | 0..1 | Fraction of the region filled, along the strategy's natural order (raster: bottom-up lines; spiral: radius; Hilbert/stipple: tour length) | `self.health`, charge level |
| `density` | 0..1 | Scales line count / steps / dot count between the strategy's min and max | `self.near`, `audio.mid` |
| `phase` | 0..1 | Moves the pattern along its own order without changing its shape (marching raster, rotating spiral) | `lfo.*`, time |
| `intensity` | 0..1 | Brightness multiplier, applied before diode shading | `self.hit` flash |

`stipple` keeps its tour stable for a given `seed`, so changing `coverage` or `density` reveals or
hides dots instead of reshuffling them.

### 2.2 Registration

```rust
pub struct FillStrategiesPlugin;

impl Plugin for FillStrategiesPlugin {
    fn build(&self, app: &mut App) {
        app.register_fill_strategy::<LissajousFill>("lissajous")
            .register_fill_strategy::<RasterFill>("raster")
            .register_fill_strategy::<SpiralFill>("spiral")
            .register_fill_strategy::<HilbertFill>("hilbert")
            .register_fill_strategy::<StippleFill>("stipple");
    }
}
```

The implementations live in `laserlogic/src/fill/`, one file per strategy, with no Bevy systems.

## 3. Path Model Prerequisites

The current renderer can't show three of the strategies correctly. These changes come first
(TRANSFORMER_PIPELINE phase P7).

### 3.1 Blanked moves inside a stroke

**Problem:** `StrokeBuilder` ends a stroke at every black point
([projector.rs](../server-core/src/plugins/projector.rs)). Raster turnarounds therefore become
straight blank jumps with depart and settle holds, so the galvo brakes at every line end, which is
exactly what overscan should avoid.

**Change:**
- Add `PathPoint.blank: bool`: the edge starting at this point is unlit, but the galvo keeps moving
  along the geometry.
- `StrokeBuilder` keeps blank edges in the same `LaserSegment`. `optimize` draws them at lit speed
  with colour 0 and no depart, settle or corner holds.
- Shape v2 gets `StrokePoint.blank: bool`. The schema meaning of a stroke becomes "one continuous
  galvo move". Black colour stays a colour, not a blanking signal.
- A real laser-off jump is still expressed as a new stroke.

### 3.2 Arc fillets

**Problem:** `corner_radius_m > 0` is accepted but drawn sharp
([shape_v2.rs](../common/src/shape_v2.rs)).

**Change:** One shared `fillet_corner(prev, corner, next, radius) -> arc points` in
`laserlogic::stages::corner_fillet`. `Shape::to_universal_path` uses it for per-point
`corner_radius_m`. The `corner_fillet` stage and the `hilbert` strategy use it too. The optimizer
then sees a gentle curve instead of a corner, so it adds no corner hold.

### 3.3 Dwell in milliseconds

**Problem:** `dwell_ms` is converted to a `u8` repeat count at `V1_REFERENCE_PPS`, not at the
live PPS.

**Change:** Carry the hold in milliseconds on `PathPoint` (or convert at `Collect` time with
`projector.dac_pps`). Stipple dots then last as long as asked at any PPS.

## 4. Data Flow and Safety

```
 Pipeline asset (fill: raster, lines: 24, …)
        │
        ▼
 Tier 1  FillSource ── FillStrategy::generate(region) ──▶ BasePath
 Tier 2  stage stack (e.g. wave_displace bound to audio.bass) ──▶ UniversalPath
 Tier 3  Collect → Order → Kinematics (compose_frame, projector.optimize)
                 → Safety (fixed) → Emit ──▶ LaserPointBuffer ──▶ DAC thread
                                                                   │ USB bulk
                                                                   ▼
                                       Helios DAC: 12-bit X/Y, 8-bit RGBI, shutter
                                                                   │ ILDA DB-25
                                                                   ▼
                                       Projector: X/Y galvos (20–40 kPPS), RGB diodes
```

Safety is enforced only in the frame pipeline's `Safety` set (TRANSFORMER_PIPELINE §9). It can't
be disabled by a pipeline. The rules relevant to fills:

1. **Step cap:** consecutive lit points at most `max_lit_step_dac` apart (150 DAC units, 3.6% of
   full scale), blank points at most `max_blank_step_dac`.
2. **Blanking:** colour channels forced to 0 on jumps and on `blank` edges.
3. **No stationary burn:** consecutive lit holds at one position are capped. This limits stipple
   `dwell_ms`.
4. **Velocity-proportional intensity** (optional, part of the diode model in
   `projector.optimize`): `I = I0 · max(floor, min(1, v / v_ref))` on lit *motion* only. Explicit
   dwell holds keep their colour, so stipple dots don't go dark.
5. **Point budget:** a frame over `dac_pps / tick_hz` points is thinned before output, never
   silently stretched.
6. **Shutter interlock:** closes on exit, user break, or 25 consecutive USB errors (DAC thread).

## 5. Implementation Phases

These phases match TRANSFORMER_PIPELINE §12. F1–F2 can start after P1–P3 (stage model, assets,
signals). Reactive demos (F3) also need P4–P5 (reactions, states, effects).

### Phase F1: Path model prerequisites (pipeline P7)
- **Scope:** `common::path`, `common::shape_v2`, `server-core` projector `StrokeBuilder`,
  `laserlogic::optimize`
- **Tasks:** blank flag (§3.1), shared fillet function (§3.2), dwell in ms (§3.3)
- **Done when:**
  - A raster stroke with blank turnarounds produces a single `LaserSegment` with no settle points.
  - A filleted square has no corner holds.
  - A 0.25 ms dwell is 7–8 points at 30 kPPS.

### Phase F2: Fill strategies in Rust (pipeline P7)
- **Scope:** `laserlogic/src/fill/`, `FillStrategiesPlugin`, fill source in `PathPipelinePlugin`
- **Tasks:**
  1. Port the five generators from `scripts/generate_fill_demos.js` to Rust with the parameters in
     §2.1, including the common `coverage` / `density` / `phase` / `intensity`.
  2. Fix two generators along the way:
     - `stipple` emits one stroke per dot (or blank transits), not one lit polyline.
     - `raster` marks turnaround edges `blank` instead of colouring them black.
  3. Add region support: lay out within the bounds, clip to contours.
  4. Write golden-file tests that compare against fixtures regenerated from the Rust code.
- **Done when:**
  - All five fill a `region: self` shape and a `region: rect` area.
  - Changing a parameter in the asset updates the projection live.
  - Sweeping `coverage` 0→1 every frame stays within each strategy's per-frame cost.

### Phase F3: Demo and reactive pipelines
- **Scope:** `assets/pipelines/fill/*.pipeline.yaml`, `assets/pipelines/hunter/*.pipeline.yaml`
- **Tasks:**
  1. One static demo pipeline per strategy (`fill/lissajous`, `fill/raster`, …).
  2. Reactive examples:

     | Pipeline | Reaction |
     |---|---|
     | `fill/health_raster` | Raster `coverage` bound to `self.health`; `hit` envelope flashes `intensity` and adds `wave_displace` |
     | `fill/charge_spiral` | Spiral `coverage` follows `self.charge`; when full, the `charged` state spins `phase` with `lfo.fast` |
     | `fill/proximity_hilbert` | Hilbert `density` bound to `self.near`: sparse when idle, dense as the reticle approaches |
     | `fill/pop_stipple` | On `pop`, crossfade the shape outline into a stipple of the same region, then `shatter` and `finished` |
     | `fill/audio_raster` | Raster `phase` on `audio.beat`, `wave_displace` on `audio.bass` |

  3. Tag-based defaults for the demo shapes.
- **Done when:**
  - Each pipeline loads without warnings.
  - The reactive ones respond correctly to headless trigger and component tests (e.g. set
    `TargetHealth` to 50% → `coverage` 0.5).

### Phase F4: `dac-test fill-demos` (pipeline P9)
- **Scope:** `dac-test/src/main.rs`
- **Tasks:**
  1. `cargo run -p dac-test -- fill-demos [pipeline]`: loads `assets/pipelines/fill_*.pipeline.yaml`
     and evaluates them with `laserlogic::evaluate` plus the same `compose_frame` and `Safety` as
     the server. There is no second implementation.
  2. Interactive keys (`crossterm`):
     - `[1]`–`[5]` select strategy, `[←]/[→]` cycle
     - `[+]/[-]` PPS 20 / 30 / 40 kPPS
     - `[[]/[]]` density down / up (writes the `Param` override)
     - `[O]` show overscan: draws blank edges dimly lit, *bench only*
     - `[A]` drive `audio.*` from a synthetic beat
     - `[H]` fire a `hit` trigger, `[P]` fire `pop`, `[N]` sweep `self.near`, `[↑]/[↓]` step `self.health`
       (the same reactions the game would trigger, replayed on the bench)
     - `[Q]/[Esc]` stop output, close shutter, release USB
  3. Status line: points per frame, budget use %, max lit step, number of jumps.

### Phase F5: Hardware bench verification
1. **DAC detection:** `dac-test info` enumerates the Helios.
2. **Alignment:** `dac-test box 5` gives a centred box, shutter OK.
3. **Lissajous:** smooth continuous motion, no coil whine across `fx:fy` presets.
4. **Raster:** straight visible scanlines. With `[O]`, turnarounds sit fully in the overscan and
   there is no brightening at line ends.
5. **Spiral:** no visible seam at the in/out handover.
6. **Hilbert:** no overshoot or ringing at filleted corners. Compare with `fillet_radius_m: 0`.
7. **Stipple:** separate crisp dots, no visible transits, dot brightness even across the tour.
8. **Budget:** every demo pipeline stays under the point budget at 30 kPPS (status line). Record
   the max `lines` / `count` before the budget is hit.
9. **Reactive bench:** a `coverage` sweep from 0 to 1 has no flicker or stalls. Rapid `[H]` hits
   don't push the frame over budget. A `[P]` pop with its effects stays within budget, with effects
   thinned first.
10. **Safety:** a pipeline with an extreme `frame:` override is clamped. Pulling USB closes the
    shutter.

## 6. Deliverables

| Deliverable | Location | Description |
|---|---|---|
| Path model changes | `common/src/path.rs`, `common/src/shape_v2.rs`, projector, `laserlogic::optimize` | Blank edges, fillets, dwell in ms |
| Fill strategies | `laserlogic/src/fill/*.rs` | Five `FillStrategy` implementations, pure and tested |
| Golden fixtures | `assets/shapes/fill_demos/*.json` | Regenerated from the Rust generators. The only copy (dropped from `templates/v2`) |
| Demo pipelines | `assets/pipelines/fill/*.pipeline.yaml` | One per strategy, plus the reactive examples in F3 |
| Hardware runner | `dac-test/src/main.rs` | `fill-demos` subcommand on the shared evaluator, with trigger replay |
| Pipeline architecture | [TRANSFORMER_PIPELINE.md](TRANSFORMER_PIPELINE.md) | Reactivity, sources, stage stacks, signals, frame pipeline |

`scripts/generate_fill_demos.js` is retired once F2's Rust generators produce the fixtures.
