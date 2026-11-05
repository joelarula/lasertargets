# Camera Stream & Vision Tracking Plan

## 1. Optical Setup & Dual-Goal Architecture

In the LaserTargets system, the **vision camera (Sony IMX219)** and the **laser projector (Helios DAC)** observe and illuminate the **exact same physical projection scene** (a wall, projection screen, or target array).

```
                      ┌────────────────────────────────┐
                      │   Shared Physical Scene / Wall │
                      │                                │
                      │      (x) Projected Target      │
                      │       *  Player Laser Hit Dot  │
                      └──────────────▲───▲─────────────┘
                                     │   │
                         Projected   │   │  Reflected
                        Laser Light  │   │  Optical Scene
                                     │   │
           ┌─────────────────────────┴┐ ┌┴─────────────────────────┐
           │   Helios Laser DAC       │ │  Sony IMX219 (CSI-2)     │
           │   Galvo Space: [-1, 1]   │ │  Pixel Space: [W, H]     │
           └──────────────────────────┘ └──────────────────────────┘
```

The system has **two distinct core goals**:

1. **Goal 1: Algorithmic Visual Event Detection (Server-Side, High-Speed Loop)**:
   - Real-time computer vision running locally on the Raspberry Pi 4.
   - Sub-15 ms detection of visual events: player laser hits on targets, beam reflection tracking, and automatic homography calibration anchor detection.
   - Dispatches zero-allocation Bevy ECS events directly into game logic (`Hunter`, `Snake`, `TargetManager`).

2. **Goal 2: Low-Latency Visual Review Stream (Server-to-Terminal Client Loop)**:
   - Live video stream sent from the Raspberry Pi 4 server across the network to the operator's Terminal / UI client.
   - Uses the Pi 4's on-chip hardware video encoder (`bcm2835-codec` / `/dev/video11`) to prevent CPU bottlenecks.
   - Overlays detected event markers, calibration grids, and detection thresholds in the terminal UI for operator visual review and alignment verification.

---

## 2. Hardware Specifications & Operating Profiles

From the live hardware probe (`cargo xtask camera --detailed`), the hardware topology on the Raspberry Pi 4 is:

### 2.1. Vision Sensor (Sony IMX219 / Pi Camera Module v2)
* **Bus Interface**: MIPI-CSI2 2-lane ribbon cable directly to BCM2711 Unicam controller (`/dev/video0`).
* **Active Modes for LaserTargets**:

| Sensor Mode | Resolution | Max FPS | FoV Crop | Frame Time | Role in System |
|---|---|---|---|---|---|
| **Hit Tracking Mode** | **`640x480`** | **200.16 FPS** | 2x2 crop | **~5.0 ms** | **Primary loop for ultra-low latency laser hit detection** |
| **Calibration Mode** | **`1640x1232`** | **81.07 FPS** | Full (2x2 bin) | **~12.3 ms** | **High-accuracy 4-point homography calibration & alignment** |
| **Preview Stream Mode** | `1280x720` | 60.00 FPS | 16:9 crop | ~16.6 ms | Video stream sent over network to Terminal |

### 2.2. Hardware Silicon Accelerators
* **Hardware ISP (`bcm2835-isp`, `/dev/video13–16`)**: Demosaicing, lens shading correction, and Bayer-to-YUV/RGB format conversion in silicon directly to DMA buffers (0% CPU).
* **Hardware Codec (`bcm2835-codec`, `/dev/video11`, `/dev/video31`)**: Hardware **MJPEG** and **H.264** encoding engine. Compresses 30–60 FPS preview streams with zero CPU load on the laser timing thread.

---

## 3. Dual-Pipeline Software Architecture

