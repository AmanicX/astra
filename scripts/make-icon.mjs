// Generates a 1024x1024 PNG icon for Astra: a four-pointed astral spark
// (astroid star) in warm ivory with a vermilion core, on transparent ground.
import zlib from "node:zlib";
import fs from "node:fs";

const SIZE = 1024;
const px = new Uint8Array(SIZE * SIZE * 4); // RGBA

function put(x, y, r, g, b, a) {
  if (x < 0 || y < 0 || x >= SIZE || y >= SIZE) return;
  const i = (y * SIZE + x) * 4;
  const na = a / 255;
  const oa = px[i + 3] / 255;
  const ra = na + oa * (1 - na);
  if (ra <= 0) return;
  px[i] = Math.round((r * na + px[i] * oa * (1 - na)) / ra);
  px[i + 1] = Math.round((g * na + px[i + 1] * oa * (1 - na)) / ra);
  px[i + 2] = Math.round((b * na + px[i + 2] * oa * (1 - na)) / ra);
  px[i + 3] = Math.round(ra * 255);
}

const cx = SIZE / 2;
const cy = SIZE / 2;

// Four-point star via astroid-like field: |dx|^0.5 + |dy|^0.5 <= s^0.5
// Soft edge with smoothstep; slight vertical elongation.
function starCoverage(x, y) {
  const dx = (x - cx) / 1.0;
  const dy = (y - cy) / 1.18;
  const v = Math.pow(Math.abs(dx), 0.5) + Math.pow(Math.abs(dy), 0.5);
  const edge = Math.pow(340, 0.5);
  const t = 1 - Math.min(1, Math.max(0, v / edge));
  return Math.pow(t, 0.55); // soft but with a solid core
}

for (let y = 0; y < SIZE; y++) {
  for (let x = 0; x < SIZE; x++) {
    const cov = starCoverage(x, y);
    if (cov <= 0.003) continue;
    // Radial warmth: ivory body, vermilion glint at the very core
    const d = Math.hypot(x - cx, y - cy) / 340;
    let r, g, b;
    if (d < 0.16) {
      const k = d / 0.16;
      r = 217; g = 72; b = 43; // vermilion #D9482B
      r = Math.round(r + (242 - r) * k);
      g = Math.round(g + (237 - g) * k);
      b = Math.round(b + (227 - b) * k);
    } else {
      r = 242; g = 237; b = 227; // ivory #F2EDE3
      // faint cool edge
      const k = Math.min(1, (d - 0.16) / 0.84);
      r = Math.round(r - 18 * k);
      g = Math.round(g - 14 * k);
      b = Math.round(b - 6 * k);
    }
    put(x, y, r, g, b, Math.round(255 * cov));
  }
}

// Small companion satellite dot (lower-right), a "period" in the galley
for (let y = 0; y < SIZE; y++) {
  for (let x = 0; x < SIZE; x++) {
    const d = Math.hypot(x - (cx + 268), y - (cy + 262));
    if (d < 54) {
      const cov = 1 - Math.pow(Math.min(1, d / 54), 2.2);
      put(x, y, 217, 72, 43, Math.round(235 * cov));
    }
  }
}

// ---- PNG encoding (no deps) ----
function crc32(buf) {
  let c;
  const table = [];
  for (let n = 0; n < 256; n++) {
    c = n;
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    table[n] = c >>> 0;
  }
  let crc = 0xffffffff;
  for (let i = 0; i < buf.length; i++) crc = table[(crc ^ buf[i]) & 0xff] ^ (crc >>> 8);
  return (crc ^ 0xffffffff) >>> 0;
}

function chunk(type, data) {
  const len = Buffer.alloc(4);
  len.writeUInt32BE(data.length);
  const td = Buffer.concat([Buffer.from(type, "ascii"), data]);
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(td));
  return Buffer.concat([len, td, crc]);
}

const raw = Buffer.alloc(SIZE * (SIZE * 4 + 1));
for (let y = 0; y < SIZE; y++) {
  raw[y * (SIZE * 4 + 1)] = 0;
  Buffer.from(px.buffer, y * SIZE * 4, SIZE * 4).copy(raw, y * (SIZE * 4 + 1) + 1);
}

const ihdr = Buffer.alloc(13);
ihdr.writeUInt32BE(SIZE, 0);
ihdr.writeUInt32BE(SIZE, 4);
ihdr[8] = 8; // bit depth
ihdr[9] = 6; // RGBA
const png = Buffer.concat([
  Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
  chunk("IHDR", ihdr),
  chunk("IDAT", zlib.deflateSync(raw, { level: 9 })),
  chunk("IEND", Buffer.alloc(0)),
]);

fs.writeFileSync("scripts/astra-icon-src.png", png);
console.log("wrote scripts/astra-icon-src.png", png.length, "bytes");
