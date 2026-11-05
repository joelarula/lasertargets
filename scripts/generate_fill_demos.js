// scripts/generate_fill_demos.js
// Generates 5 Shape v2 example assets demonstrating each laser scene-filling strategy.
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const rootDir = path.resolve(__dirname, '..');

const outDirs = [
  path.join(rootDir, 'assets', 'shapes', 'templates', 'v2'),
  path.join(rootDir, 'assets', 'shapes', 'fill_demos')
];

for (const dir of outDirs) {
  if (!fs.existsSync(dir)) {
    fs.mkdirSync(dir, { recursive: true });
  }
}

function round4(n) {
  return Math.round(n * 10000) / 10000;
}

// -----------------------------------------------------------------------------
// 1. Lissajous Resonant Harmonic Pendulum
// -----------------------------------------------------------------------------
function generateLissajous() {
  const fx = 7, fy = 8;
  const phase = Math.PI / 4;
  const steps = 360;
  const Ax = 0.4, Ay = 0.4;
  const points = [];

  for (let i = 0; i < steps; i++) {
    const t = (i / steps) * 2 * Math.PI;
    const x = round4(Ax * Math.sin(fx * t + phase));
    const y = round4(Ay * Math.sin(fy * t));
    points.push({ x, y });
  }

  return {
    name: "fill_lissajous_pendulum",
    description: "Method A: Resonant Lissajous Harmonic Pendulum fill (7:8 ratio, C-infinity continuous motion, zero corner jerk)",
    tags: ["fill:harmonic", "fill:pendulum", "pattern:lissajous", "version:2"],
    author: "LaserTargets",
    version: 2,
    strokes: [
      {
        closed: true,
        color: [0, 255, 200],
        points
      }
    ]
  };
}

// -----------------------------------------------------------------------------
// 2. Interlaced CRT Raster with Overscan Turnaround Loops
// -----------------------------------------------------------------------------
function generateInterlacedRaster() {
  const halfW = 0.3; // visible -0.3 to +0.3
  const overscanX = 0.38; // turnaround loop extends 8cm into blanked margin
  const loopRadius = 0.02; // arc radius
  const totalLines = 16;
  const points = [];

  // 2 fields: Field 1 = Even lines (0, 2, 4, ...), Field 2 = Odd lines (1, 3, 5, ...)
  const field1 = [];
  const field2 = [];
  for (let i = 0; i < totalLines; i++) {
    if (i % 2 === 0) field1.push(i);
    else field2.push(i);
  }

  const lineOrder = [...field1, ...field2];
  let movingRight = true;

  for (let idx = 0; idx < lineOrder.length; idx++) {
    const lineIndex = lineOrder[idx];
    const y = round4(0.3 - (lineIndex / (totalLines - 1)) * 0.6);

    const xStart = movingRight ? -halfW : halfW;
    const xEnd = movingRight ? halfW : -halfW;

    // Visible line segment
    points.push({ x: round4(xStart), y: round4(y), color: [255, 180, 50] });
    points.push({ x: round4(xEnd), y: round4(y), color: [255, 180, 50] });

    // If not the very last line, add blanked overscan turnaround arc
    if (idx < lineOrder.length - 1) {
      const nextLineIndex = lineOrder[idx + 1];
      const nextY = round4(0.3 - (nextLineIndex / (totalLines - 1)) * 0.6);
      
      const turnDir = movingRight ? 1 : -1;
      const apexX = round4(xEnd + turnDir * 0.07);
      const midY = round4((y + nextY) / 2);

      // Overscan turnaround control points with laser blanked
      points.push({ x: apexX, y: round4(y + (midY - y) * 0.3), color: [0, 0, 0] });
      points.push({ x: apexX, y: round4(nextY - (nextY - midY) * 0.3), color: [0, 0, 0] });
      points.push({ x: round4(movingRight ? halfW : -halfW), y: round4(nextY), color: [0, 0, 0] });

      movingRight = !movingRight;
    }
  }

  return {
    name: "fill_interlaced_crt_raster",
    description: "Method B: 2-Field Interlaced CRT Raster with Blanked Overscan Turnaround Loops (eliminates galvo edge braking)",
    tags: ["fill:raster", "fill:interlaced", "pattern:crt", "version:2"],
    author: "LaserTargets",
    version: 2,
    strokes: [
      {
        closed: false,
        color: [255, 180, 50],
        points
      }
    ]
  };
}

// -----------------------------------------------------------------------------
// 3. Continuous Double Archimedean Spiral
// -----------------------------------------------------------------------------
function generateSpiral() {
  const maxR = 0.38;
  const turns = 8;
  const stepsPerTurn = 48;
  const totalSteps = turns * stepsPerTurn;
  const points = [];

  // Outward spiral
  for (let i = 0; i <= totalSteps; i++) {
    const frac = i / totalSteps;
    const r = round4(0.015 + frac * (maxR - 0.015));
    const theta = frac * turns * 2 * Math.PI;
    const x = round4(r * Math.cos(theta));
    const y = round4(r * Math.sin(theta));
    points.push({ x, y });
  }

  // Interleaved inward spiral (shifted by pi)
  for (let i = totalSteps; i >= 0; i--) {
    const frac = i / totalSteps;
    const r = round4(0.015 + frac * (maxR - 0.015));
    const theta = frac * turns * 2 * Math.PI + Math.PI;
    const x = round4(r * Math.cos(theta));
    const y = round4(r * Math.sin(theta));
    points.push({ x, y });
  }

  return {
    name: "fill_continuous_spiral",
    description: "Method C: Continuous Double Archimedean Spiral (interleaved in/out sweep, smooth rotational inertia)",
    tags: ["fill:spiral", "fill:polar", "pattern:archimedean", "version:2"],
    author: "LaserTargets",
    version: 2,
    strokes: [
      {
        closed: true,
        color: [50, 220, 255],
        points
      }
    ]
  };
}

