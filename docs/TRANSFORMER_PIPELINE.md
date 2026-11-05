# Reactive Laser Pipeline

How entities in the Bevy world **react** to what happens to them (hits, health, proximity, state,
music) with generated laser visuals, and how the pipeline that produces those visuals is plugged
in, ordered and configured at runtime.

## 1. Goals

- **Reactive first.** A pipeline describes how an entity *looks* and also how it *responds*. When
  an entity is hit, loses health, changes state or comes close to a player, the reaction is visual
  and declared in data: a flash, a ripple, a fill draining, a burst of sparks, a morph into another
  state. Game code raises events and updates components. It doesn't draw.
- **Pluggable.** New triggers, signals, generators, stages and effects each come from one plugin
  in any crate. No central enum, match or reload function needs to change.
- **Composable.** Each entity has its own ordered stack of stages. The same stage can appear more
  than once, and reactions can target any stage by id.
- **Dynamic.** Any numeric parameter can follow a signal: the entity's own state (`self.health`), a
  reaction envelope (`self.hit`), or a global signal (`audio.bass`, `gamepad.rt`). Pipeline files
  hot-reload without a restart.
- **Correct by construction.** Every stage reads an input and writes a separate output. Nothing
  accumulates frame over frame.
- **Safe.** Galvo and diode limits are owned by the projector and enforced by a final stage that no
  pipeline or reaction can disable.
- **Testable.** Stages and generators are plain functions. Reactions are testable with a headless
  `App` that triggers events and inspects the resulting `UniversalPath`.

## 2. Starting Point

| Exists today | Where | Role in this design |
|---|---|---|
| `UniversalPath` component | [common/src/path.rs](../common/src/path.rs) | Final per-entity output, read by the projector and the network broadcast |
| `PathModifier` + `PathModifierPipeline` + `PathModifierRegistry` | [common/src/path.rs](../common/src/path.rs) | Replaced by `PathStage` + `PathPipeline` + stage registry (§6) |
| `ModulatorSource` (LFO, timeline, gamepad, ratio, manual) | [common/src/path.rs](../common/src/path.rs) | Becomes the built-in signal sources (§4.2) |
| `PathModulator` + `MorphStrategy` | [common/src/path.rs](../common/src/path.rs) | Morph source, and crossfade between reactive states (§5) |
| `Shape::from_json` / `to_universal_path` | [common/src/shape_v2.rs](../common/src/shape_v2.rs) | The shape source (§5) |
| Galvo + diode model, `compose_frame` | [laserlogic/src/optimize.rs](../laserlogic/src/optimize.rs) | The frame pipeline's kinematics step (§9) |
| `update_point_buffer`, `StrokeBuilder` | [server-core/src/plugins/projector.rs](../server-core/src/plugins/projector.rs) | Split into the frame pipeline (§9) |
| `ConfigSection` layered config | [common/src/settings.rs](../common/src/settings.rs) | Still owns projector, galvo and DAC settings (§10) |
| Hand-written reactive visuals: `HunterShotRipple`, title announcement | [minigames/hunter/src/systems/render.rs](../minigames/hunter/src/systems/render.rs) | Become pipeline reactions with no game-side drawing code (§4.6) |
| Game messages (`HunterClickEvent`, `SpawnHunterTargetEvent`, session messages) | `minigames/*/src/events.rs`, [common/src/session.rs](../common/src/session.rs) | Registered as triggers (§4.3) |
| `seldom_state` in the workspace | `Cargo.toml` | Optional state-machine source for reactive states (§4.5) |

Known defects this design fixes:
- `apply_path_modifier_pipelines` modifies `UniversalPath` in place every frame, so modifier effects
  accumulate.
- Every reactive visual is custom code that overwrites `UniversalPath`. The visual can't be
  restyled without recompiling, and it can't be combined with other effects.
- There is no live audio analysis yet.

## 3. Architecture

