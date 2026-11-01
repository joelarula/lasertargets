# Gamepad User Guide & Controller Reference

LaserTargets supports full physical gamepad control for headless menu navigation, switching minigames, and live physical laser rig/canvas calibration.

---

## 🎮 Controller Layout & Button Mapping

Controls are mapped using standard Xbox / PlayStation / DirectInput conventions:

| Function | Xbox | PlayStation | DirectInput / Bevy `Btn` |
| :--- | :--- | :--- | :--- |
| **Toggle Calibration Mode** | **Y** | **△ Triangle** | `Btn::North` |
| **Cycle Game / Main Menu** | **X** | **□ Square** | `Btn::West` |
| **Switch Target (Rig vs Canvas)** | **Back / View** | **Share / Select** | `Btn::Select` |
| **Boost Speed (2.5x)** | **A** (Hold) | **✕ Cross** (Hold) | `Btn::South` |
| **Precision Speed (0.25x)** | **L3** (Hold Left Thumb) | **L3** (Hold Left Thumb) | `Btn::LeftThumb` |
| **Canvas Center Shift** | **RB / RT / R3** (Hold) | **R1 / R2 / R3** (Hold) | `RightBumper / RightTrigger / RightThumb` |

---

## 🧭 Global Navigation (Available Anytime)

### 1. Cycle Minigames (`X` / `□ Square`)
Tapping `Btn::West` cycles the server through all registered game modes and returns to the menu:
$$\text{Main Menu} \longrightarrow \text{Game 1 (Hunter)} \longrightarrow \text{Game 2 (Snake)} \longrightarrow \text{Main Menu}$$

### 2. Enter / Exit Calibration (`Y` / `△ Triangle`)
Tapping `Btn::North` toggles **Calibration Mode**:
- **Entering Calibration**: Projects visible laser bounding brackets, center anchor, and active client pointer crosshairs.
- **Exiting Calibration**: Despawns all calibration laser overlays and saves configuration in memory.

---

## 🎯 Calibration Modes & Workflow

Once Calibration is active (`Y`), tap **Back / Select** to toggle between **Point A (Physical Rig)** and **Point B (Billboard Canvas)**:

```
          [ POINT A: Physical Rig ]                    [ POINT B: Projection Canvas ]
     Laser Projector + Tracking Camera               Wall / Billboard Reflection Area
        (Position, Height & Optical FOV)             (Canvas Width, Height & Center)
                     \                                      /
                      \ ────────── Distance Z ──────────── /
```

---

### Speed Modifiers ("Gear Shifting")
Hold modifier buttons while moving sticks to adjust sensitivity:
- **Normal Speed (1.0x)**: Default stick movement.
- **🐢 Precision Mode (0.25x)** — **Hold `L3` (Click Left Stick)**:
  Slows down all movements to 25% for millimeter-accurate edge alignment against physical screens or markers.
- **🚀 Boost Mode (2.5x)** — **Hold `A` / `✕ Cross`**:
  Accelerates all movements to 250% for fast repositioning across large rooms.

---

## 📐 Mode A: Projector & Camera Rig (`RigPointA`)

Use Point A to adjust the physical position and optical throw angle of the projector/camera hardware relative to the canvas.

> [!NOTE]
> In Point A, the laser projector and tracking camera are automatically kept pointed directly at the canvas center (`looking_at(billboard_center)`).

### Analog Controls
- **Left Stick Left / Right**: Lateral rig position ($X$).
- **Left Stick Up / Down**: Rig elevation / height ($Y$).
- **Right Stick Up / Down**: Rig distance closer to / further from canvas ($Z$).
- **Right Stick Left / Right**: Optical throw angle / FOV ($15^\circ$ to $90^\circ$).

### Discrete Step Controls (D-Pad)
Tap the D-Pad for precise **0.1m (10 cm)** increments:
- **D-Pad Up / Down**: Height $\pm 0.1\,\text{m}$.
- **D-Pad Left / Right**: Lateral position $\pm 0.1\,\text{m}$.

---

## 🖼️ Mode B: Billboard Canvas Surface (`BillboardPointB`)

Use Point B to adjust the size, bounds, and placement of the laser projection area on the wall.

### Default Mode: Resize Canvas
- **Left Stick Left / Right**: Canvas Width ($X$).
- **Left Stick Up / Down**: Canvas Height ($Y$).
- **Right Stick Up / Down**: Canvas depth / distance along $Z$-axis.
- **Right Stick Left / Right**: Canvas lateral position ($X$).
- **D-Pad Up / Down**: Canvas height $\pm 0.1\,\text{m}$.
- **D-Pad Left / Right**: Canvas width $\pm 0.1\,\text{m}$.

### Center Placement Mode: Hold `RB`, `RT`, or `R3`
Holding any right modifier (**Right Bumper**, **Right Trigger**, or **Right Stick Click**) shifts from resizing to translating:
- **Left Stick Left / Right**: Shift canvas center Left / Right ($X$).
- **Left Stick Up / Down**: Shift canvas center Up / Down ($Y$).

---

## 👁️ Visual Laser Overlays & Anchors

When Calibration Mode is **ON**, the laser engine renders three helpers:

1. **Corner L-Brackets (`ProjectionAreaRectangle`)**:
   - 4 green corner brackets with high corner dwell ($14\,\text{samples}$) for 100% sharp 90-degree corners.
   - Defines the maximum playable bounding box on the wall.
2. **Center Anchor Crosshair (`CalibrationCenterCrosshair`)**:
   - Green crosshair locked at the exact mathematical center $(0, 0, 0)$ of the canvas.
   - Use this to align the physical center of your target projection surface.
3. **Client Mouse Crosshairs (`CalibrationCrosshair`)**:
   - Cyan crosshairs indicating mouse pointer positions of connected client terminals.

---

## 💡 Calibration Tips

1. **Start with Canvas Size (Point B)**:
   - Switch to Point B (`Select`).
   - Use the Left Stick to resize the corner L-brackets until they align with the physical projection wall or screen.
2. **Fine-tune with Precision Mode**:
   - Click and hold **`L3`** while adjusting with the Left Stick or tap the **D-Pad** for 10 cm adjustments to snap directly to screen borders.
3. **Reposition the Center**:
   - Hold **`RB`** and use the Left Stick to move the entire canvas without altering its dimensions.
4. **Lock in Settings**:
   - Tap **`Y`** (`Btn::North`) to exit Calibration Mode. All overlays will despawn and the calibrated dimensions will be applied to all minigames.

