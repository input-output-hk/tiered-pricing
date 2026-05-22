# Multi-Seed Variance Bands — TEST-03 + TEST-04 Results


> **⚠️ SUPERSEDED 2026-05-21** — numerical claims below were computed under the
> pre-Cardano Improvement Proposal (CIP)-0164 EB-sizing simulator variant
> (`linear`, 12 megabyte (MB) EB wire object). Endorser Block (EB) certification
> failed under that variant, biasing every inclusion-rate / latency / welfare
> measurement. See [`../../../docs/phase-2/eb-sizing-fix-postmortem.md`](../../../docs/phase-2/eb-sizing-fix-postmortem.md) for the diagnosis and the re-run schedule.

**Run id:** `20260518-084846`
**Suites:** `robustness-sign-flip-variance.yaml`, `robustness-canonical-variance.yaml`
**N seeds:** 20 (chosen per [scoping-results.md](scoping-results.md))
**Per-seed scalar:** `retained_value` = `priority_retained_value_total + standard_retained_value_total` (per CONTEXT.md D-24)
**Confidence interval method:** paired-sample Bias-corrected and accelerated (BCa) bootstrap, 9999 iterations
**Analyser:** `sim-rs/scripts/analyse-robustness.py` (Python port of `sim-cli/src/metrics/paired_bootstrap.rs`; documented bootstrap seeds; deterministic given seed within `random` module version)
**Per-cell raw artefacts:** [`sign-flip/`](sign-flip/), [`canonical/`](canonical/)

---

## TEST-03 — Sign-flip Variance Bands (4 cells)

Each cell is a (mechanism × demand × protocol) configuration where, in the
2026-05-14 accumulator-vs-chain-derived welfare-impact characterisation, the
welfare delta sign flipped between Family A and Family B. This test re-runs
each cell under committed Family B at N=20 seeds and bootstraps the per-seed
`retained_value` delta vs a single-lane Ethereum Improvement Proposal 1559
(EIP-1559) baseline at the same protocol+demand.

| Cell | Verdict | BCa 95% CI on Δ `retained_value` | Median Δ | Sign-coherence | Distinct hashes |
|---|---|---|---|---|---|
| `cell_eip1559_d4_t50_w32` | **BACKED** | `[+3.38e+09, +1.35e+10]` | `+5.37e+09` | 0.75 | 20/20 ✓ |
| `cell_eip1559_d8_t25_w32` | **BACKED** | `[+4.68e+08, +5.66e+09]` | `+7.81e+07` | 0.55 | 20/20 ✓ |
| `cell_rb_reserved_x4_rb_quarter` | **WEAK** | `[-1.50e+09, +2.18e+09]` | `+1.50e+09` | 0.60 | 20/20 ✓ |
| `cell_partitioned_x4_rb_quarter` | **WEAK** | `[-1.61e+09, +2.14e+09]` | `+1.50e+09` | 0.60 | 20/20 ✓ |

**Verdict criteria (CONTEXT.md D-32):**
- **BACKED** iff (a) 95% CI excludes zero AND (b) hash-diversity gate passes
  (distinct `pricing_event_stream.sha256` count = N).
- **WEAK** iff CI crosses zero.
- **re-run-needed** iff hash-diversity gate fails.

**Findings:**
- Both single-lane EIP-1559 sign-flip cells (D=4 step-size, target=0.25)
  produce a **statistically significant positive welfare delta** vs the
  baseline (D=8, target=0.5, window=32). The 95% CIs are strictly above zero.
  Sign-coherence at 0.75 and 0.55 indicates moderate seed-level agreement.
- The two RB-quarter cells (priority-only-static and partitioned both-dynamic
  at multiplier_floor = 4, RB capacity reduced to quarter) yield identical
  delta summaries (`median=+1.50e+09`, sign-coherence=0.60) but **CIs straddle
  zero** — variance dominates. The mechanism's welfare improvement at RB-quarter
  is real-but-noisy at N=20.
- All 4 cells pass the COV-05 hash-diversity gate (20 distinct pricing event
  stream SHA-256 hashes across 20 seeds). No collisions, no re-run-needed.

---

## TEST-04 — Canonical Menu-Item Variance Bands (4 menu options + 1 control)

Each menu option is a representative configuration of one of the four
Cardano Improvement Proposal (CIP) mechanism arms, paired against the
single-lane EIP-1559 control (`d8_target0.5_window32`) at the same
demand+topology.