```
┌────────────────────────────────────────────────────────────────────────┐
│                        RASPBERRY PI 4 SERVER                           │
│                                                                        │
│   ┌────────────────────────────────────────────────────────┐           │
│   │  Sony IMX219 Camera ──► Hardware ISP (bcm2835-isp)     │           │
│   └───────────────────────────┬────────────────────────────┘           │
│                               │ DMA YUV/RGB Frames (80-200 FPS)        │
│                               ▼                                        │
│   ┌────────────────────────────────────────────────────────┐           │
│   │              Vision Worker Thread                      │           │
│   │  - Fixed exposure: 3,000 µs (suppresses ambient light) │           │
│   │  - Fixed gain: 1.0 (prevents laser spot bloom)         │           │
│   │  - Color thresholding: Green / Red laser isolation     │           │
│   │  - Centroid calculation: sub-pixel moment (cx, cy)     │           │
│   │  - Homography transformation: Camera(px) ──► Galvo[-1,1│           │
│   └───────────────┬────────────────────────┬───────────────┘           │
│                   │                        │                           │
│     Event Packets │ (80-200 Hz)            │ Preview Frames (20-30 Hz) │
│                   ▼                        ▼                           │
│   ┌────────────────────────┐  ┌────────────────────────────┐           │
│   │   Bevy ECS Systems     │  │ Hardware Encoder (/video11)│           │
│   │   - LaserHitEvent      │  │ - MJPEG / H.264 encode     │           │
│   │   - Minigame collision │  │ - Packs telemetry overlays │           │
│   │   - Score / Animation  │  └────────────┬───────────────┘           │
│   └────────────────────────┘               │                           │
└────────────────────────────────────────────┼───────────────────────────┘
                                             │ Network Packets (QUIC/UDP)
                                             ▼
┌────────────────────────────────────────────────────────────────────────┐
│                        REMOTE TERMINAL CLIENT                          │
│                                                                        │
│   ┌────────────────────────────────────────────────────────┐           │
│   │ Terminal Review Panel (egui / Bevy 2D)                 │           │
│   │  - Live video texture of the physical scene            │           │
│   │  - Layer 1: Raw camera view                            │           │
│   │  - Layer 2: Detection threshold heatmap & laser spots  │           │
│   │  - Layer 3: Projector homography bounds wireframe      │           │
│   │  - Layer 4: Live projected targets & hit markers       │           │
│   └────────────────────────────────────────────────────────┘           │
└────────────────────────────────────────────────────────────────────────┘
```

---

## 4. Goal 1: Algorithmic Visual Event Detection Details

### 4.1. Optical Exposure Tuning (Isolating the Laser)
By default, auto-exposure attempts to brighten the room, turning a laser spot into a large white blown-out circle.
- **Fixed manual shutter**: `2,000 µs – 5,000 µs`.
- **Low analog gain**: `1.0` (zero gain boost).
- **Result**: The ambient room drops to near-black; only high-energy laser reflections remain visible as sharp, isolated peaks.

### 4.2. Wavelength Separation & Sub-Pixel Centroid
- **Color differentiation**:
  $$\text{LaserIntensity}_{\text{Green}} = G - \max(R, B)$$
  $$\text{LaserIntensity}_{\text{Red}} = R - \max(G, B)$$
- **Sub-pixel precision**: When intensity exceeds threshold $T$, compute centroid via image moments:
  $$C_x = \frac{\sum x \cdot I(x, y)}{\sum I(x, y)}, \quad C_y = \frac{\sum y \cdot I(x, y)}{\sum I(x, y)}$$

### 4.3. Planar Homography Transformation
Both the camera and the laser galvo project onto the same flat 2D plane. A $3 \times 3$ planar homography matrix $H$ maps camera pixel coordinates $(u, v, 1)^T$ into normalized projector coordinates $(x_g, y_g, 1)^T \in [-1.0, 1.0]^2$:

$$\begin{bmatrix} x_w \\ y_w \\ w \end{bmatrix} = H \begin{bmatrix} u \\ v \\ 1 \end{bmatrix}, \quad x_g = \frac{x_w}{w}, \quad y_g = \frac{y_w}{w}$$

- **Automatic Calibration Event**:
  1. Server projects 4 corner dots sequentially: $(-0.8, 0.8)$, $(0.8, 0.8)$, $(0.8, -0.8)$, $(-0.8, -0.8)$.
  2. Camera identifies the 4 spot centroids $(u_i, v_i)$.
  3. Server solves $H$ using the Direct Linear Transform (DLT) and persists it to `config/camera_calibration.yaml`.

