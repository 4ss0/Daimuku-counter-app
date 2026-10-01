# Counter development notes (v0.5.0)

## Architecture (src-tauri/src/engine.rs)

Four independent controllers share a map of the session at 0.1 s
resolution (`TimeMap`). A Daimoku found by one controller and not by the
others is added to the count; one already counted is never added twice
(`same_daimoku`: overlap >= 40% of the shorter phrase).

- **B, base**: streaming Viterbi (fixed lag 0.25 s), counts as soon as
  "kyo" is heard. Strong evidence counts alone (an isolated phrase only if
  perfectly formed); middling evidence only inside a run (any controller's
  Daimoku just before), fitting its rhythm. A phrase refused a few frames
  into "kyo" is judged again when "kyo" ends.
- **F, fast checker**: every 2 s re-decodes the last 60 s with hindsight.
  Accepts runs of >= 4 sane phrases, runs of >= 2 strong phrases, or
  regular runs of >= 5 phrases (duration CV <= 12%) with looser checks.
- **S, slow checker**: same with the slow model (`build_slow_model`: slow
  base clips + slow user takes, garbage trained on all chant); phrases of
  2-9 s in groups.
- **N, noise checker**: walks from confirmed Daimoku through voice nobody
  could read, one period at a time, matching the low-band (100-1000 Hz)
  envelope against the session's verified Daimoku. Weak matches only with
  decoder or noise evidence, never right after a pause; a phrase at the
  forward edge of a run must be complete (real "kyo") on the decoding.

Front end (features.rs) adds per frame: `hf` (high/low band ratio),
`voiced` (pitch strength), `lf` (low-band level).

The UI shows checker additions (`LiveView.recovered`) as coins that fly
into the counter (src/routes/+page.svelte).

Base model: 11 built-in clips (4 original + 7 of the author's clean
recordings, 16 kHz) in src-tauri/src/assets.

## Evaluation (tools/bench)

The bench compiles the app's own sources (see src/lib.rs). Test audio in
`data/` (16 kHz mono, not in git); hand-checked phrase intervals in
`data/truth/intervals.txt`.

    cd tools/bench
    cargo run --release --example bench -- new A P3     # new engine, base / base + profile
    FULL=1 cargo run --release --example bench -- new A  # + loose negatives, talk tests
    cargo run --release --example bench -- old A         # previous engine (v0.4) for comparison
    V=1 ...                                              # per-file events (B/F/S/N) and misses
    cargo run --release --example seg -- c28.wav         # full-hindsight decoding of one file
    cargo run --release --example phrfeat -- c28.wav wa0929.wav:rev > f.csv   # phrase features

Profiles: `A` = base only, `P3` = the author's PC profile (data/pcprof3).
Negatives: reversed audio, chant cut in 100/200 ms pieces and shuffled,
TTS speech (strict); 500 ms shuffles (loose: two halves can form a real
Daimoku, so these are not all errors).

## Results at v0.5.0

| | v0.4 (A) | v0.5 A | v0.5 P3 |
|---|---|---|---|
| hand-checked phrases (59) | 42 | 59, 0 FP | 59, 0 FP |
| count error, 5 other files | 23 | 3 | 4 |
| noisy1 (bell + beads) | 4/17 | 16/17 | 16/17 |
| strict negatives FP | 3 | 0 | 3 |
| loose negatives FP | - | 27 | 84 |
| counted at once by B | 85% | 55% | 69% |

## Known weaknesses / open questions

- Tuned on few recordings, one voice; 59 hand-checked phrases only.
- The base model now contains the author's voice: unknown for others.
- Many thresholds; some rules may be redundant.
- Personal profile (P3) gives more FP on shuffled chant; the phone profile
  that hurt v0.4 was never tested.
- Slow Daimoku under a bell may arrive late (after the next one).
- CPU/latency on Android not measured (PC: ~110x real time).
- t1212 has no ground truth (23-28 counted).
- Ideas not taken: spectral subtraction, median-filtered mel, low fmax
  features (measured, no gain or worse).
