# Camera Stream & Vision Tracking Plan

## 1. Objective & Hardware Baseline

Integrate physical camera capture into the `lasertargets` server on the Raspberry Pi 4 to enable:
1. **Real-time laser hit detection**: Detecting laser beam dots or sweeps on physical targets with low latency (< 15 ms).
2. **Projector-to-camera homography calibration**: Aligning the laser galvo coordinate space ($[-1.0, 1.0]$) with the camera field-of-view.
3. **Live diagnostic streaming**: Streaming video frames to the remote terminal UI / operator monitor with minimal Pi CPU overhead.

### Verified Hardware Profile (Sony IMX219 / Pi Camera Module v2)

From the hardware probe (`cargo xtask camera --detailed`), the Raspberry Pi 4 has an active **Sony IMX219 (Raspberry Pi Camera Module v2)** connected via the native MIPI-CSI2 ribbon interface (`/base/soc/i2c0mux/i2c@1/imx219@10` via Unicam):

| Sensor Mode | Resolution | Max FPS | Aspect Ratio | FOV Mode | Role in LaserTargets |
|---|---|---|---|---|---|
| **Mode 1** | **`640x480`** | **200.16 FPS** | 4:3 | 2x2 crop | **High-speed laser dot tracking** (5 ms frame time) |
| **Mode 2** | **`1640x1232`** | **81.07 FPS** | 4:3 | Full FOV (2x2 bin) | **Target alignment & 4-point homography calibration** |
| **Mode 3** | `1920x1080` | 47.57 FPS | 16:9 | 1080p crop | High-definition live preview stream |
| **Mode 4** | `3280x2464` | 21.19 FPS | 4:3 | Full 8 MP native | Still frame reference & background capture |

### On-Chip Hardware Acceleration

The Raspberry Pi 4 (BCM2711 / VideoCore VI) includes dedicated hardware silicon blocks exposed in the kernel:
- **Hardware ISP (`bcm2835-isp`, `/dev/video13–16`, `/dev/video20–23`)**: Performs Bayer demosaicing, black-level correction, lens shading, and color matrix conversion in silicon directly into RAM without loading the CPU.
- **Hardware Codec (`bcm2835-codec`, `/dev/video11`, `/dev/video31`)**: Hardware **H.264** and **MJPEG** video encoders. Can encode 1080p / 720p streams for remote UI preview with near 0% CPU consumption.

---

## 2. Server Architecture (Bevy ECS Integration)

The `lasertargets` server must maintain a rock-solid **60 FPS** update rate for the Helios Laser DAC galvo output. The camera pipeline must never block Bevy's main schedule or the DAC thread.

```
┌────────────────────────────────────────────────────────┐
│               Hardware Silicon (Pi 4)                  │
│   Sony IMX219 (CSI-2) ──► Unicam ──► bcm2835-ISP       │
└────────────────────────────────────┬───────────────────┘
                                     │ DMA-BUF / YUV420
┌────────────────────────────────────▼───────────────────┐
│              Dedicated Capture Thread                  │
│   - libcamera / GStreamer / rpicam-vid pipeline        │
│   - Fixed shutter exposure (5000 µs), fixed gain       │
│   - Non-blocking Crossbeam channel                     │
└──────────────────┬─────────────────────────────────────┘
                   │ Receiver<CameraFrame>
┌──────────────────▼─────────────────────────────────────┐
│          Bevy ECS Server (`server-core`)               │
│                                                        │
│   ┌────────────────────────────────────────────────┐   │
│   │ Resource: CameraStreamReceiver                 │   │
│   └────────────────────────┬───────────────────────┘   │
│                            │                           │
│   ┌────────────────────────▼───────────────────────┐   │
│   │ System: receive_camera_frames                  │   │
│   │  - Extracts latest frame buffer                │   │
│   │  - Drops stale queued frames (always current)  │   │
│   └────────────────────────┬───────────────────────┘   │
│                            │                           │
│   ┌────────────────────────▼───────────────────────┐   │
│   │ System: detect_laser_hits                      │   │
│   │  - Color thresholding (Red / Green / Blue)     │   │
│   │  - Sub-pixel centroid extraction               │   │
│   │  - Homography warp: Camera (px) ──► Galvo [-1,1│   │
│   │  - Emits: LaserHitEvent { pos: Vec2, beam: ... }│   │
│   └────────────────────────┬───────────────────────┘   │
│                            │                           │
│   ┌────────────────────────▼───────────────────────┐   │
│   │ Game Systems (Hunter / Snake)                  │   │
│   │  - Collision check against target bounding box │   │
│   │  - Score updates, pop animations, sound trigger│   │
│   └────────────────────────────────────────────────┘   │
└────────────────────────────────────────────────────────┘
```

---

## 3. Computer Vision & Laser Dot Extraction