// -----------------------------------------------------------------------------
// 4. Filleted Hilbert Space-Filling Curve
// -----------------------------------------------------------------------------
function generateHilbert() {
  // Generate Order 3 Hilbert curve (8x8 = 64 cells)
  function hilbert(i, order) {
    const positions = [
      { x: 0, y: 0 },
      { x: 0, y: 1 },
      { x: 1, y: 1 },
      { x: 1, y: 0 }
    ];
    let pt = { ...positions[i & 3] };
    for (let j = 1; j < order; j++) {
      i >>>= 2;
      const quad = i & 3;
      const len = 1 << j;
      if (quad === 0) {
        const temp = pt.x; pt.x = pt.y; pt.y = temp;
      } else if (quad === 1) {
        pt.y += len;
      } else if (quad === 2) {
        pt.x += len; pt.y += len;
      } else if (quad === 3) {
        const temp = len - 1 - pt.y;
        pt.y = len - 1 - pt.x;
        pt.x = temp + len;
      }
    }
    return pt;
  }

  const order = 3;
  const N = 1 << order; // 8
  const totalPts = N * N; // 64
  const scale = 0.7 / (N - 1); // 0.7m width
  const offset = 0.35;
  const points = [];

  for (let i = 0; i < totalPts; i++) {
    const raw = hilbert(i, order);
    const x = round4(raw.x * scale - offset);
    const y = round4(raw.y * scale - offset);
    // Explicit corner fillet radius of 1.5 cm so stage 2/3 rounds it for max speed
    points.push({
      x,
      y,
      corner_radius_m: 0.018
    });
  }

  return {
    name: "fill_filleted_hilbert",
    description: "Method D: Filleted Hilbert Space-Filling Curve (Order 3 with 18mm corner fillets for non-stop galvo speed)",
    tags: ["fill:hilbert", "fill:fractal", "pattern:space_filling", "version:2"],
    author: "LaserTargets",
    version: 2,
    strokes: [
      {
        closed: false,
        color: [255, 80, 180],
        points
      }
    ]
  };
}

// -----------------------------------------------------------------------------
// 5. Continuous TSP Stipple Matrix
// -----------------------------------------------------------------------------
function generateTspStipple() {
  // Concentric target ring dot pattern
  const dots = [];
  // Center
  dots.push({ x: 0, y: 0 });
  // Inner ring (8 dots)
  for (let i = 0; i < 8; i++) {
    const a = (i / 8) * 2 * Math.PI;
    dots.push({ x: round4(0.12 * Math.cos(a)), y: round4(0.12 * Math.sin(a)) });
  }
  // Middle ring (16 dots)
  for (let i = 0; i < 16; i++) {
    const a = (i / 16) * 2 * Math.PI + 0.1;
    dots.push({ x: round4(0.24 * Math.cos(a)), y: round4(0.24 * Math.sin(a)) });
  }
  // Outer ring (24 dots)
  for (let i = 0; i < 24; i++) {
    const a = (i / 24) * 2 * Math.PI;
    dots.push({ x: round4(0.36 * Math.cos(a)), y: round4(0.36 * Math.sin(a)) });
  }

  // Greedy Nearest-Neighbor TSP sort for smooth tour
  const unvisited = [...dots];
  const tour = [unvisited.shift()];
  while (unvisited.length > 0) {
    const curr = tour[tour.length - 1];
    let bestIdx = 0;
    let bestDist = Infinity;
    for (let i = 0; i < unvisited.length; i++) {
      const d = Math.hypot(unvisited[i].x - curr.x, unvisited[i].y - curr.y);
      if (d < bestDist) {
        bestDist = d;
        bestIdx = i;
      }
    }
    tour.push(unvisited.splice(bestIdx, 1)[0]);
  }

  // Construct stroke: each dot has dwell_ms: 0.25ms hold for bright burst
  const points = tour.map(pt => ({
    x: pt.x,
    y: pt.y,
    dwell_ms: 0.25,
    color: [255, 255, 100]
  }));

  return {
    name: "fill_tsp_stipple_matrix",
    description: "Method E: Continuous TSP Stipple Dot Matrix (optimized tour with 0.25ms dot hold times and high transit speed)",
    tags: ["fill:stipple", "fill:tsp", "fill:dots", "version:2"],
    author: "LaserTargets",
    version: 2,
    strokes: [
      {
        closed: true,
        color: [255, 255, 100],
        points
      }
    ]
  };
}

// -----------------------------------------------------------------------------
// Write Files
// -----------------------------------------------------------------------------
const demos = [
  generateLissajous(),
  generateInterlacedRaster(),
  generateSpiral(),
  generateHilbert(),
  generateTspStipple()
];

for (const demo of demos) {
  const jsonStr = JSON.stringify(demo, null, 2);
  for (const dir of outDirs) {
    const dest = path.join(dir, `${demo.name}.json`);
    fs.writeFileSync(dest, jsonStr, 'utf-8');
    console.log(`Wrote: ${dest} (${demo.strokes[0].points.length} points)`);
  }
}
console.log("All 5 Fill Demo Shapes successfully generated.");