```
                 WORLD                                         VISUALS
 ┌────────────────────────────┐   ┌──────────────── per entity ────────────────────┐  ┌─ per frame ─┐
 │ Components  Health, Speed, │   │ Tier 1: SOURCE    Tier 2: STAGE STACK          │  │ Tier 3      │
 │             TargetState …  │──▶│ shape | fill |    static prefix ▶ dynamic      │  │ Collect     │
 │ Events      Hit, Pop,      │   │ morph | inline ─▶ (cached)        (per frame) ─┼─▶│ Order       │
 │             Spawned, Shot  │   │      ▲ BasePath        ▲ params bound to Param │  │ Kinematics  │
 │ Positions   reticles,      │   │      │ state switch    │                      │  │ Safety      │
 │             hunters        │   └──────┼─────────────────┼──────────────────────┘  │ Emit → DAC  │
 └──────┬─────────────────────┘          │                 │                         └─────────────┘
        │ REACTIVITY (§4)                │                 │
        │ bind_signal  ──▶ EntitySignals: self.health, self.speed, self.near …
        │ triggers     ──▶ Reactions: envelope ▸ self.hit │ set │ state │ spawn effect │ emit
        │ global       ──▶ Signals: audio.bass, gamepad.rt, lfo.slow …
        ▼
   spawned effect entities (ripple, sparks, shockwave) run their own pipelines and despawn on finish
```

| Layer | Unit | Runs when | Declared in |
|---|---|---|---|
| Reactivity | Entity + world | Events, component changes, every frame for continuous signals | Plugins register inputs; pipeline asset `reactions:`/`states:` |
| 1. Source | One entity | Its inputs or reactive state change | Pipeline asset `source:` |
| 2. Stage stack | One entity | Static prefix on change, dynamic suffix every frame | Pipeline asset `stages:` |
| 3. Frame pipeline | All entities | Every frame | `projector.*` in `server.yaml` |

Schedule (all in `Update`, chained):

```rust
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum PathPipelineSet {
    /// Pipeline assets resolved into per-entity instances (spawn, hot reload)
    Bind,
    /// Global signals, component bindings and message triggers; newly bound entities
    /// already get their component signals in their first frame
    Signals,
    /// Triggers routed; reactions, envelopes, clocks advance; emits sent; finished effects despawn
    React,
    /// Source (when changed) + static prefix (cached) + dynamic suffix (per frame) → UniversalPath
    Stages,
}

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum LaserFrameSet { Collect, Order, Kinematics, Safety, Emit }
```

Game systems that change health, raise hits or move targets run *before* `PathPipelineSet`, so a
hit is visible in the same frame it happens.

## 4. Reactivity

Reactivity has three inputs and five kinds of output. Plugins provide the inputs, and the
pipeline asset declares the responses.

### 4.1 Entity signals: continuous world state

Each pipeline entity gets an `EntitySignals` component. It's the entity's own signal namespace,
read in YAML as `self.<name>`.

```rust
#[derive(Component, Default)]
pub struct EntitySignals(SmallMap<SignalId, f32>);
```

Plugins fill it from components with one registration line, Bevy-style:

```rust
pub trait ReactiveAppExt {
    /// Publishes a component value as `self.<key>` whenever the component changes
    fn bind_signal<C: Component>(&mut self, key: &'static str, read: fn(&C) -> f32) -> &mut Self;
}

app.bind_signal::<TargetHealth>("health", |h| h.current as f32 / h.max as f32)
   .bind_signal::<BalloonRiseSpeed>("speed", |s| (s.0 / 1.0).clamp(0.0, 1.0));
```

`bind_signal` adds one generic system per type that runs on `Changed<C>`, so its cost is
proportional to how many entities changed. Built-in entity signals that are always present:

| Signal | Meaning |
|---|---|
| `self.age_secs` | Seconds since the pipeline was bound |
| `self.life` | 0..1 progress through `lifetime_ms` (0 without a lifetime) |
| `self.state_t` | 0..1 progress of the current state transition |
| `self.near` | Proximity 0..1 to the nearest point of interest (§4.4) |
| `self.<envelope>` | Any running reaction envelope (§4.2) |

A `Param` resolves `self.*` from the entity, and everything else from the global `Signals`.

### 4.2 Global signals

```rust
#[derive(Resource, Default)]
pub struct Signals(HashMap<SignalId, f32>);

pub trait SignalSource: Send + Sync + 'static {
    fn update(&mut self, time: &Time, out: &mut Signals);
}
```

