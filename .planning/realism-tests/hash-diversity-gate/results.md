# Wave 3 — COV-05 Hash-Diversity Gate Report

**Run id:** `20260518-084846`
**Gate semantics (CONTEXT.md D-19 + REQUIREMENTS.md COV-05):** for every
`BACKED`-labelled row in `docs/phase-2/coverage-check.md`, the count of
distinct `pricing_event_stream.sha256` values across the cited seeds
must equal the seed count. If `distinct < N`, the seed-set has collapsed
(the simulator produced the same event sequence twice from different
seeds, suggesting hidden coupling) and the verdict is downgraded to
`WEAK` with annotation `hash collision detected: distinct count = K < N`,
or marked `re-run-needed` with different seed values.

## Per-suite results

### TEST-03 sign-flip-variance (6 jobs × 20 seeds)

| Job | Distinct hashes | Gate status |
|---|---|---|
| `cell_eip1559_d4_t50_w32` | 20/20 | PASS |
| `cell_eip1559_d8_t25_w32` | 20/20 | PASS |
| `control_eip1559_d8_t50_w32_base` | 20/20 | PASS |
| `cell_rb_reserved_x4_rb_quarter` | 20/20 | PASS |
| `cell_partitioned_x4_rb_quarter` | 20/20 | PASS |
| `control_eip1559_d8_t50_w32_rb_quarter` | 20/20 | PASS |

### TEST-04 canonical-variance (5 jobs × 20 seeds)

| Job | Distinct hashes | Gate status |
|---|---|---|
| `menu_rb_reserved_priority_only_static_x4` | 20/20 | PASS |
| `menu_unreserved_priority_only_static_x4` | 20/20 | PASS |
| `menu_rb_reserved_both_dynamic_x4` | 20/20 | PASS |
| `menu_unreserved_both_dynamic_x4` | 20/20 | PASS |
| `control_eip1559_d8_t50_w32` | 20/20 | PASS |

### TEST-07a multiplier-floor-16-companion (6 jobs × 5 seeds)

| Job | Distinct hashes | Gate status |
|---|---|---|
| `rb_scarcity_x16_baseline` | 5/5 | PASS |
| `rb_scarcity_x16_rb_half` | 5/5 | PASS |
| `rb_scarcity_x16_rb_third` | 5/5 | PASS |
| `rb_scarcity_x16_rb_quarter` | 5/5 | PASS |
| `urgency_inversion_x16_correctly_priced` | 5/5 | PASS |
| `urgency_inversion_x16_mispriced_high_urgency` | 5/5 | PASS |

### TEST-05 pool-number-sensitivity — gate not applied

Insufficient coverage (35/1650). Gate evaluation deferred until the cut
suite (165 runs) is executed. See `pool-number-sensitivity/results.md`.

### TEST-06 run-length / steady-state — gate not applied

Insufficient coverage (31/120). Gate evaluation deferred until the suite
completes. See `run-length-steady-state/results.md`.

## Summary

| Total `BACKED`-eligible rows checked | 17 |
| Passing | 17 |
| Downgraded to WEAK | 0 |
| Marked re-run-needed | 0 |

**Cross-cell observation (informational).** In TEST-07a, the cells
`rb_scarcity_x16_baseline` and `urgency_inversion_x16_correctly_priced`
produce **identical pricing_event_stream SHA-256 values at seeds 1 and 2**
(`749ecfe6c0e3dec0...` and `4c36fdc8200c79c9...`). This is across-cell,
not within-cell, so it does NOT trigger the within-cell hash-diversity
gate (which counts distinct hashes per cell). It IS a meaningful
mechanism finding documented in `multiplier-floor-16-companion/results.md`:
at multiplier_floor=16 under `paper_like_congested` demand, the partitioned-
both-dynamic mechanism degenerates to priority-only-static because the
standard controller never sees enough standard-lane demand to drift.

The Phase-3 deliverable that BACKED rows have N distinct hashes (per
COV-05) holds at 17/17. No further action required for Phase 3 closure.

## Cross-reference for Phase 4

If Phase 4 chooses to re-run TEST-05 or TEST-06, the gate must be re-
applied at the new run-id over the freshly-collected BACKED cells. The
gate is per-suite per-run-id; cross-run gate composition is not defined.

## Abbreviations on first use

- **SHA-256** — Secure Hash Algorithm 256-bit
- **CLM** — claim identifier in `docs/phase-2/coverage-check.md`
- **RSK** — realism-risk identifier in `docs/phase-2/realism-risks-register.md`
- **CIP** — Cardano Improvement Proposal
