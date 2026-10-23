// The bell: three struck bells (static/bells, made by tools/bells/make.py),
// played with Web Audio so that a tap sounds at once. Each tap is a new
// strike: the previous one keeps ringing under it, as with a real bell.
export const BELLS = ['high', 'medium', 'deep'] as const;
export type BellKind = (typeof BELLS)[number];

let ctx: AudioContext | null = null;
const buffers = new Map<BellKind, Promise<AudioBuffer>>();

function audio(): AudioContext {
  if (!ctx) ctx = new AudioContext();
  return ctx;
}

function load(kind: BellKind): Promise<AudioBuffer> {
  let p = buffers.get(kind);
  if (!p) {
    p = fetch(`/bells/${kind}.mp3`)
      .then((r) => r.arrayBuffer())
      .then((b) => audio().decodeAudioData(b));
    p.catch(() => buffers.delete(kind));
    buffers.set(kind, p);
  }
  return p;
}

/** Loads a bell ahead of time, so the first strike is not late. */
export function prepareBell(kind: BellKind) {
  load(kind).catch(() => {});
}

/** Strikes the bell at `volume` (0..1). */
export async function strike(kind: BellKind, volume: number) {
  const ac = audio();
  if (ac.state === 'suspended') await ac.resume();
  const buf = await load(kind);
  const src = ac.createBufferSource();
  src.buffer = buf;
  const gain = ac.createGain();
  gain.gain.value = Math.max(0, Math.min(1, volume));
  src.connect(gain).connect(ac.destination);
  src.start();
}