| Prefix | Plugin | Signals |
|---|---|---|
| `lfo.*`, `timeline.*` | built-in (from `ModulatorSource`) | Declared in the asset's `signals:` block |
| `gamepad.*` | gamepad plugin | `gamepad.rt`, `gamepad.lt`, sticks |
| `audio.*` | `AudioAnalysisPlugin` | `audio.sub_bass`, `audio.bass`, `audio.mid`, `audio.high`, `audio.beat` |
| `game.*` | game code | e.g. `game.time_left`, `game.combo` |

All signals are normalised to 0..1. A missing signal reads 0 and logs once.

### 4.3 Triggers: discrete events

A trigger is a named moment that reactions listen to. Plugins map their events to triggers:

```rust
pub trait ReactiveAppExt {
    /// Entity-targeted Bevy event, delivered through an observer on the entity
    fn register_trigger<E: EntityEvent>(&mut self, key: &'static str, strength: fn(&E) -> f32) -> &mut Self;

    /// Buffered message; `route` says which entity it concerns, or a world position for
    /// effects without an owner (e.g. a shot that missed)
    fn register_trigger_message<M: Message>(&mut self, key: &'static str, route: fn(&M) -> TriggerTarget) -> &mut Self;
}

pub enum TriggerTarget {
    Entity(Entity),
    At(Vec3),
    /// Every entity whose pipeline listens to this trigger
    Broadcast,
}
```

```rust
// hunter plugin
app.register_trigger::<TargetHit>("hit", |e| e.damage_ratio)
   .register_trigger::<TargetPopped>("pop", |_| 1.0)
   .register_trigger_message::<HunterClickEvent>("shot", |m| TriggerTarget::At(m.click_position))
   .register_trigger_message::<GameSessionCreated>("session_start", |_| TriggerTarget::Broadcast);
```

Built-in triggers: `bound` (pipeline attached), `state:<name>` (state entered), `finished`
(lifetime or effect ended).

Triggers are delivered as a `ReactionTriggered { target, key, strength }` message, consumed in
`PathPipelineSet::React`. This keeps reactions out of game schedules and lets one frame batch many
triggers.

### 4.4 Points of interest: spatial context

Stages are pure and can't query the world. Spatial reactions get their world context from a
resource that plugins fill each frame:

```rust
/// World positions stages and signals may react to, refreshed in PathPipelineSet::Signals
#[derive(Resource, Default)]
pub struct PointsOfInterest(Vec<Poi>);

pub struct Poi { pub kind: PoiKind, pub position: Vec3, pub owner: Option<Entity> }

pub enum PoiKind { Reticle, Hunter, Impact, Custom(SignalId) }

app.publish_poi::<ReticleMarker>(PoiKind::Reticle);
```

`self.near` is computed from it per entity, and `StageCtx` exposes it so a stage can bend or
attract geometry toward a reticle (the `attract` stage), or push it away from impacts (`repel`).

### 4.5 Reactive states

A pipeline can declare `states:`. Each state overrides the source and/or stage parameters, and the
entity switches between them. The current state comes from:
- **A component:** `state_from: TargetState`, registered with
  `app.bind_state::<TargetState>(|s| s.name())`. It works with `seldom_state` or any enum
  component.
- **Reactions:** `- state: dying` (§4.6).

On a switch, the system bumps the static generation. If `crossfade_ms` is set, the system morphs
the old `UniversalPath` into the new one with the existing `MorphStrategy`, and publishes
`self.state_t`.

### 4.6 Reactions: what happens on a trigger

A reaction is a **curator step tree** (`curator_core::Step<ReactionAction>`), the same control
flow that drives game workflows: `sequence`, `parallel`, `repeat`, `delay` and `wait` around
pipeline actions. A list is a `sequence`. Each trigger starts a new `StepRunner`; instant steps
chain within the frame, `wait: <trigger>` resumes on a later trigger of the same entity
(`finished`, `hit` ...).

```yaml
reactions:
  hit:                                   # trigger key
    - envelope: { name: hit, attack_ms: 5, decay_ms: 250 }      # → self.hit, peak × strength
    - set: { stage: nervous, enabled: true, for_ms: 300 }
  pop:
    - parallel:
        - state: dying
        - spawn: { pipeline: fx/burst, at: self, inherit: [color] }
    - wait: finished                     # the dying state's lifetime ran out
    - emit: { tag: visual_finished }     # the game workflow can now despawn the target
```

