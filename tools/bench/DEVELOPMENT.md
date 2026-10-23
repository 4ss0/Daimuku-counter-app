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
    cargo run --release --example session -- s.wav take-0.wav:3 ...   # one session with a profile, every event (TRUTH=n NEG=1 BASEONLY=1)
    cargo run --release --example loo -- take-0.wav:3 take-1.wav:5 ...  # each take by the profile trained without it, + negatives
    cargo run --release --example frames -- s.wav 0 10                 # front end every 0.1 s (energy, activity, voicing)
    cargo run --release --example latency -- 5 480                      # time of one Engine::push

A voice exported from the app (.json) holds the takes as base64 WAV with
their `n_daimoku`: decode `wav_base64` of each take to a file for these
tools.

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

- v0.6.0: four fixes in the checkers, found on a phone session of 56
  Daimoku (3 + 3 slow under a bell + 44 fast with beads rubbed twice + 3 slow
  under a bell + 3; recorded 2026-10-23, sent as WhatsApp AAC, not in the
  bench) that the app counted 49:
  1. N stopped its walk at the first Daimoku already counted (by B, not
     confirmed by a checker) instead of walking past it: the 4 Daimoku under
     the beads right after it were never reached.
  2. `phrases_on_path` trims the ends of a phrase of frames whose low-band
     level is `EDGE_QUIET_NATS` below its voice, not only of inactive ones:
     room noise (~26 dB under the voice) kept the activity flag on, and the
     decoder stretched "Nam" or "kyo" over seconds of silence, so the
     phrases around a pause had absurd lengths for F and S.
  3. F links two phrases whose lengths up to "kyo" match (`WPhrase::body`)
     even when the whole lengths do not (the last "kyo" of a group is held),
     and accepts a run of three with two strong phrases (before: two strong
     only, or four sane): the three Daimoku that open and close a practice.
  4. N's "complete phrase on the decoding" test, for the last phrase of a
     walk, ignores decoded phrases already counted: the held "kyo" after the
     last Daimoku of a group was counted a second time.

  Measured (`examples/session`, `examples/loo`; the full bench was not run,
  its audio is not in git: run `bench new A P3` and `FULL=1` before release):

  | | v0.5.3 | v0.6.0 |
  |---|---|---|
  | session above, user profile (13 takes) | 45/56 | 52/56 |
  | same, base model only | 48/56 | 53/56 |
  | the 13 takes, each by the profile trained without it | 12/13 exact, \|err\| 1 | 13/13, \|err\| 0 |
  | negatives of the takes (reversed, 100/200 ms shuffled) | 4 | 0 |
  | negatives of the session (reversed, 100/200/500 ms) | 0/0/0/2 | 0/0/0/2 |
  | built-in clips looped 5 min (`examples/latency`) | 161/162 | 161/162 |

  Still missed: the first group of three (1/3 at best). Its first Daimoku
  scores badly (llr 0.3): the session starts with room noise taken for voice
  (the noise estimate starts at `ABS_ENERGY_FLOOR`, or lower when the first
  frames are digital silence, and rises 0.2 nats/s), which also feeds the
  cepstral mean. Measured and not adopted: a faster rise in the first second
  (session 54/56 but take-8 10 -> 9) and starting the estimate after digital
  silence only (session 51/56): the start changes the cepstral mean for the
  whole session and moves borderline phrases both ways. Open: rescoring the
  first seconds once the mean has settled.

- **v0.6.1, a slow Daimoku with breaths inside.** A 26 s session (3 fast,
  then 3 slow of ~5 s; new profile of 4 takes, one slow) counted 5 of 6.
  The middle slow one had two breaths inside ("Nam-myoho-ren ... ge ...
  kyo") and a clipped "kyo" of 0.25 s. B refused it (middling evidence,
  and not in a run: the slow one before it was still waiting for S, which
  confirms slow ones only in pairs); S's decoding put "kyo" on the breath
  and called the real "kyo" garbage, so the phrase had no "kyo"; N found
  it (envelope 0.68) but its window was at least 0.85 x the period, the
  period being the longer of the two slow anchors (B's estimated end of a
  slow one runs long), so the window took in the next pause and only 73%
  of it was voice (< 0.8). Two changes in N: the period is the lower
  median of the anchors, and a slow step needs 70% voice instead of 80%.

  | | before | after |
  |---|---|---|
  | this session, its profile / the 13-take profile | 5/6, 5/6 | 6/6, 6/6 |
  | session of 56, 13-take profile / new 4-take profile | 52, 53 | 52, 54 |
  | the 13 takes, leave one out | 13/13 | 13/13 |
  | the 4 new takes, leave one out | 3/4 (slow 2/3) | 3/4 |
  | negatives (takes, both sessions) | 0 (500 ms: 2) | same |

- **v0.6.1, the app's bell taken out of the microphone.** The bells of the
  Count screen are synthetic, so their partials are known exactly
  (tools/bells/make.py). On every strike the front end tells the engine
  (`bell_struck`) and `src-tauri/src/bell.rs` puts a notch (RBJ, Q 20) on
  both sines of every partial (they come in pairs a few Hz apart, the
  beat; one notch on the centre left them at only ~-22 dB) for 6.5 s,
  fading in and out over 20 ms. `examples/bellmix` mixes the real bell
  (static/bells, struck every 10 s / 6 s, peak 0.5x or 1.5x the voice's)
  into a session and counts it clean, with the bell, and with the bell
  and the filter:

  | session | bell | clean | with bell (0.5x / 1.5x) | bell + filter | filter, no bell |
  |---|---|---|---|---|---|
  | 56 (13-take profile) | high | 52 | 36 / 37 | 52 / 52 | 52 |
  | | medium | 52 | 28 / 21 | 51 / 51 | 51 |
  | | deep | 52 | 51 / 42 | 53 / 53 | 53 |
  | 6 of the 15:41 session | high | 6 | 0 / 0 | 6 / 6 | 6 |
  | | medium | 6 | 0 / 0 | 6 / 6 | 6 |
  | | deep | 6 | 6 / 7 | 6 / 6 | 6 |

  With the filter the bell changes nothing: the counts are those of the
  filter alone, which moves the session of 56 by one either way.

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
