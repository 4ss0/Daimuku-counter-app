"""Bell sounds for the Bell page: three struck bells, from the highest to
the deepest, made by additive synthesis (no samples).

    python3 tools/bells/make.py       # needs numpy and ffmpeg (libmp3lame)

writes static/bells/{high,medium,deep}.mp3.

Each partial is a pair of slightly detuned sines (the beating of a real
bell, whose modes come in close pairs) with its own exponential decay; the
strike adds a few milliseconds of filtered noise. Every bell lasts 6 s.
The high bell follows a rin measured on a recording: partials at 1876,
3484, 5460 and 7782 Hz, each higher one fading faster.
"""
import os
import subprocess
import tempfile
import wave

import numpy as np

SR = 44_100
OUT = os.path.join(os.path.dirname(__file__), "..", "..", "static", "bells")

# (frequency Hz, relative amplitude, decay time constant s, beat Hz)
# The frequencies and beats are also in src-tauri/src/bell.rs, which takes
# them out of the microphone while a bell rings: change both together.
# About 6 s each: the decays are shorter than a real bell's, and the last
# 1.5 s fade out, so the sound ends while it is already soft.
SECONDS = 6.0
FADE = 1.5
BELLS = {
    "high": dict(
        partials=[(1876, 1.0, 1.5, 1.6), (3484, 0.75, 0.9, 6.0), (5460, 0.3, 0.35, 4.0), (7782, 0.06, 0.2, 3.0)],
        strike=0.10,
        strike_hz=6000,
    ),
    "medium": dict(
        partials=[(523, 1.0, 1.7, 0.9), (1417, 0.55, 1.1, 2.2), (2678, 0.28, 0.6, 3.1), (4236, 0.10, 0.3, 2.0)],
        strike=0.08,
        strike_hz=3500,
    ),
    "deep": dict(
        partials=[
            (146, 1.0, 1.9, 0.6),
            (296, 0.7, 1.5, 0.9),
            (351, 0.45, 1.2, 1.3),
            (438, 0.35, 1.0, 1.1),
            (604, 0.2, 0.7, 1.7),
            (789, 0.1, 0.45, 2.3),
        ],
        strike=0.14,
        strike_hz=900,
    ),
}


def lowpass(x, hz):
    """One-pole low-pass filter."""
    a = np.exp(-2 * np.pi * hz / SR)
    y = np.empty_like(x)
    acc = 0.0
    for i, v in enumerate(x):
        acc = (1 - a) * v + a * acc
        y[i] = acc
    return y


def bell(partials, strike, strike_hz, seconds=SECONDS, seed=1):
    rng = np.random.default_rng(seed)
    t = np.arange(int(seconds * SR)) / SR
    x = np.zeros_like(t)
    for f, amp, tau, beat in partials:
        ph = rng.uniform(0, 2 * np.pi, 2)
        pair = 0.55 * np.sin(2 * np.pi * (f - beat / 2) * t + ph[0]) + 0.45 * np.sin(2 * np.pi * (f + beat / 2) * t + ph[1])
        x += amp * pair * np.exp(-t / tau)
    # attack: 3 ms, so the start does not click
    x *= np.minimum(1.0, t / 0.003)
    # the strike: a short burst of filtered noise
    n = int(0.03 * SR)
    burst = lowpass(rng.standard_normal(n), strike_hz) * np.exp(-np.arange(n) / (0.006 * SR))
    x[:n] += strike * burst / (np.abs(burst).max() + 1e-9)
    # fade out the end (half a cosine), normalise to -1 dBFS
    m = int(FADE * SR)
    x[-m:] *= 0.5 + 0.5 * np.cos(np.linspace(0, np.pi, m))
    return x / np.abs(x).max() * 10 ** (-1 / 20)


def main():
    os.makedirs(OUT, exist_ok=True)
    for name, spec in BELLS.items():
        x = bell(**spec)
        with tempfile.NamedTemporaryFile(suffix=".wav", delete=False) as tmp:
            path = tmp.name
        with wave.open(path, "wb") as w:
            w.setnchannels(1)
            w.setsampwidth(2)
            w.setframerate(SR)
            w.writeframes((x * 32767).astype(np.int16).tobytes())
        out = os.path.join(OUT, f"{name}.mp3")
        subprocess.run(["ffmpeg", "-loglevel", "error", "-y", "-i", path, "-codec:a", "libmp3lame", "-b:a", "96k", out], check=True)
        os.remove(path)
        print(out)


if __name__ == "__main__":
    main()
