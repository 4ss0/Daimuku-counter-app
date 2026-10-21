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

## Change log after v0.5.0

- v0.5.2: `NOISE_MARGIN_NATS` 1.2 -> 0.2 (activity gate). Clean bench
  unchanged; large gain in noise (white 30 dB: wa0929 75 -> 145 of 148).
- After v0.5.2: the decoding graph (`hmm.rs`, `decode_loop_min`) lets a
  phrase stop after "ge" and fall silent (`GE_STOP_PEN` = -30). Before, the
  decoder had to stretch "kyo" over the end of "ge" and counted phrases broken
  off before "kyo". Bench new A: TP 59/59, |err| 3, strict FP 0 -> 1
  (live51.sh200.1, a 200 ms shuffle); new P3: unchanged (strict FP 3).
  neg/cut_kyo_1017 at 48 kHz: female-take profile 17 -> 0, group-take
  profile 17 -> 0, phone profile of 2026-10-17 6 -> 2. Long female/group
  recordings (60 Daimoku each) unchanged. Not solved: profiles trained on
  takes the model cannot learn (e.g. 0.7 s/Daimoku) still count truncated
  phrases through the F/N checkers.
- After v0.5.2: the user's recordings share a fixed training weight
  (`train::user_take_weight`: 2.5 in total, at least 0.5 each; before, 2.5
  each). With 2.5 each, a phone profile of five takes (2026-10-21) learned a
  "kyo" loose enough to be found in phrases stopped at "renge": the end of
  "ge" plus the start of the next "Nam" passed for a 6-frame "kyo" (the
  decoder's minimum). Bench new A unchanged; new P3 (5 takes, now 0.5
  each): TP 59/59, |err| 3, strict FP 3 -> 3 (other files), t1212 27 -> 33.
  Phone profiles, measured in a review harness (personal recordings, not
  in the bench):

  | | before | after |
  |---|---|---|
  | profile 2026-10-21 pm (5 takes): truncated recording of 10-17 (48 kHz) | 15 | 0 |
  | same: truncated recording of 10-21 (11 phrases stopped at "renge") | 11 | 0 |
  | same: neg/cut_kyo_1017 | 15 | 0 |
  | same: count error, 5 bench files / noisy1 | 35 / 0 of 17 | 12 / 8 of 17 |
  | same: each take judged by the profile trained without it | 30/31 | 30/31 |
  | profile 2026-10-21 am (8 takes): the two truncated recordings | 19 + 11 | 1 + 0 |
  | same: strict FP | 64 | 12 |
  | same: each take judged by the profile trained without it | 39/44 | 32/44 |

  Measured and not adopted: a "kyo" verifier in front of every controller
  (two Gaussian mixtures, "kyo" against what a phrase without "kyo" puts in
  its place, trained on copies of the takes with "kyo" cut out): no gain on
  the truncated recordings, noisy1 16 -> 14 on good profiles; trimming the
  first "Nam" and last "kyo" of each take in training (noisy1 16 -> 8 on
  P3); at least 8 frames of "kyo" before a "Nam" (2026-10-21 pm: TP 59 ->
  10). Open: copies of the user's own takes with "kyo" cut out are still
  counted about half the time by the profile trained without that take, and
  a profile of many takes recognises fewer of its slow takes (39 -> 32 of
  44 above).

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
