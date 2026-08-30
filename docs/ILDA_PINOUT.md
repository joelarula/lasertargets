# 🔌 ILDA DB25 to RJ45 (Cat5/Cat6) Wiring & Pinout Guide

This document provides the hardware pinout mapping for connecting an **ILDA DB25 Standard Projector Port** to an **RJ45 (Cat5/Cat6 Ethernet Cable)** adapter (e.g. Pangolin ILDA-over-Cat5, EtherDream, Helios, or DIY RJ45 breakout boards).

### Official DB25 ILDA Pinout (Male & Female Views)
![Official ILDA DB25 Pinout](images/ilda_pinout_official.png)

---

## 1. Standard ILDA DB25 Pinout Reference

| Pin # | Signal Name | Voltage Range | Description |
| :--- | :--- | :--- | :--- |
| **1** | **X+** | -5V to +5V | Galvo X Differential Positive |
| **14** | **X-** | +5V to -5V | Galvo X Differential Negative |
| **2** | **Y+** | -5V to +5V | Galvo Y Differential Positive |
| **15** | **Y-** | +5V to -5V | Galvo Y Differential Negative |
| **3** | **Intensity+** | 0V to +5V | Master Laser Intensity / Unblanking |
| **16** | **Intensity-** | 0V / GND | Intensity Differential Return |
| **4** | **Interlock A** | Loop to Pin 17 | Hardware Safety Shutter Loop |
| **17** | **Interlock B** | Loop to Pin 4 | Hardware Safety Shutter Loop |
| **5** | **Red+** | 0V to +5V | Red Laser Diode Modulation |
| **18** | **Red-** | 0V / GND | Red Differential Return |
| **6** | **Green+** | 0V to +5V | Green Laser Diode Modulation |
| **19** | **Green-** | 0V / GND | Green Differential Return |
| **7** | **Blue+** | 0V to +5V | Blue Laser Diode Modulation |
| **20** | **Blue-** | 0V / GND | Blue Differential Return |
| **25** | **GND** | 0V Reference | Common Signal Ground |

---

## 2. DB25 to RJ45 Breakout Wiring Diagram

Your cable sequence: **Orange-White, Orange, Green-White, Blue, Blue-White, Green, Brown-White, Brown** (Pins 1 to 8 in order looking at top of plug):

```
         DB25 Male Pinout                    RJ45 Plug Pinout
  +-------------------------------+         +----------------------------+
  | Pin  1: Galvo X+              | ------> | Pin 1: Orange-White        |
  | Pin 14: Galvo X-              | ------> | Pin 2: Orange              |
  | Pin  2: Galvo Y+              | ------> | Pin 3: Green-White         |
  | Pin 15: Galvo Y-              | ------> | Pin 4: Blue (Galvo Y-)     |
  | Pin  5: Red Laser (+5V)       | ------> | Pin 5: Blue-White          |
  | Pin  6: Green Laser (+5V)     | ------> | Pin 6: Green               |
  | Pin  7: Blue Laser (+5V)      | ------> | Pin 7: Brown-White         |
  | Pin 25 & Interlock (GND/Loop) | ------> | Pin 8: Brown               |
  +-------------------------------+         +----------------------------+
```

---

## 2.1 DB25 to RJ45 Master Soldering Table

| RJ45 Pin # | Wire Color | DB25 Solder Pin # | Signal Function | Functional Group & Notes |
| :--- | :--- | :--- | :--- | :--- |
| **Pin 1** | 🟧 **Orange-White** | **Pin 1** | **Galvo X+** (Positive X) | ↕️ Galvo Motion Pair 1 |
| **Pin 2** | 🟧 **Solid Orange** | **Pin 14** | **Galvo X-** (Negative X) | ↕️ Galvo Motion Pair 1 *(Fixes Squeezed Width)* |
| **Pin 3** | 🟩 **Green-White** | **Pin 2** | **Galvo Y+** (Positive Y) | ↕️ Galvo Motion Pair 2 |
| **Pin 4** | 🟦 **Solid Blue** | **Pin 15** | **Galvo Y-** (Negative Y) | ↕️ Galvo Motion Pair 2 *(Galvo Y Differential Return)* |
| **Pin 5** | 🟦 **Blue-White** | **Pin 5** | **Red Laser** (+5V) | 🔴 Laser Color Diode Modulation |
| **Pin 6** | 🟩 **Solid Green** | **Pin 6** | **Green Laser** (+5V) | 🟢 Laser Color Diode Modulation |
| **Pin 7** | 🟫 **Brown-White** | **Pin 7** | **Blue Laser** (+5V) | 🔵 Laser Color Diode Modulation *(Fixes Purple Tint)* |
| **Pin 8** | 🟫 **Solid Brown** | **Pin 25 & 4-17** | **Common GND & Interlock** | ⚡ Signal Ground & Safety Loop (Short Pin 4 to 17) |

