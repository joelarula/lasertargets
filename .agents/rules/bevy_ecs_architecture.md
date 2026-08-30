# Bevy ECS Architectural Guidelines for Token Efficiency & Maintainability

To maintain high AI performance, clean separation of concerns, and token efficiency, follow these Bevy ECS conventions:

## 1. Schema / Contract Separation
- Keep pure data definitions (`Component`, `Resource`, `Event`, `States`) separated from execution logic.
- Prefer dedicated modules (`components.rs`, `events.rs`, `resources.rs`, `types.rs`) or isolate definitions at the top of domain modules.
- Avoid mixing large system algorithms into pure struct definition files.

## 2. Granular Feature Plugins
- Structure functionality into small, focused Bevy `Plugin` implementations rather than monolithic 500+ line plugins.
- Group related systems, resources, and events into self-contained feature plugins (e.g. `CollisionPlugin`, `InputPlugin`, `HardwarePlugin`).

## 3. Focused, Single-Responsibility Systems
- Keep individual Bevy system functions short and focused (< 40–50 lines per system).
- Use explicit query filters (`With<T>`, `Without<T>`, `Changed<T>`, `Added<T>`) to narrow system scope.
- Split multi-step workflows into chained systems or Bevy `Event` / `Observer` triggers.

## 4. Declarative Scheduling with SystemSets & States
- Use explicit `SystemSet` enums and `States` (e.g. `in_state(GameState::Playing)`) for execution order and conditions rather than embedding manual branching inside system bodies.

## 5. DTOs & Public Contracts in `common`
- Shared network packets, math structures, and laser point formats belong in `common`.
- Minimize cross-crate internal dependencies by implementing `From`/`Into` conversions at the crate boundaries.