---

## 5. Goal 2: Live Review Stream & Network Protocol

### 5.1. Hardware-Accelerated Video Compression
To prevent network congestion and CPU starvation on the Pi:
- The vision worker taps a sub-sampled stream at **20–30 FPS**.
- Frames are piped into `/dev/video11` (`bcm2835-codec`), producing compressed **MJPEG** frames in silicon.
- Bandwidth per stream: **~1.5 – 3.0 Mbps** (easily manageable over standard 2.4/5GHz Wi-Fi or Ethernet).

### 5.2. Network Protocol Extensions (`common/src/network.rs`)

```rust
/// Telemetry marker for detected visual events sent alongside preview frames
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisualEventMarker {
    /// Normalized coordinates in camera space [0.0, 1.0]
    pub camera_norm_pos: Vec2,
    /// Projected coordinates in laser galvo space [-1.0, 1.0]
    pub galvo_pos: Vec2,
    /// Event classification (e.g. PlayerHit, CalibrationDot, StrayReflection)
    pub kind: VisualEventKind,
    /// Optical energy / confidence score
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VisualEventKind {
    PlayerLaserHit,
    CalibrationAnchor(u8),
    TargetOcclusion,
}

/// Network message for live visual review stream
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CameraStreamFrame {
    pub timestamp: u64,
    pub width: u32,
    pub height: u32,
    /// Encoded JPEG buffer from bcm2835-codec
    pub jpeg_data: Vec<u8>,
    /// Live visual event annotations detected in this specific frame
    pub detected_events: Vec<VisualEventMarker>,
    /// Current 4-point homography calibration quad in camera pixel space
    pub calibration_quad: Option<[Vec2; 4]>,
}
```

### 5.3. Terminal UI Review Panel Features
In `terminal` / `terminal-ui`:
1. **Interactive Image Viewport**: Displays the live video feed.
2. **Overlay Toggles**:
   - `[X] Projector Calibration Quad`: Draws the polygon bounding the laser's active projection area.
   - `[X] Visual Event Crosshairs`: Renders real-time crosshairs over detected player laser hits with hit timestamp and confidence.
   - `[X] Active Targets`: Renders the game target bounding boxes directly over the camera image so the operator can verify target alignment.
3. **Threshold Tuning Controls**: Sliders to adjust exposure time, sensor gain, and hit sensitivity live without restarting the server.

---

## 6. Phased Implementation Roadmap

### Phase 1: Prototype Capture & Streaming Harness
- [ ] Create `server-core/src/plugins/camera.rs` with `CameraPlugin`.
- [ ] Implement capture worker thread using `rpicam-vid` pipe (YUV/RGB) on the Raspberry Pi.
- [ ] Implement preview frame encoder piping into `bcm2835-codec` (MJPEG).
- [ ] Add `NetworkMessage::CameraStreamFrame` to `common/src/network.rs`.

### Phase 2: Terminal UI Visual Review Panel
- [ ] Implement camera review widget in `terminal-ui` using `egui::Image` / Bevy texture.
- [ ] Stream video frames over existing client-server QUIC/UDP transport.
- [ ] Add overlay toggles for calibration quad and detection markers.

### Phase 3: Algorithmic Laser Dot Detection
- [ ] Implement manual shutter/exposure locking in server config (`config/server.yaml`).
- [ ] Implement wavelength subtraction algorithm ($G - \max(R, B)$) and centroid moment calculation.
- [ ] Emit `LaserHitEvent` in Bevy ECS and verify latency is < 15 ms.

### Phase 4: Automated Homography Calibration
- [ ] Build 4-point auto-calibration routine:
  - Laser projects 4 corners.
  - Camera detects centroids.
  - Compute homography matrix $H$.
  - Store calibration in `config/camera_calibration.yaml`.

### Phase 5: Gameplay Integration
- [ ] Connect `LaserHitEvent` to Hunter minigame (shooting balloons/targets).
- [ ] Connect `LaserHitEvent` to Snake minigame (controlling snake / collecting food).
- [ ] Trigger reactive explosion / burst fills via the transformer pipeline on verified hit.