---

## 2.2 The Logical "4 - 3 - 1" Pin Grouping Rule

```text
  +-------------------------------------------------------------------------+
  |  RJ45 Pins 1..4  [ GALVOS ]    ==>  Pin 1: X+,  Pin 2: X-,  Pin 3: Y+,  Pin 4: Y-
  |  RJ45 Pins 5..7  [ COLORS ]    ==>  Pin 5: Red, Pin 6: Green, Pin 7: Blue
  |  RJ45 Pin  8     [ MAIN GND ]  ==>  Pin 8: Common Signal Ground & Interlock
  +-------------------------------------------------------------------------+
```

* **Pins 1..4 (Galvo Motion)**:
  * Pin 1 (Orange-White) $\rightarrow$ DB25 Pin 1 (X+)
  * Pin 2 (Solid Orange) $\rightarrow$ DB25 Pin 14 (X-)
  * Pin 3 (Green-White) $\rightarrow$ DB25 Pin 2 (Y+)
  * Pin 4 (Solid Blue) $\rightarrow$ DB25 Pin 15 (Y-)
* **Pins 5..7 (Laser Colors)**:
  * Pin 5 (Blue-White) $\rightarrow$ DB25 Pin 5 (Red)
  * Pin 6 (Solid Green) $\rightarrow$ DB25 Pin 6 (Green)
  * Pin 7 (Brown-White) $\rightarrow$ DB25 Pin 7 (Blue)
* **Pin 8 (Ground & Safety)**:
  * Pin 8 (Solid Brown) $\rightarrow$ DB25 Pin 25 & Interlock (Jumper Pin 4 to 17)

---

## 🛠️ 3. Troubleshooting & Symptoms Guide

> [!TIP]
> ### Symptom 1: Horizontally Squeezed / Narrow Projection
> * **Cause**: Missing or shorted **DB25 Pin 14 ($X-$)** line (RJ45 Pin 2 Orange). Without $X-$, Galvo $X$ voltage drops in half.
> * **Fix**: Resolder DB25 Pin 14 and verify 0 ohms continuity to RJ45 Pin 2.
>
> ### Symptom 2: Blue Laser Looks Purple
> * **Cause**: Wire swap between $X$-axis or Red diode modulation line (DB25 Pin 5) and Blue line (DB25 Pin 7).
> * **Fix**: Ensure **RJ45 Pin 5 (Blue-White)** connects to **DB25 Pin 5 (Red)** and **RJ45 Pin 7 (Brown-White)** connects to **DB25 Pin 7 (Blue)**.
>
> ### Symptom 3: Laser Output Fails / Mechanical Shutter Closed
> * **Cause**: Open Safety Interlock loop.
> * **Fix**: Solder a small jumper wire between **DB25 Pin 4** and **DB25 Pin 17**.

---

## 4. Hardware Safety & Assembly Instructions

1. **Keep Untwisted Wire Leads Short**:
   * Strip only **$1.5\text{ cm}$** of the outer CAT cable jacket and **$2\text{ mm}$** of insulation from wire tips. Keep wire leads inside the DB25 hood as short as possible to prevent noise pickup.
2. **Insulate Every Solder Cup**:
   * Use **$1.5\text{ mm}$ heat-shrink tubing** on all DB25 solder cups to prevent bare copper from touching adjacent pins.
3. **Safety Interlock Loop**:
   * Pins **4** and **17** on the DB25 connector **must be shorted together** (or wired to an emergency E-Stop kill switch).
4. **Earth Grounding**:
   * Connect **DB25 Pin 25** to Common Signal Ground and ensure your laser projector chassis is connected to a 3-prong Earth Ground wall outlet to eliminate static discharge.
