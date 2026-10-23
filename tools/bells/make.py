"""Bell sounds for the Bell page: three struck bells, from the highest to
the deepest, made by additive synthesis (no samples).

    python3 tools/bells/make.py       # needs numpy and ffmpeg (libmp3lame)

writes static/bells/{high,medium,deep}.mp3.

Each partial is a pair of slightly detuned sines (the beating of a real
bell, whose modes come in close pairs) with its own exponential decay; the
strike adds a few milliseconds of filtered noise. The high bell follows a
rin measured on a recording: partials at 1876, 3484, 5460 and 7782 Hz,
the first fading by about 4 dB/s, the higher ones faster.
"""
import os
import subprocess
import tempfile
import wave

import numpy as np

SR = 44_100
OUT = os.path.join(os.path.dirname(__file__), "..", "..", "static", "bells")

# (frequency Hz, relative amplitude, decay time constant s, beat Hz)
BELLS = {
    "high": dict(
        seconds=8.0,
        partials=[(1876, 1.0, 2.2, 1.6), (3484, 0.75, 1.3, 6.0), (5460, 0.3, 0.42, 4.0), (7782, 0.06, 0.25, 3.0)],
        strike=0.10,
        strike_hz=6000,
    ),
    "medium": dict(
        seconds=11.0,
        partials=[(523, 1.0, 4.6, 0.9), (1417, 0.55, 2.6, 2.2), (2678, 0.28, 1.3, 3.1), (4236, 0.10, 0.6, 2.0)],
        strike=0.08,
        strike_hz=3500,
    ),
    "deep": dict(
        seconds=15.0,
        partials=[
            (146, 1.0, 7.5, 0.6),
            (296, 0.7, 5.5, 0.9),
            (351, 0.45, 4.0, 1.3),
            (438, 0.35, 3.2, 1.1),
            (604, 0.2, 2.0, 1.7),
            (789, 0.1, 1.2, 2.3),
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


def bell(seconds, partials, strike, strike_hz, seed=1):
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
    # fade out the last half second, normalise to -1 dBFS
    m = int(0.5 * SR)
    x[-m:] *= np.linspace(1, 0, m)
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
