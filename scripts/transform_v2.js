const fs = require('fs');
const path = require('path');

const inputDir = path.resolve(__dirname, '../assets/shapes/unsorted');
const outputDir = path.resolve(__dirname, '../assets/shapes/unsorted_v2');
const combinedFile = path.resolve(__dirname, '../assets/shapes/unsorted_v2.json');
const targetTag = process.argv[2] || 'pattern:calibration';
const sizeM = 1.0;
const scale = sizeM / 2.0;
const V1_REFERENCE_PPS = 30000.0;

if (!fs.existsSync(outputDir)) {
  fs.mkdirSync(outputDir, { recursive: true });
}

function convertV1(template) {
  const points = template.points;
  const isLit = (i) => (points[i].r !== 0 || points[i].g !== 0 || points[i].b !== 0);
  const edgeLit = (i) => (i + 1 < points.length && isLit(i));

  const runs = [];
  let i = 0;
  while (i < points.length) {
    if (edgeLit(i)) {
      const run = [i];
      while (edgeLit(i)) {
        i++;
        run.push(i);
      }
      runs.push(run);
    } else {
      const incomingLit = i > 0 && edgeLit(i - 1);
      if (isLit(i) && !incomingLit) {
        runs.push([i]);
      }
      i++;
    }
  }

  const strokes = runs.map((run) => {
    const firstP = points[run[0]];
    const strokeColor = [firstP.r, firstP.g, firstP.b];
    const strokePoints = [];

    for (const index of run) {
      const p = points[index];
      const dwellMs = (isLit(index) && p.dwell > 0) ? (p.dwell / V1_REFERENCE_PPS * 1000.0) : null;
      const posX = parseFloat((p.x * scale).toFixed(6));
      const posY = parseFloat((p.y * scale).toFixed(6));

      if (strokePoints.length > 0) {
        const last = strokePoints[strokePoints.length - 1];
        if (last.x === posX && last.y === posY) {
          if (dwellMs !== null) {
            last.dwell_ms = last.dwell_ms !== undefined ? Math.max(last.dwell_ms, dwellMs) : dwellMs;
          }
          continue;
        }
      }

      const edgeColor = [p.r, p.g, p.b];
      const colorMismatch = isLit(index) && (edgeColor[0] !== strokeColor[0] || edgeColor[1] !== strokeColor[1] || edgeColor[2] !== strokeColor[2]);
      
      const ptObj = { x: posX, y: posY };
      if (colorMismatch) {
        ptObj.color = edgeColor;
      }
      if (dwellMs !== null) {
        ptObj.dwell_ms = parseFloat(dwellMs.toFixed(6));
      }
      strokePoints.push(ptObj);
    }

    let closed = false;
    if (strokePoints.length > 3) {
      const first = strokePoints[0];
      const last = strokePoints[strokePoints.length - 1];
      if (Math.abs(first.x - last.x) < 1e-4 * Math.max(1e-6, scale) && Math.abs(first.y - last.y) < 1e-4 * Math.max(1e-6, scale)) {
        closed = true;
        const popped = strokePoints.pop();
        if (popped.dwell_ms !== undefined) {
          first.dwell_ms = first.dwell_ms !== undefined ? Math.max(first.dwell_ms, popped.dwell_ms) : popped.dwell_ms;
        }
      }
    }

    const stroke = { points: strokePoints };
    if (closed) stroke.closed = true;
    stroke.color = strokeColor;
    return stroke;
  });

  return {
    name: template.name,
    description: template.description || '',
    tags: template.tags || [],
    author: template.author || 'LaserTargets',
    version: 2,
    strokes: strokes
  };
}

function formatJson(shape) {
  let str = JSON.stringify(shape, null, 2);
  str = str.replace(/\{\s+"x":\s*([^,\n]+),\s+"y":\s*([^,\n]+)(?:,\s+"color":\s*\[([\s\S]*?)\])?(?:,\s+"dwell_ms":\s*([^,\n]+))?\s+\}/g, (m, x, y, col, dw) => {
    let res = '{"x": ' + x.trim() + ', "y": ' + y.trim();
    if (col) {
      const parsedCol = col.split(',').map(s => s.trim()).filter(Boolean).join(', ');
      res += ', "color": [' + parsedCol + ']';
    }
    if (dw) res += ', "dwell_ms": ' + dw.trim();
    res += '}';
    return res;
  });
  return str + '\n';
}

const files = fs.readdirSync(inputDir).filter(f => f.endsWith('.json')).sort();
const converted = [];

console.log('============================================================');
console.log('  Shape Language v2 Transformer');
console.log('============================================================');
console.log('Target Tag:', targetTag);
console.log('Output Dir:', outputDir);
console.log('------------------------------------------------------------');

for (const f of files) {
  const filePath = path.join(inputDir, f);
  const data = JSON.parse(fs.readFileSync(filePath, 'utf8'));
  if (targetTag !== '--all' && (!data.tags || !data.tags.includes(targetTag))) continue;

  const shapeV2 = convertV1(data);
  const totalPts = shapeV2.strokes.reduce((sum, s) => sum + s.points.length, 0);
  const outPath = path.join(outputDir, f);
  fs.writeFileSync(outPath, formatJson(shapeV2));
  converted.push(shapeV2);
  console.log(`  [CONVERT] ${f.padEnd(10)} -> ${shapeV2.name.padEnd(15)} strokes: ${shapeV2.strokes.length}, pts: ${totalPts}`);
}

fs.writeFileSync(combinedFile, JSON.stringify(converted, null, 2) + '\n');
console.log('------------------------------------------------------------');
console.log(`Successfully converted ${converted.length} shapes.`);
console.log(`Collection file: ${combinedFile}`);
console.log('============================================================');