| Action | Effect | Status |
|---|---|---|
| `envelope` | Starts an attack/hold/decay curve published as `self.<name>`. Peak is multiplied by the trigger strength unless `use_strength: false`. `mode: restart \| add \| max` | done |
| `set` | Changes stage `params` and/or `enabled` by stage `id`, optionally `for_ms`, then restores | done |
| `emit` | Sends a `CuratorEventEnvelope` (`tag`, `payload`, `source_entity` = this entity), so workflows can `wait_event` on visuals | done |
| `trigger` | Raises another trigger on the same entity | done |
| `state` | Switches the reactive state (§4.5) | P5 |
| `spawn` | Spawns a transient **effect entity** with its own pipeline, at the owner, the trigger position or a POI. It despawns when its lifetime ends | P5 (today: spawn in code with `DespawnOnFinish`) |

Plugins can add actions with `app.register_reaction_action::<T>("key")`, the same way as stages.

**Effect entities** are ordinary pipeline entities with an `EffectOf(owner)` relationship and a
lifetime. They react, render and pass through `Safety` like everything else. The point budget
treats them as lowest priority (§9), so a burst of effects thins out before targets do.

**Example: HunterShotRipple as data.** Today the ripple is a component, a spawn system and an
update system. With reactions it becomes one asset and one trigger registration:

```yaml
# assets/pipelines/fx/ripple.pipeline.yaml
name: fx/ripple
lifetime_ms: 600
source:
  shape: { builtin: circle, radius_m: 0.01 }
stages:
  - scale: { id: grow, factor: { signal: self.life, range: [1.0, 40.0] } }
  - fade:  { amount: { signal: self.life, range: [1.0, 0.0] } }
```

```yaml
# on the scene pipeline
reactions:
  shot:
    - spawn: { pipeline: fx/ripple, at: trigger }
```

## 5. Tier 1: Sources

A source creates geometry and writes `BasePath`. Each entity, or each reactive state, has at most
one source.

```rust
#[derive(Component, Default, Deref, DerefMut)]
pub struct BasePath(pub UniversalPath);
```

| Source | Input | Rebuilds on |
|---|---|---|
| `shape` | `Handle<ShapeAsset>`, or a `builtin` (circle, balloon, diamond, crosshair, rect) | Asset change or state switch |
| `fill` | Region + `FillStrategy` + params (see [FILL_STRATEGIES_PLAN.md](FILL_STRATEGIES_PLAN.md)) | Region, params or state change. Every frame if a param is bound |
| `morph` | Two keyframe sources + `Param t` + `MorphStrategy` | Every frame while `t` is bound, otherwise on change |
| `text` | String + font + size | Text or state change |
| `inline` | A `UniversalPath` written by game code | Whenever game code changes it |

Entities with only a `UniversalPath` and no `PipelineRef` skip tiers 1 and 2 and behave exactly as
they do today, so calibration paths, minigames and text keep working during migration.

```rust
pub trait FillStrategy: Send + Sync + 'static {
    fn generate(&self, region: &FillRegion, ctx: &StageCtx, out: &mut UniversalPath);
}
```

## 6. Tier 2: Stage Stack

### 6.1 The stage contract

```rust
pub enum StageKind { Static, Dynamic }

/// One pluggable step of a pipeline. Pure: reads `input`, writes `out` (cleared by the caller,
/// capacity reused, no steady-state allocation). Never touches the ECS world directly.
pub trait PathStage: Send + Sync + 'static {
    fn kind(&self) -> StageKind;
    fn apply(&mut self, input: &UniversalPath, ctx: &StageCtx, out: &mut UniversalPath);
}

pub struct StageCtx<'a> {
    pub delta_secs: f32,
    pub elapsed_secs: f32,
    /// self.* first, then global
    pub signals: SignalView<'a>,
    pub poi: &'a PointsOfInterest,
    /// The entity's world transform, so stages can relate local geometry to POIs
    pub transform: &'a GlobalTransform,
    pub scene: &'a SceneSetup,
}
```

A stage with any parameter bound to a signal is `Dynamic`, whatever `kind()` returns. A stage
switched by a reaction `set` invalidates the static cache once.

### 6.2 Per-entity state

