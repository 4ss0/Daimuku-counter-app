// Picture cards to share on social networks and WhatsApp (1080 x 1350,
// the 4:5 format that Instagram, WhatsApp and Facebook show uncropped).
import { get } from 'svelte/store';
import { api } from './api';
import { t } from './i18n';
import { hasNative, shareFile } from './native';

export interface Card {
  /** Small line above the number (period, "Goal reached!"...). */
  kicker: string;
  /** The big number, already formatted. */
  big: string;
  /** Under the number (usually "daimoku"). */
  unit: string;
  /** A few short lines of detail. */
  lines: string[];
  /** Optional bar chart (e.g. the days of the week). */
  bars?: { label: string; value: number; current: boolean }[];
  /** Optional progress towards a goal, 0..1 (instead of the bars). */
  progress?: number;
}

const W = 1080;
const H = 1350;
const FONT = "system-ui, -apple-system, 'Segoe UI', Roboto, 'Hiragino Sans', 'Noto Sans JP', sans-serif";

function loadImage(src: string): Promise<HTMLImageElement | null> {
  return new Promise((resolve) => {
    const img = new Image();
    img.onload = () => resolve(img);
    img.onerror = () => resolve(null);
    img.src = src;
  });
}

function accent(): string {
  const v = getComputedStyle(document.documentElement).getPropertyValue('--accent').trim();
  return v || '#5cc9a7';
}

/** Shrinks the font until `text` fits in `max` pixels. */
function fit(ctx: CanvasRenderingContext2D, text: string, weight: number, size: number, max: number): number {
  let s = size;
  ctx.font = `${weight} ${s}px ${FONT}`;
  while (s > 20 && ctx.measureText(text).width > max) {
    s -= 4;
    ctx.font = `${weight} ${s}px ${FONT}`;
  }
  return s;
}

export async function renderCard(c: Card): Promise<HTMLCanvasElement> {
  const cv = document.createElement('canvas');
  cv.width = W;
  cv.height = H;
  const ctx = cv.getContext('2d')!;
  const acc = accent();

  // background: the app's dark photo, darkened towards the bottom
  const bg = await loadImage('/bg-dark.jpg');
  ctx.fillStyle = '#0c0d0f';
  ctx.fillRect(0, 0, W, H);
  if (bg) {
    const k = Math.max(W / bg.width, H / bg.height);
    ctx.drawImage(bg, (W - bg.width * k) / 2, (H - bg.height * k) / 2, bg.width * k, bg.height * k);
  }
  const g = ctx.createLinearGradient(0, 0, 0, H);
  g.addColorStop(0, 'rgba(8,9,11,0.15)');
  g.addColorStop(0.38, 'rgba(8,9,11,0.45)');
  g.addColorStop(0.55, 'rgba(8,9,11,0.8)');
  g.addColorStop(1, 'rgba(8,9,11,0.94)');
  ctx.fillStyle = g;
  ctx.fillRect(0, 0, W, H);

  ctx.textAlign = 'center';
  ctx.textBaseline = 'alphabetic';

  // the photo's lotus fills the top; the text sits below it
  ctx.fillStyle = 'rgba(238,238,236,0.85)';
  fit(ctx, c.kicker, 600, 50, W - 160);
  ctx.fillText(c.kicker, W / 2, 610);

  ctx.fillStyle = '#ffffff';
  ctx.shadowColor = 'rgba(0,0,0,0.55)';
  ctx.shadowBlur = 30;
  fit(ctx, c.big, 800, 220, W - 140);
  ctx.fillText(c.big, W / 2, 815);
  ctx.shadowBlur = 0;
  ctx.fillStyle = acc;
  ctx.font = `600 60px ${FONT}`;
  ctx.fillText(c.unit, W / 2, 895);

  // laid out from the bottom up: detail lines, then progress or bars
  const lines = c.lines.slice(0, c.bars?.length ? 2 : 3);
  const lineGap = 58;
  const linesTop = H - 130 - (lines.length - 1) * lineGap;
  ctx.fillStyle = 'rgba(238,238,236,0.92)';
  lines.forEach((line, i) => {
    fit(ctx, line, 500, 40, W - 160);
    ctx.fillText(line, W / 2, linesTop + i * lineGap);
  });

  if (c.progress !== undefined) {
    const x0 = 180;
    const bw = W - 360;
    const y = linesTop - 90;
    ctx.fillStyle = 'rgba(255,255,255,0.14)';
    roundRect(ctx, x0, y, bw, 22, 11);
    ctx.fill();
    ctx.fillStyle = acc;
    roundRect(ctx, x0, y, Math.max(22, bw * Math.min(1, c.progress)), 22, 11);
    ctx.fill();
  } else if (c.bars && c.bars.length > 0) {
    const max = Math.max(1, ...c.bars.map((b) => b.value));
    const n = c.bars.length;
    const labelY = linesTop - 70;
    const base = labelY - 40;
    const hgt = Math.max(60, Math.min(170, base - 925));
    const x0 = 120;
    const slot = (W - 240) / n;
    const w = Math.max(4, Math.min(60, slot * 0.62));
    c.bars.forEach((b, i) => {
      const h = b.value > 0 ? Math.max(8, (b.value / max) * hgt) : 4;
      const x = x0 + i * slot + (slot - w) / 2;
      ctx.fillStyle = b.current ? acc : 'rgba(255,255,255,0.34)';
      roundRect(ctx, x, base - h, w, h, Math.min(8, w / 2));
      ctx.fill();
      if (b.label) {
        ctx.fillStyle = 'rgba(238,238,236,0.6)';
        ctx.font = `500 ${n > 12 ? 24 : 28}px ${FONT}`;
        ctx.fillText(b.label, x + w / 2, labelY);
      }
    });
  }

  // footer
  ctx.fillStyle = 'rgba(238,238,236,0.55)';
  ctx.font = `500 34px ${FONT}`;
  ctx.fillText('Daimoku Counter', W / 2, H - 45);
  return cv;
}

function roundRect(ctx: CanvasRenderingContext2D, x: number, y: number, w: number, h: number, r: number) {
  ctx.beginPath();
  ctx.moveTo(x + r, y);
  ctx.arcTo(x + w, y, x + w, y + h, r);
  ctx.arcTo(x + w, y + h, x, y + h, r);
  ctx.arcTo(x, y + h, x, y, r);
  ctx.arcTo(x, y, x + w, y, r);
  ctx.closePath();
}

function toBase64(cv: HTMLCanvasElement): Promise<string> {
  return new Promise((resolve, reject) => {
    cv.toBlob((blob) => {
      if (!blob) return reject(new Error('image'));
      const r = new FileReader();
      r.onload = () => resolve(String(r.result).split(',')[1] ?? '');
      r.onerror = () => reject(r.error);
      r.readAsDataURL(blob);
    }, 'image/png');
  });
}

/**
 * Draws the card and shares it as a picture. On desktop the picture is
 * saved and the returned text says where; on Android it returns ''.
 */
export async function shareCard(c: Card, fileStem: string): Promise<string> {
  const cv = await renderCard(c);
  const d = new Date();
  const pad = (n: number) => String(n).padStart(2, '0');
  const name = `${fileStem}-${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}.png`;
  const f = await api.writeBinaryExport(name, await toBase64(cv));
  if (hasNative()) {
    await shareFile(f, 'image/png', get(t)('share.button'));
    return '';
  }
  return get(t)('set.fileSavedIn', { p: f.path });
}