### 3.1. Optical Exposure Control (Darkened Frame Strategy)
Standard auto-exposure (AE) will cause the entire frame to wash out when a high-power laser beam strikes a surface, blooming into a large blurry spot.
- **Fixed Shutter Speed**: Set manual shutter to `2,000 µs – 6,000 µs` (1/500s – 1/160s).
- **Low Analogue Gain**: Set analogue gain to `1.0` and turn off auto-white-balance (AWB) or lock temperature to `4500K`.
- **Result**: The background scene appears underexposed/dark, while the laser spot appears as an ultra-high-contrast peak (pixel values near 255).

### 3.2. Color Thresholding & Sub-Pixel Centroid
For RGB/YUV input:
$$\text{Brightness} = 0.299R + 0.587G + 0.114B$$
For red/green laser separation:
$$\text{LaserScore}_{\text{Green}} = G - \max(R, B)$$
$$\text{LaserScore}_{\text{Red}} = R - \max(G, B)$$

Once candidate pixels exceed threshold $T$, compute the center of mass (image moments):
$$C_x = \frac{\sum x \cdot I(x,y)}{\sum I(x,y)}, \quad C_y = \frac{\sum y \cdot I(x,y)}{\sum I(x,y)}$$

### 3.3. Homography Transform (Perspective Mapping)
The camera and the laser projector sit at different angles. A $3 \times 3$ planar homography matrix $H$ maps camera pixel coordinates $(u, v, 1)^T$ directly to normalized laser galvo space $(x_g, y_g, 1)^T$:

$$\begin{bmatrix} x_w \\ y_w \\ w \end{bmatrix} = H \begin{bmatrix} u \\ v \\ 1 \end{bmatrix}, \quad x_g = \frac{x_w}{w}, \quad y_g = \frac{y_w}{w}$$

- **Calibration Routine**: Projector displays 4 calibration crosshairs at known galvo coordinates:
  - Top-Left: $(-0.8, 0.8)$
  - Top-Right: $(0.8, 0.8)$
  - Bottom-Right: $(0.8, -0.8)$
  - Bottom-Left: $(-0.8, -0.8)$
- Camera detects the 4 centroid coordinates and solves for $H$ using Direct Linear Transformation (DLT).

---

## 4. Implementation Options Analysis

| Approach | Implementation | Pros | Cons | Recommendation |
|---|---|---|---|---|
| **Option A: GStreamer (`gstreamer-rs`)** | `libcamerasrc ! video/x-raw,format=RGB ! appsink` | Zero-copy DMA buffers, hardware ISP format conversion, clean Rust bindings. | Requires `libgstreamer1.0-dev` on build image. | **Recommended for production** |
| **Option B: Subprocess Pipe (`rpicam-vid`)** | Spawn `rpicam-vid --codec yuv420 -t 0 -o -` and read `stdout` | Zero build dependencies, isolated process crash protection, easily testable. | Slight IPC overhead; manual stream header parsing. | **Recommended for prototype** |
| **Option C: V4L2 compat (`v4l` crate)** | Connect directly to `/dev/video0` via standard V4L2 ioctl | Standard Linux API. | On modern Pi OS, libcamera bypasses standard V4L2 for raw Bayer streams unless using complex media-controller pipelines. | Not recommended for CSI |

---

## 5. Phased Implementation Roadmap

### Phase 1: Prototype Capture Harness
- [ ] Create `server-core/src/plugins/camera.rs` with `CameraPlugin`.
- [ ] Implement capture worker thread using `Option B` (`rpicam-vid` pipe) for rapid verification.
- [ ] Expose `CameraFrame` resource in Bevy world.

### Phase 2: Exposure Control & Dot Detection
- [ ] Add config options in `config/server.yaml`: `camera.shutter_us`, `camera.gain`, `camera.threshold`.
- [ ] Implement fast threshold and centroid extraction system (< 2 ms per frame).
- [ ] Fire `LaserHitEvent` when spot detected.

### Phase 3: Projector-Camera Calibration
- [ ] Implement automated 4-point calibration state:
  - Project 4 reference points sequentially with laser.
  - Detect camera centroid for each point.
  - Calculate homography matrix $H$ and persist to `config/camera_calibration.yaml`.

### Phase 4: Gameplay Integration
- [ ] Wire `LaserHitEvent` into Hunter minigame (target shooting).
- [ ] Wire `LaserHitEvent` into Snake minigame (fruit eating / steering).
- [ ] Add hit feedback ripple / burst effect in the transformer pipeline.

### Phase 5: GStreamer Migration & Network Streaming
- [ ] Migrate capture worker to `gstreamer-rs` with `libcamerasrc`.
- [ ] Tee stream into `/dev/video11` (hardware H.264/MJPEG encoder) to broadcast live low-latency preview stream to Terminal UI.

