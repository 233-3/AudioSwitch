// 生成托盘四态图标与应用图标。
//
// 设计：统一使用「圆角方形徽标 + 白色图形符号」，四种状态只改变底色与符号，
// 保证在 Windows 浅色/深色任务栏上都有足够对比度，并且属于同一套图标体系。
//
// 用法：node scripts/generate-icons.mjs

import { Resvg } from "@resvg/resvg-js";
import pngToIco from "png-to-ico";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";

const ROOT = path.resolve(import.meta.dirname, "..");
const TRAY_DIR = path.join(ROOT, "src-tauri", "assets", "tray");
const ICON_DIR = path.join(ROOT, "src-tauri", "icons");

const COLORS = {
  normal: "#2563EB",
  switching: "#0891B2",
  success: "#15803D",
  error: "#C2261E",
};

/** 耳机（正常状态 / 应用图标） */
function glyphHeadphone(compact = false) {
  const stroke = compact ? 30 : 26;
  const cupW = compact ? 46 : 44;
  return `
    <path d="M64 144v-12a64 64 0 0 1 128 0v12" stroke="#fff" stroke-width="${stroke}"
          fill="none" stroke-linecap="round"/>
    <rect x="${128 - 86 - (compact ? 1 : 0)}" y="126" width="${cupW}" height="80" rx="${cupW / 2}" fill="#fff"/>
    <rect x="${128 + 86 - cupW + (compact ? 1 : 0)}" y="126" width="${cupW}" height="80" rx="${cupW / 2}" fill="#fff"/>
  `;
}

/** 循环箭头（切换中） */
function glyphSwitching() {
  return `
    <path d="M128 58a70 70 0 1 0 70 70" stroke="#fff" stroke-width="26" fill="none" stroke-linecap="round"/>
    <path d="M186 52l22 40-44-2z" fill="#fff"/>
  `;
}

/** 对勾（成功） */
function glyphSuccess() {
  return `
    <path d="M66 132l40 42 84-92" stroke="#fff" stroke-width="28" fill="none"
          stroke-linecap="round" stroke-linejoin="round"/>
  `;
}

/** 感叹号（失败） */
function glyphError() {
  return `
    <rect x="112" y="56" width="32" height="94" rx="16" fill="#fff"/>
    <circle cx="128" cy="186" r="18" fill="#fff"/>
  `;
}

const GLYPHS = {
  normal: () => glyphHeadphone(false),
  switching: glyphSwitching,
  success: glyphSuccess,
  error: glyphError,
};

function svgFor(state, size = 256) {
  const glyph = state === "normal" ? glyphHeadphone(size <= 16) : GLYPHS[state]();
  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256" width="256" height="256">
  <rect x="0" y="0" width="256" height="256" rx="${size <= 16 ? 48 : 58}" fill="${COLORS[state]}"/>
  ${glyph}
</svg>`;
}

function render(svg, size) {
  const resvg = new Resvg(svg, {
    fitTo: { mode: "width", value: size },
    background: "rgba(0,0,0,0)",
  });
  return resvg.render().asPng();
}

const TRAY_SIZES = [16, 20, 24, 32];
const APP_SIZES = [32, 48, 64, 128, 256, 512];

async function main() {
  await mkdir(TRAY_DIR, { recursive: true });
  await mkdir(ICON_DIR, { recursive: true });

  // 1) 托盘图标：四种状态 × 四个尺寸
  for (const state of Object.keys(GLYPHS)) {
    for (const size of TRAY_SIZES) {
      const buffer = render(svgFor(state, size), size);
      await writeFile(path.join(TRAY_DIR, `${state}-${size}.png`), buffer);
    }
  }

  // 2) 应用图标
  const normalSvg = svgFor("normal");
  const rendered = new Map();
  for (const size of APP_SIZES) {
    const buffer = render(normalSvg, size);
    rendered.set(size, buffer);
    await writeFile(path.join(ICON_DIR, `icon-${size}.png`), buffer);
  }

  await writeFile(path.join(ICON_DIR, "32x32.png"), rendered.get(32));
  await writeFile(path.join(ICON_DIR, "128x128.png"), rendered.get(128));
  await writeFile(path.join(ICON_DIR, "128x128@2x.png"), rendered.get(256));
  await writeFile(path.join(ICON_DIR, "icon.png"), rendered.get(512));

  // 3) ICO（多尺寸，Windows 会按 DPI 选择最合适的图层）
  const icoSizes = [16, 24, 32, 48, 64, 128, 256];
  const icoInputs = icoSizes.map((size) => render(normalSvg, size));
  const ico = await pngToIco(icoInputs);
  await writeFile(path.join(ICON_DIR, "icon.ico"), ico);

  console.log(`托盘图标 -> ${TRAY_DIR}`);
  console.log(`应用图标 -> ${ICON_DIR}`);
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
