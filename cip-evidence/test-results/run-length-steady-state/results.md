# TEST-06 — Run-Length / Steady-State Results


> **⚠️ SUPERSEDED 2026-05-21** — numerical claims below were computed under the
> pre-Cardano Improvement Proposal (CIP)-0164 EB-sizing simulator variant
> (`linear`, 12 megabyte (MB) EB wire object). Endorser Block (EB) certification
> failed under that variant, biasing every inclusion-rate / latency / welfare
> measurement. See [`../../../docs/phase-2/eb-sizing-fix-postmortem.md`](../../../docs/phase-2/eb-sizing-fix-postmortem.md) for the diagnosis and the re-run schedule.

**Status:** PARTIAL (only 1 of 4 menu arms has data)
**Run id:** `20260518-084846`
**Suite:** `robustness-run-length.yaml`

## Coverage

| Job (menu arm × slot length) | Completed / N=10 |
|---|---|
| `rb_reserved_priority_only_x4_slots_2000` | **10/10 ✓** |
| `rb_reserved_priority_only_x4_slots_4000` | **10/10 ✓** |
| `rb_reserved_priority_only_x4_slots_8000` | **8/10** (2 seeds incomplete) |
| `unreserved_priority_only_x4_slots_2000` | 3/10 |
| `unreserved_priority_only_x4_slots_4000` | 0/10 |
| `unreserved_priority_only_x4_slots_8000` | 0/10 |
| `rb_reserved_both_dynamic_x4_slots_2000` | 0/10 |
| `rb_reserved_both_dynamic_x4_slots_4000` | 0/10 |
| `rb_reserved_both_dynamic_x4_slots_8000` | 0/10 |
| `unreserved_both_dynamic_x4_slots_2000` | 0/10 |
| `unreserved_both_dynamic_x4_slots_4000` | 0/10 |
| `unreserved_both_dynamic_x4_slots_8000` | 0/10 |

Total: **31/120 (≈26%) runs complete**.

## Why this gap matters

The D-33 steady-state criterion needs per-(job, length, N seeds) rolling
mean comparisons. With only one of four menu arms covered at full N, the
suite cannot answer "which menu options need slot length > 2000 to reach
steady state". Reporting the partial data for the rb-reserved priority-only
arm alone would imply a stronger conclusion than the data supports — the
other three arms might have very different steady-state behaviour, and
the suite default-raise recommendation would be partial.

## Recommendation

**Re-run the full suite.** From `sim-rs/`:

```bash
scripts/run-robustness-suites.sh 1 \
    parameters/phase-2-sweep/suites/robustness-run-length.yaml
```

Expected wall-clock at `-P 8`: 12 jobs × 10 seeds × variable wall-clock
(2000 / 4000 / 8000 slots → ~94 / 188 / 376 s per seed). Total compute:
4 × (10×94 + 10×188 + 10×376) = ~27000 s = ~7.5 hours of compute.
At parallelism 8: ~56 min wall-clock.

`experiment-suite run --run-id <existing-id>` is resumable, so re-running
with the same batch id picks up the 31 already-complete (job, seed) pairs
and only runs the remaining 89.

## Partial result (rb-reserved priority-only arm only — informational)

The one arm with complete data (`rb_reserved_priority_only_x4_slots_{2000, 4000}`)
allows a single-arm D-33 verdict. This is not propagated into
`coverage-check.md` because the menu-option comparison the test was designed
to support requires coverage of all four arms.

A full re-run will populate this section with per-(job, length) STEADY
verdicts and any suite-default-raise recommendations. The analyser
(`sim-rs/scripts/analyse-robustness.py`) does NOT currently parse
`time_series.csv` for the D-33 rolling-mean computation — that logic will
be added when the data is in hand and the comparison can run across all
4 menu arms.

## Coverage-check impact

The CLM-NN rows referencing `RSK-steady-state-run-length` keep their
existing pre-robustness status until the re-run completes.

## Abbreviations on first use

- **CLM** — claim identifier in `../../audit-documents/coverage-check.md`
- **RSK** — realism-risk identifier in `../../audit-documents/realism-risks-register.md`