```rust
#[derive(Component)]
pub struct PipelineRef(pub Handle<PipelineAsset>);

#[derive(Component)]
pub struct PathPipeline {
    pub stages: Vec<StageInstance>,
    pub state: StateId,
    /// Running envelopes and timed `set`s from reactions
    reactions: ReactionState,
    split: usize,
    static_gen: u64,
    cached_gen: u64,
    scratch: [UniversalPath; 2],
}

pub struct StageInstance {
    /// Stable id from the asset; reactions target it, and state survives reloads
    pub id: StageId,
    pub enabled: bool,
    pub stage: Box<dyn PathStage>,
}

#[derive(Component, Default)]
pub struct StaticPath(pub UniversalPath);
```

### 6.3 Evaluation

`run_stage_stacks` in `PathPipelineSet::Stages`, with `par_iter_mut` across entities:

1. If `static_gen != cached_gen`, run the static prefix `BasePath → StaticPath`.
2. Run the dynamic suffix `StaticPath → scratch → … → UniversalPath`.
3. If there is no dynamic stage, copy into `UniversalPath` only when it changed.

Every stage starts from an immutable input, so nothing accumulates. A static stage after a dynamic
one is still correct but runs every frame, and the loader logs a warning.

### 6.4 Registration

```rust
pub trait PathPipelineAppExt {
    fn register_path_stage<T: PathStage + DeserializeOwned>(&mut self, key: &'static str) -> &mut Self;
    fn register_fill_strategy<T: FillStrategy + DeserializeOwned>(&mut self, key: &'static str) -> &mut Self;
    fn register_signal_source<T: SignalSource>(&mut self, prefix: &'static str) -> &mut Self;
    fn register_reaction_action<T: ReactionAction + DeserializeOwned>(&mut self, key: &'static str) -> &mut Self;
}
```

Factories go from a YAML value to a boxed instance. Params derive `Deserialize` and `Reflect`.
Unknown keys, unknown stage ids in reactions, or unknown triggers are load errors naming the file
and path. The previous version stays live.

### 6.5 Built-in stages

| Stage | Kind | Typical reactive use |
|---|---|---|
| `scale`, `rotate`, `translate` | static/bound | Pulse on `self.hit`, spin with `self.speed` |
| `fade`, `color_shift`, `color_lerp` | static/bound | Flash white on hit, redden with low `self.health` |
| `wave_displace` | dynamic | Ripple with `audio.bass` or `self.hit` |
| `wobble`, `jitter` | dynamic | Shake on hit, nervous when `self.near` is high |
| `attract`, `repel` | dynamic | Bend toward the reticle, recoil from an impact POI |
| `trim` | static/bound | Draw-on/draw-off (0..1 of path length), spawn and death animations |
| `dash` | static/bound | Marching dashes with phase bound to time |
| `corner_fillet`, `simplify`, `clip` | static | Geometry cleanup |
| `shatter` | dynamic | Split strokes into fragments flying apart (pop) |

### 6.6 ECS escape hatch

A plugin that needs full world access adds a normal system
`.in_set(PathPipelineSet::Stages).after(run_stage_stacks)` that writes `UniversalPath`. It must be
idempotent per frame. Before using it, prefer a trigger, a bound signal or a POI. Those three cover
almost every reactive case and keep the visual editable in data.

## 7. Pipeline Assets

### 7.1 Format

```yaml
# assets/pipelines/hunter/balloon.pipeline.yaml
name: hunter/balloon
description: Rising balloon that flinches on hit, reddens as it weakens, and bursts on pop

signals:
  lfo.bob: { sine: { frequency_hz: 0.8 } }

state_from: TargetState            # optional; reactions can also switch states
states:
  idle:
    source: { shape: { builtin: balloon } }
    stages:
      - trim:        { id: draw_on, amount: { signal: self.age_secs, range: [0.0, 1.0] } }   # draws on over the first second
      - translate:   { id: bob, y: { signal: lfo.bob, range: [-0.02, 0.02] } }
      - color_lerp:  { id: weak, to: [255, 40, 40], amount: { signal: self.health, range: [1.0, 0.0] } }
      - scale:       { id: flinch, factor: { signal: self.hit, range: [1.0, 1.25] } }
      - wobble:      { id: nervous, amplitude_m: { signal: self.near, range: [0.0, 0.01] } }
  dying:
    crossfade_ms: 80
    stages:
      - shatter: { pieces: 8, speed_m_s: { signal: self.state_t, range: [0.0, 2.0] } }
      - fade:    { amount: { signal: self.state_t, range: [1.0, 0.0] } }
    lifetime_ms: 400

reactions:
  hit:
    - envelope: { name: hit, attack_ms: 5, decay_ms: 250, scale: strength }
  pop:
    - state: dying
    - spawn: { pipeline: fx/ripple, at: self, inherit: [color] }
  finished:
    - emit: VisualFinished

frame:                              # optional; can only lower limits (§9)
  priority: normal                  # effects default to low
```