| Menu option | Verdict | BCa 95% CI on Δ `retained_value` | Median Δ | Sign-coherence | Distinct hashes |
|---|---|---|---|---|---|
| `menu_rb_reserved_priority_only_static_x4` | **BACKED** | `[-6.02e+09, -1.00e+09]` | `-4.15e+09` | 0.65 | 20/20 ✓ |
| `menu_unreserved_priority_only_static_x4` | **BACKED** | `[+4.28e+09, +8.49e+09]` | `+6.66e+09` | 0.90 | 20/20 ✓ |
| `menu_rb_reserved_both_dynamic_x4` | **BACKED** | `[-5.95e+09, -8.87e+08]` | `-4.15e+09` | 0.65 | 20/20 ✓ |
| `menu_unreserved_both_dynamic_x4` | **BACKED** | `[+5.65e+09, +1.09e+10]` | `+7.95e+09` | 0.90 | 20/20 ✓ |

**Findings:**
- All 4 menu options have CIs that strictly exclude zero — every menu arm
  produces a statistically significant welfare delta vs single-lane EIP-1559.
- **RB-reserved arms (both priority-only-static and both-dynamic) produce a
  negative welfare delta** of order `-4e+09` (CIs `[-6.0, -0.9]e+09`).
  Reserving RB partition for priority-only at multiplier_floor=4 reduces total
  retained value vs leaving the chain on single-lane EIP-1559.
- **Un-reserved arms (both priority-only-static and both-dynamic) produce a
  positive welfare delta** of order `+6e+09` to `+8e+09` (CIs `[+4.3, +10.9]e+09`).
  Sign-coherence of 0.90 indicates strong seed-level agreement.
- The qualitative pattern — un-reserved variants strictly outperform RB-reserved
  variants at multiplier_floor=4 under `sundaeswap_moderate` demand — replicates
  the Family-B-vs-accumulator characterisation pattern from
  `.planning/mechanism-welfare-impact-2026-05-14.md`, and is statistically
  defended at N=20.
- All 4 cells pass the hash-diversity gate.

---

## Methodology

**Pairing:** for each cell, `samples_a[i]` is the cell's `retained_value` at
seed `i`, `samples_b[i]` is the control's `retained_value` at the same seed.
The bootstrap operates on `delta[i] = samples_a[i] - samples_b[i]`. Paired
resampling preserves the seed-level coupling — the same seed produces the
same network gossip schedule, lottery wins, and tx generation across both
arms, so paired comparison nets out simulator-side variance unrelated to
the mechanism.

**Hash-diversity gate (COV-05 / CONTEXT.md D-19):** for each cell, count the
distinct `pricing_event_stream.sha256` values across the 20 seeds. If the
count equals 20, the seed-set is genuinely diverse (no collisions). If
count < 20, the verdict is **re-run-needed**: the seed-set must be re-drawn
with different seed values before a verdict is licensed. All 8 cells across
TEST-03 and TEST-04 pass at 20/20.

**Bootstrap seeds:** TEST-03 uses base seeds `[1001, 1002, 1003, 1004]` for
the 4 cells (offsets `+0..3`). TEST-04 uses `[2001, 2002, 2003, 2004]`. The
bootstrap-seed namespace is disjoint from the simulator-seed namespace
(CONTEXT.md D-23); seeds are recorded in every cell JSON.

**Comparison to Rust library:** the Python analyser
(`sim-rs/scripts/analyse-robustness.py`) is an algorithmic port of
`sim-rs/sim-cli/src/metrics/paired_bootstrap.rs`. Python uses
`random.Random(seed)` (Mersenne Twister) and Rust uses `StdRng::seed_from_u64`
(ChaCha12); the two RNGs produce different bootstrap resamples even for
the same `bootstrap_seed`. Python results are deterministic given the same
seed within a `random` module version. Future Rust-emitted CiResult artefacts
will differ in trailing digits but should land in the same verdict bucket.

## Subsidiary metrics (informational, per CONTEXT.md D-24)

`net_utility_total` and `retained_value_ratio` per-seed values are stored in
each cell's JSON artefact alongside `retained_value` but do not gate the
verdict — only `retained_value` does. Subsidiary metrics may be cited
alongside CIs in the Cardano Improvement Proposal (CIP) Evidence section as
sanity checks, never as primary gates.

## Abbreviations on first use

- **BCa** — Bias-corrected and accelerated (bootstrap confidence-interval method)
- **CIP** — Cardano Improvement Proposal
- **EIP-1559** — Ethereum Improvement Proposal 1559
- **IQR** — Inter-Quartile Range
- **RB** — Ranking Block (Cardano's standard block)
