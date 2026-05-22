# TEST-05 — Pool-Number Sensitivity Results


> **⚠️ SUPERSEDED 2026-05-21** — numerical claims below were computed under the
> pre-Cardano Improvement Proposal (CIP)-0164 EB-sizing simulator variant
> (`linear`, 12 megabyte (MB) EB wire object). Endorser Block (EB) certification
> failed under that variant, biasing every inclusion-rate / latency / welfare
> measurement. See [`../../../docs/phase-2/eb-sizing-fix-postmortem.md`](../../../docs/phase-2/eb-sizing-fix-postmortem.md) for the diagnosis and the re-run schedule.

**Status:** DATA-GAP (insufficient coverage; defer to Phase 4 disclosure)
**Run id:** `20260518-084846`
**Suite:** `robustness-pool-number-sensitivity.yaml`

## Coverage

The TEST-05 suite at this batch id captured **35/1650 (≈2.1%) runs** before
the runner was interrupted. The 1650-run figure reflects an earlier
over-scoped version of the suite (5 demand profiles × 2 pool counts × 5
seeds × 33 sundaeswap-smoke jobs); the suite was later cut to the 165-run
version (sundaeswap_moderate × 150-pool × 5 seeds only), but the cut
version did not get re-launched before the run was stopped. As a result
the partial data covers only a small slice of one demand profile
(`baseline_flat_fee` + a handful of others under `paper_like_*` /
`sundaeswap_moderate`) — not the 33-job mainline.

| Status | Count |
|---|---|
| completed | 35 |
| pending | 1607 |
| running (interrupted) | 8 |

## Why this gap matters

`RSK-pool-count` and `RSK-calibration-stale-stake-snapshot` in
`../../audit-documents/realism-risks-register.md` carry **LIVE** verdicts that
TEST-05 was the path to MITIGATED. With the gap, those entries remain
LIVE going into Phase 4. The Phase 1 plan-02 disclosure paragraph for
`RSK-pool-count` is the load-bearing fallback: it explicitly anticipates
the LIVE → DISCLOSED transition if the 150-pool data is not in hand.

## Recommendation for Phase 4

**Option A — Disclose without re-running.** Phase 4's
`../../audit-documents/cardano-realism-audit.md` and `validity-threats.md` refresh
includes the disclosure paragraph for `RSK-pool-count`. The CIP cites this
disclosure rather than measured Δ% values, and the test result column for
the relevant `CLM-NN` rows in `../../audit-documents/coverage-check.md` stays
`UNBACKED` (Phase 4 may downgrade to `DISCLOSED` if the verdict vocabulary
supports it; otherwise `UNBACKED` + annotation `"100-vs-150 not measured;
see RSK-pool-count disclosure"`).

**Option B — Re-run the cut TEST-05 suite (165 runs).** From `sim-rs/`:

```bash
scripts/run-robustness-suites.sh 1 \
    parameters/phase-2-sweep/suites/robustness-pool-number-sensitivity.yaml
```

Expected wall-clock: ~50 min at `-P 8` on a 32-core box. The 150-pool
side runs ~1.5× slower than 100-pool (1.5× nodes, gossip cost
approximately `O(N log N)` per slot). After the run completes, re-run
`python3 scripts/analyse-robustness.py <run-id>` to update this file with
proper Δ%-vs-IQR analysis.

**If Option B is chosen, the comparison is:**
- 150-pool seed-medians from this suite (33 jobs × 5 seeds)
- 100-pool seed-IQR from existing 3-seed data in
  `output/phase-2/sundaeswap-{singlelane, priority-only, both-dynamic}/<job>/{1,2,3}/run_summary.json`
- MITIGATED iff every job's `|Δ%|` is within its 100-pool seed-IQR; LIVE → DISCLOSED otherwise.

**Recommended action:** Option B. The 50-min compute cost is small relative
to the LIVE-going-to-DISCLOSED downgrade Option A imposes.

## Coverage-check impact

The CLM-NN rows referencing `RSK-pool-count` /
`RSK-calibration-stale-stake-snapshot` keep their existing pre-robustness
status (`UNBACKED` / `WEAK`) until Option B's data lands, OR are downgraded
explicitly to `UNBACKED` with annotation in Phase 4 if Option A is chosen.

## Abbreviations on first use

- **CLM** — claim identifier in `../../audit-documents/coverage-check.md` (`CLM-NN` format)
- **RSK** — realism-risk identifier in `../../audit-documents/realism-risks-register.md`
- **IQR** — Inter-Quartile Range
- **CIP** — Cardano Improvement Proposal