`extends: other_pipeline` inherits everything, and overrides states, stages and reactions by key
or `id`.

### 7.2 Loading and hot reload

- `PipelineAsset` has an `AssetLoader` that parses YAML into a `PipelineSpec` (plain serde data) and
  validates it against all registries.
- Bevy's `file_watcher` in development builds raises `AssetEvent::Modified`, and the `Bind` set
  rebuilds affected instances. Stage instances, envelopes and effect entities with unchanged ids
  keep their state. A file that fails validation keeps the last good version and reports through
  the status plugin.

### 7.3 Binding to entities

- **By name (implemented):** game code inserts `UsePipeline::new("snake/head")`. The plugin loads
  `pipelines/snake/head.pipeline.yaml` from the asset folder (hot-reloadable) and uses the copy
  embedded with `app.embed_pipeline(include_str!(...))` until the file loads, or for good when it
  is missing. Changing the component switches the pipeline. A file's `name` must equal its path
  under `assets/pipelines`; `common/tests/pipeline_assets_test.rs` checks this and compiles every
  shipped file.
- **Explicitly:** game code inserts a `PipelineRef` (asset handle with optional fallback, or an
  inline spec).
- **By shape tag:** a Shape v2 tag `pipeline:<name>` picks a default on spawn.
- **At runtime:** the menu, keyboard or network sends a `SetPipeline { entity, name }` message.
- **Scene pipeline:** a pipeline bound to the `SceneEntity` receives `Broadcast` and `At(..)`
  triggers that have no owner, such as shots that missed or session start.

## 8. Game Integration

What a minigame does under this design:

1. Spawn targets with a transform, game components (`TargetHealth`, `TargetState`), a
   `BasePath` and a `UsePipeline`. Game code updates `BasePath`, never `UniversalPath`, which the
   pipeline owns.
2. Register its triggers, signal bindings and POIs in its plugin (§4.1–4.4), and embed its
   pipeline files.
3. Raise domain events (`SnakeGemEatenEvent`, `TargetPopped`) and update components as gameplay
   happens. Prefix trigger keys with the game (`snake.gem_eaten`), since broadcasts reach every
   pipeline.
4. Optionally wait for `VisualFinished` before despawning.

### 8.1 Pilots

| Game | Entity | Pipeline | Reacts to |
|---|---|---|---|
| Hunter | shot ripple | `hunter/ripple` | its own lifetime: grows 7× over 100 ms, then despawns (`DespawnOnFinish`). Replaced `HunterShotRipple` and its system |
| Snake | head | `snake/head` | `snake.gem_eaten` → gulp (scale envelope); `snake.game_over` → dims |
| Snake | body | `snake/body` | `snake.gem_eaten` → white flash; global `snake.speed` → slither ripple grows with game speed; `snake.game_over` → shakes and dims |
| Snake | gem | `snake/gem` | local `spin`/`breathe` signals; `bound` and `snake.gem_eaten` → draws itself on at the new cell |
| Snake | SNAKE / GAME OVER text | `snake/title`, `snake/game_over` | own lifetime: write-on, fade-out, despawn. Replaced the timer component and system |

Snake publishes `snake.speed` (0 at the starting pace, 1 at top speed) as a global signal, and
maps its `SnakeGemEatenEvent` / `SnakeGameOverEvent` messages to broadcast triggers with
`register_trigger_message`. Its grid logic is unchanged.

Hit testing stays in game logic, on the game's own shape (`Target.path` or a radius). An optional
`HitShape::FromVisual` can test against the final `UniversalPath`, so a target that reacts by
growing is also easier to hit. It's off by default, because gameplay then depends on visuals.

