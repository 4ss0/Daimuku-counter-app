// The bell: three struck bells (static/bells, made by tools/bells/make.py),
// played with Web Audio so that a tap sounds at once. Each tap is a new
// strike: the previous one keeps ringing under it, softer.
// The counter is told of every strike, so that it does not hear the bell
// (src-tauri/src/bell.rs takes its frequencies out of the microphone).
import { invoke } from '@tauri-apps/api/core';
export const BELLS = ['high', 'medium', 'deep'] as const;
export type BellKind = (typeof BELLS)[number];

// Loudness. Each bell peaks just under full scale, and strikes ringing
// together add up: past full scale the phone clips them, a harsh crackle at
// high volume. So the slider tops out a little lower, a new strike quiets
// the strikes still ringing (it covers them anyway), and what still goes
// too high is rounded off softly instead of clipped.
/** Gain at the top of the volume slider. */
const MAX_GAIN = 0.75;
/** What is left of the strikes still ringing when a new one comes. */
const DUCK = 0.35;
/** How quickly they get there (time constant, s)... */
const DUCK_S = 0.004;
/** ...before the new strike starts (s): too short a delay to be heard. */
const DUCK_LEAD = 0.012;
/** Up to this level the sound passes untouched; above it, it is rounded off. */
const KNEE = 0.7;

let ctx: AudioContext | null = null;
let out: AudioNode | null = null;
const buffers = new Map<BellKind, Promise<AudioBuffer>>();
const ringing = new Set<{ gain: GainNode; level: number }>();

function audio(): AudioContext {
  if (!ctx) ctx = new AudioContext();
  return ctx;
}

/** Where the strikes go: a soft limiter in front of the speaker. */
function output(ac: AudioContext): AudioNode {
  if (!out) {
    const shaper = ac.createWaveShaper();
    const n = 4096;
    const curve = new Float32Array(n);
    for (let i = 0; i < n; i++) {
      const x = (i / (n - 1)) * 2 - 1;
      const a = Math.abs(x);
      const y = a <= KNEE ? a : KNEE + (1 - KNEE) * Math.tanh((a - KNEE) / (1 - KNEE));
      curve[i] = Math.sign(x) * y;
    }
    shaper.curve = curve;
    shaper.oversample = '4x';
    shaper.connect(ac.destination);
    out = shaper;
  }
  return out;
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
  invoke('bell_struck', { kind }).catch(() => {});
  const ac = audio();
  if (ac.state === 'suspended') await ac.resume();
  const buf = await load(kind);
  const now = ac.currentTime;
  for (const v of ringing) {
    v.level *= DUCK;
    v.gain.gain.setTargetAtTime(v.level, now, DUCK_S);
  }
  const src = ac.createBufferSource();
  src.buffer = buf;
  const gain = ac.createGain();
  const voice = { gain, level: MAX_GAIN * Math.max(0, Math.min(1, volume)) };
  gain.gain.value = voice.level;
  src.connect(gain).connect(output(ac));
  ringing.add(voice);
  src.onended = () => {
    ringing.delete(voice);
    gain.disconnect();
  };
  src.start(now + DUCK_LEAD);
}