## 9. Tier 3: Frame Pipeline

| Set | Does | Source today |
|---|---|---|
| `Collect` | Visibility, calibration filter, transform, cull, project, `StrokeBuilder` → `LaserFrame` | `update_point_buffer` first half |
| `Order` | Order strokes across entities to shorten jumps (plugin slot) | new |
| `Kinematics` | `laserlogic::compose_frame` with `projector.optimize` | `update_point_buffer` second half |
| `Safety` | Enforces the hard limits below | new |
| `Emit` | Convert to `HeliosPoint`, write `LaserPointBuffer` | existing |

**Safety rules.** The `Safety` set always runs, with no enable flag:
1. Clamp lit and blank step size to `max_lit_step_dac` / `max_blank_step_dac`.
2. Force colour to 0 on jump points.
3. Cap consecutive lit holds at one position (no stationary burn).
4. **Point budget:** a frame over `dac_pps / tick_hz` points is reduced in priority order. First
   drop `low` effects, then thin density, then drop `normal` entities. The frame is never stretched
   silently. Reactions can spawn many effects, so this rule matters for reactivity.
5. The shutter interlock stays in the DAC thread.

A pipeline's `frame:` block can only lower limits (scales `<= 1.0`, longer holds) and set
priority.

## 10. Who Owns Which Setting

| Setting | Lives in |
|---|---|
| PPS, DAC, galvo model, diode delays, step caps | `projector.*` in `server.yaml` |
| Look, reactions, states, signal bindings | `assets/pipelines/**/*.pipeline.yaml` |
| Which triggers, signals and POIs exist | Plugin registration code |
| Which pipeline an entity uses | Game code, shape tag, or runtime message |

`config/pipeline.yaml` from the first draft is replaced by pipeline assets. Its galvo and diode
values duplicated `projector.optimize` and are dropped.

## 11. Code Layout

Everything a game crate needs lives in `common::pipeline`, because minigames depend on `common`,
not on `server-core`. Only the plugin module touches the ECS; the rest is plain Rust shared with
`dac-test` and tests.

| Crate | Module | Contents |
|---|---|---|
| `curator-core` | `tree.rs` | `Step<A>`, `StepRunner<A>`, `WaitSpec`: generic step tree and its runner, used by reactions (and later by `CuratorStep`) |
| `common` | `pipeline/spec.rs` | `PipelineSpec` (YAML via figment), `ReactionAction`, `EnvelopeSpec`, `SetSpec`, `EmitSpec`, `LocalSignal`, `Clock` |
| `common` | `pipeline/compile.rs` | `compile(spec, registry)` → `CompiledPipeline`, all errors with locations |
| `common` | `pipeline/instance.rs` | `PipelineInstance`: source, cached static prefix, dynamic suffix, reactions, envelopes, timed sets, hot rebind |
| `common` | `pipeline/stage.rs`, `param.rs`, `signals.rs`, `registry.rs` | `PathStage`, `PathSource`, `StageCtx`, `Param`, `Signals`, `EntitySignals`, `PipelineRegistry` |
| `common` | `pipeline/stages.rs`, `sources.rs` | Built-in stages (§6.5) and the `shape` source |
| `common` | `pipeline/plugin.rs` | `PathPipelinePlugin`, `PathPipelineSet`, `PipelineRef`, `BasePath`, `DespawnOnFinish`, `ReactionTriggered`, `ReactiveAppExt`, asset loader |
| `laserlogic` | `fill/*.rs` | Fill strategies (P7) |
| `server-core` | `lib.rs` | Adds `AssetPlugin` (`runtime.assets_dir`) and `PathPipelinePlugin` |
| `server-core` | `plugins/projector.rs` | Runs after `PathPipelineSet::Stages`; later `LaserFrameSet` split and `Safety` |
| `minigames/*` | their plugins | Register triggers and bindings, ship pipelines in `assets/pipelines/<game>/` |

YAML is parsed with `figment`, which is already in the workspace (§13). Hot reload needs the
`server-core/hot_reload` cargo feature (Bevy `file_watcher`).

## 12. Implementation Phases

| Phase | Work | Done when | Status (2026-11-05) |
|---|---|---|---|
| **P1: Correct stage model** | `PathStage` `(input, ctx, out)`, `BasePath`, `StaticPath`, `run_stage_stacks`. Port `PathModifierPipeline` users. Remove in-place modification | A dynamic displacement over 100 frames stays within amplitude. A static stage runs once until its input changes | done |
| **P2: Registry + assets** | `AssetPlugin` on the headless server with a `runtime.assets_dir` setting, registries, `PipelineAsset` loader (figment) + validation, `PipelineRef`, hot reload via `file_watcher` dev feature | Editing a pipeline changes the projection live. A broken file keeps the last good version | done, except `xtask pipeline check` |
| **P3: Signals** | `Signals`, `EntitySignals`, `Param` with `self.*`, `bind_signal`, built-in LFO/timeline/gamepad, `self.life`, `self.age_secs`. Port `PathModulator` to the morph source | `color_lerp` bound to `self.health` follows a component in a headless test | done, except gamepad signals and the morph source |
| **P4: Reactions** | Triggers (`register_trigger`, `register_trigger_message`), envelopes, `set`, `emit`, `finished` | `TargetHit` makes the entity flash and decay within one frame of the event | done for message and envelope triggers; `EntityEvent` observers open |
| **P5: States + effects** | `states:`, `state_from`, crossfade, `spawn` effects with `EffectOf` and lifetime, effect priority | The hunter balloon pops through a dying state and a ripple effect. `HunterShotRipple` code is deleted | Hunter ripple and all Snake visuals migrated (§8.1); `states`, `spawn` open |
| **P6: Spatial** | `PointsOfInterest`, `self.near`, `attract`/`repel` | A target wobbles when the reticle approaches | open |
| **P7: Fill + path model** | Blank edges, fillets, dwell in ms, fill sources (FILL_STRATEGIES_PLAN) | Fills react, e.g. a raster drains with `self.health` | open |
| **P8: Frame pipeline** | `LaserFrameSet` split, `Safety` with priority budget | Safety tests pass. 50 simultaneous effects stay within budget | open |
| **P9: Audio + tooling** | `AudioAnalysisPlugin`, `dac-test fill-demos` on `laserlogic::evaluate`, trigger replay in `dac-test` | Bench runs reactive demos with synthetic triggers | open |

## 13. Decisions (2026-11-05)

| Topic | Decision | Consequence |
|---|---|---|
| Asset loading | Add Bevy `AssetPlugin` to the headless server (it runs on `MinimalPlugins` today). Enable `file_watcher` behind a dev cargo feature | The assets root becomes a `runtime.assets_dir` setting (dev `./assets`, Pi `/opt/lasertargets/assets`). Shapes and fonts can move onto the asset server later, replacing hunter's hard-coded font paths |
| YAML parsing | Reuse `figment` (already a dependency). `serde_yaml` is archived | Check in P2 that untagged `Param` deserialises through figment's value model. If it doesn't, fall back to a maintained YAML crate |
| Old modifier API | Replace `PathModifier`/`PathModifierPipeline` outright in P1. Only `server-core/plugins/path.rs` and its tests use it | No compatibility shim |
| Pilot game | **Hunter** for P4–P6 | `HunterShotRipple`, balloon pop and the title announcement move to `assets/pipelines/hunter/`, and their drawing code is deleted |
| Networking | **Server-authoritative.** The server runs all pipelines and reactions, and the terminal keeps drawing the finished `UniversalPath`s | No new protocol for triggers. The terminal shows exactly what the laser draws |
| Wire format | `PathPoint.blank` (P7) changes the bincode format, and there's no protocol version | Server and terminal must be deployed together from P7 on. Consider adding a protocol version handshake in the same change |
| Clock | Per pipeline, `clock: game \| real`, default `game` | Game visuals freeze when a session pauses. Calibration and menu pipelines set `clock: real` |
| Shape v2 `blank` | Optional field, no version bump | The shape editor is updated in the same change |
| Hit-testing | Stays in game logic. `HitShape::FromVisual` is postponed | — |
| Units | Metres in the entity's local frame for all `*_m` params, matching `scene_dimension` | — |
| Performance | Measure the stage runner on the Raspberry Pi in P1, then set per-stage and per-fill cost limits from the numbers | Limits are recorded in this doc after P1 |

Before P1: commit or stash the current uncommitted work on `master`, then start the pipeline work
on a feature branch.
