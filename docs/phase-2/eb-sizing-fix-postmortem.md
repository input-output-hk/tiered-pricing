# EB-sizing fix postmortem (May 2026)

## What was wrong

The phase-2 sweep's protocol baseline pinned `leios-variant: linear`,
which sizes the Endorser Block (EB) wire object as the sum of the
referenced transactions' bytes (capped at
`eb-referenced-txs-max-size-bytes = 12 MB`). Under sustained demand
the EB approached its 12 MB cap, and a 12 MB block could not diffuse
across the 100-node topology within the linear-Leios voting window
`L_vote = 4 slots ≈ 2 seconds`. EBs failed to certify; the chain
fell back to the 90 kB Praos RB body for every slot; inclusion rates
collapsed to 16–30 % across every mechanism arm and every demand
profile.

The fix swaps the variant to `linear-with-tx-references` per
CIP-0164's S_EB / S_EB-tx split: the EB wire object carries only
32-byte references to mempool transactions and is bounded
independently at `S_EB = 512 kB`, while the referenced-transaction
total remains bounded at `S_EB-tx = 12 MB`. The 512 kB EB wire
object diffuses easily within `L_vote`, so EB certification works,
the chain uses the 12 MB EB body path, and inclusion rates rise to
~98–100 % across every mechanism arm.

## Visible consequences

Under the pre-fix (`linear`) variant, all phase-2 and robustness runs
predating 2026-05-21 exhibit:

- **Inclusion rate of 16–30 % across all jobs**, dominated by the
  90 kB Praos RB body throughput rather than the EB body.
- **Latency of 12–42 blocks for most components** versus the
  ~2.5-block latency the spec predicts.
- **0 % inclusion for patient (low-urgency) standard-fee
  transactions under the RB-reserved variants**. Standard-fee
  transactions cannot enter the RB body in RB-reserved (the on-chain
  validity rule rejects them) and could not enter the EB body either
  (no EB ever certified), so every patient-traffic component
  reported `included = 0`.
- **Negative welfare for high-urgency components under single-lane
  EIP-1559**. The dynamic controller pushed quotes up under demand,
  inclusion stayed at 2–14 %, and the urgent components paid
  elevated fees on a tiny fraction of submissions while losing value
  on the rest.
- **Mechanism distinctions invisible**: the four candidate mechanism
  arms (priority-only-static {RB-reserved, un-reserved} × both-dynamic
  {RB-reserved, un-reserved}) and the two controls (flat-fee,
  EIP-1559) produced essentially the same welfare numbers because the
  EB-diffusion bottleneck dominated everything.

For a side-by-side comparison see [`robustness-fifo-smoke` pre-fix vs
post-fix](#paired-evidence) below.

## Root cause and fix

**Root cause.** The simulator's `linear` Leios variant predates the
CIP-0164 specification of the S_EB / S_EB-tx split. Under `linear`,
the EB serialises a transaction-bodies blob whose size scales with
the referenced transactions' total bytes. CIP-0164's Linear Leios
specifies that the EB carries a *bounded reference structure* of
32-byte transaction hashes — the actual transaction bodies travel
through the existing mempool layer and don't need to diffuse with the
EB. The simulator's `linear` variant was a pessimistic over-
approximation of EB diffusion cost under that spec.

**Fix.** A new `LeiosVariant::LinearWithTxReferences` is added to the
simulator. Under this variant the EB wire size is computed as
`eb_constant + (32 × tx_count)` rather than `eb_constant + Σ tx_bytes`,
and a new protocol knob `eb-max-size-bytes` (= 512 000 per CIP-0164
Table 7) bounds the wire object independently from
`eb-referenced-txs-max-size-bytes` (= 12 000 000 per the same table).

The simulator's `select_eb_with_partition` now caps EB packing by
*both* total bytes (`S_EB-tx`) and reference count (derived from
`S_EB`); fullness/partition-activation triggers if either limit
saturates. A new helper `BlockSizeConfig::linear_eb_reference_count_limit`
performs the derivation. See:

- `sim-rs/sim-core/src/config.rs` — `RawParameters::eb_max_size_bytes`,
  `SimConfiguration::max_eb_wire_size`,
  `BlockSizeConfig::linear_eb_reference_count_limit` (with unit tests).
- `sim-rs/sim-core/src/sim/linear_leios.rs` — `select_eb_with_partition`
  feeds `max_reference_count` to `sample_from_mempool_lane_aware`,
  and the partition-activation rule is widened from
  "selected_bytes ≥ capacity" to "selected_bytes ≥ capacity OR
  refs_saturated".
- `sim-rs/parameters/phase-2-sweep/protocol-base.yaml` and the three
  `protocol-rb-reduced-*.yaml` overlays — flip
  `leios-variant: linear` → `leios-variant: linear-with-tx-references`
  and add `eb-max-size-bytes: 512000`.

## Paired evidence

The `robustness-fifo-smoke` suite was run twice on the same demand /
topology / pricing / mechanism configuration, once before and once
after the fix. Each cell is `mean latency (blocks) / inclusion rate
(%)`, weighted by included-tx volume across 3 seeds × all components
in the tier.

### Pre-fix (`linear`, 12 MB EB wire object, 2026-05-20)

| tier | flat | EIP | RB-res FIFO | unres PO FIFO | unres BD FIFO |
|---|---|---|---|---|---|
| **very-high** (≤2 min) | 41.6 / 27.5% | 12.9 / 6.9% | 15.8 / 27.1% | 41.6 / 27.6% | 19.0 / 7.1% |
| **high** (5–10 min) | 42.0 / 23.9% | 11.7 / 2.4% | 16.0 / 15.5% | 42.0 / 24.1% | 19.2 / 7.9% |
| **medium** (15–30 min) | 30.0 / 26.0% | 6.5 / 19.3% | 6.9 / 7.7% | 29.9 / 26.1% | 11.5 / 21.4% |
| **low** (1–6 h) | 15.8 / 27.5% | 6.0 / 46.3% | 4.2 / 1.0% | 15.8 / 27.6% | 9.3 / 41.8% |
| **very-low** (≥1 day) | 15.9 / 27.9% | 5.8 / 50.4% | -- / 0.0% | 15.9 / 27.9% | 8.6 / 41.5% |

### Post-fix (`linear-with-tx-references`, 512 kB EB wire object, 2026-05-21)

| tier | flat | EIP | RB-res FIFO | unres PO FIFO | unres BD FIFO |
|---|---|---|---|---|---|
| **very-high** (≤2 min) | 3.0 / 100.0% | 3.0 / 100.0% | 2.3 / 100.0% | 3.0 / 100.0% | 3.0 / 100.0% |
| **high** (5–10 min) | 2.9 / 99.5% | 2.9 / 99.5% | 2.5 / 99.4% | 3.0 / 99.5% | 2.9 / 99.5% |
| **medium** (15–30 min) | 2.7 / 99.1% | 2.7 / 99.1% | 2.6 / 98.8% | 2.8 / 99.1% | 2.7 / 99.1% |
| **low** (1–6 h) | 2.5 / 98.2% | 2.5 / 98.2% | 2.6 / 97.8% | 2.5 / 98.2% | 2.4 / 98.2% |
| **very-low** (≥1 day) | 2.5 / 98.2% | 2.5 / 98.2% | 2.6 / 97.7% | 2.5 / 98.2% | 2.5 / 98.2% |

The pre-fix run is at
[`sim-rs/output/robustness/fifo-smoke-20260520-165952`](../../sim-rs/output/robustness/fifo-smoke-20260520-165952);
the post-fix run is at
[`sim-rs/output/robustness/fifo-smoke-20260521-093755`](../../sim-rs/output/robustness/fifo-smoke-20260521-093755).

## What this invalidates

Every numerical claim in the evidence base that depends on EB
certification is computed against the broken-EB regime and requires
re-verification under `linear-with-tx-references`.

### Must re-run (load-bearing for CIP claims)

| Suite | Cardano Improvement Proposal (CIP) headline backed | Authored seeds × jobs |
|---|---|---|
| [`robustness-canonical-variance`](../../sim-rs/parameters/phase-2-sweep/suites/robustness-canonical-variance.yaml) (TEST-04) | Headline Claims 1, 2, 4; CLM-05..09 | 20 × 5 |
| [`robustness-sign-flip-variance`](../../sim-rs/parameters/phase-2-sweep/suites/robustness-sign-flip-variance.yaml) (TEST-03) | Headline Claim 5; CLM-10..13 | 20 × 6 |
| [`robustness-multiplier-floor-16-companion`](../../sim-rs/parameters/phase-2-sweep/suites/robustness-multiplier-floor-16-companion.yaml) (TEST-07a) | Headline Claim 3 | 5 × 6 |
| [`phase-2-eip1559-robustness`](../../sim-rs/parameters/phase-2-sweep/suites/phase-2-eip1559-robustness.yaml) | Goldens-pinned | 3 × 5 |
| [`phase-2-eip1559-smoothing`](../../sim-rs/parameters/phase-2-sweep/suites/phase-2-eip1559-smoothing.yaml) | Goldens-pinned | 3 × 3 |
| [`phase-2-priority-only-rb-reserved`](../../sim-rs/parameters/phase-2-sweep/suites/phase-2-priority-only-rb-reserved.yaml) | Goldens-pinned | 3 × 3 |
| [`phase-2-priority-only-unreserved`](../../sim-rs/parameters/phase-2-sweep/suites/phase-2-priority-only-unreserved.yaml) | Goldens-pinned | 3 × 3 |
| [`phase-2-two-lane-both-dynamic`](../../sim-rs/parameters/phase-2-sweep/suites/phase-2-two-lane-both-dynamic.yaml) | Goldens-pinned | 3 × 4 |
| [`phase-2-rb-scarcity`](../../sim-rs/parameters/phase-2-sweep/suites/phase-2-rb-scarcity.yaml) | Goldens-pinned | 3 × 4 |
| [`phase-2-urgency-inversion`](../../sim-rs/parameters/phase-2-sweep/suites/phase-2-urgency-inversion.yaml) | Goldens-pinned | 3 × 2 |

After the seven goldens-pinned suites re-run, the suite-level
`.goldens/*.sha256` files must be regenerated via:

```sh
cd sim-rs
UPDATE_GOLDENS=1 cargo test --release -- --ignored determinism
```

### Should re-run (extends generalisation; not load-bearing for headlines)

The 12-suite demand-regime matrix:
`phase-2-{congested,moderate,realistic,sundaeswap}-{singlelane,priority-only,both-dynamic}.yaml`.
Each carries 8 / 15 / 10 jobs depending on mechanism family, all at
3 seeds. Total: ~99 jobs × 3 seeds = ~297 (job, seed) pairs.

### Mark as superseded; don't re-run

| Suite | Reason |
|---|---|
| `robustness-pool-number-sensitivity` (TEST-05) | 35 of 1650 runs (~2 %); already disclosed at `RSK-pool-count`; not load-bearing. |
| `robustness-run-length` (TEST-06) | 31 of 120 runs (~26 %); already disclosed at `RSK-steady-state-run-length`; only one menu arm covered. |
| Earlier `robustness-fifo-smoke-*` runs (20260520) | Superseded by `fifo-smoke-20260521-093755`. |
| Earlier `robustness-fifo-sensitivity-*` runs (20260520-164*) | Re-run under new variant alongside the smoke suite. |

### Documents that cite invalidated numerics

The following carry concrete BCa CI numerics, point estimates, or
inclusion-rate / latency tables computed under the pre-fix variant.
Each carries a SUPERSEDED banner pointing at this postmortem:

- [`cip-evidence/cip-author-summary.md`](../../cip-evidence/cip-author-summary.md) — Headline Claims 1–6
- [`cip-evidence/audit-documents/coverage-check.md`](../../cip-evidence/audit-documents/coverage-check.md) — CLM-05..13 most directly
- [`cip-evidence/audit-documents/validity-threats.md`](../../cip-evidence/audit-documents/validity-threats.md) — 19 per-suite trust verdicts
- [`cip-evidence/audit-documents/realism-risks-register.md`](../../cip-evidence/audit-documents/realism-risks-register.md) — entries citing robustness-suite evidence
- [`cip-evidence/audit-documents/methodology-overview.md`](../../cip-evidence/audit-documents/methodology-overview.md) — worked-example numerics
- [`cip-evidence/audit-documents/sweep-design.md`](../../cip-evidence/audit-documents/sweep-design.md) — §6 robustness table
- [`cip-evidence/test-results/multi-seed-variance/results.md`](../../cip-evidence/test-results/multi-seed-variance/results.md)
- [`cip-evidence/test-results/multi-seed-variance/scoping-results.md`](../../cip-evidence/test-results/multi-seed-variance/scoping-results.md)
- [`cip-evidence/test-results/multiplier-floor-16-companion/results.md`](../../cip-evidence/test-results/multiplier-floor-16-companion/results.md)
- [`cip-evidence/test-results/hash-diversity-gate/results.md`](../../cip-evidence/test-results/hash-diversity-gate/results.md)
- [`cip-evidence/test-results/pool-number-sensitivity/results.md`](../../cip-evidence/test-results/pool-number-sensitivity/results.md)
- [`cip-evidence/test-results/run-length-steady-state/results.md`](../../cip-evidence/test-results/run-length-steady-state/results.md)

### Documents that need a content update (not just a banner)

- [`cip-evidence/audit-documents/cardano-realism-audit.md`](../../cip-evidence/audit-documents/cardano-realism-audit.md):
  add a new `(eb-max-size-bytes = 512 kB, source: CIP-0164 Table 7,
  date-retrieved: 2026-05-21)` triple under §"What lines up with
  mainnet" / §"Endorser block sizing"; refresh the simulator-vs-spec
  alignment narrative to note that pre-fix `linear` over-approximated
  EB diffusion cost.

### Documents that DO NOT need updating

- The mechanism specification ([`docs/phase-2/mechanism-design.md`](mechanism-design.md))
  — unchanged; the EB-sizing fix is a simulator-fidelity improvement,
  not a mechanism-design change.
- The Family B chain-derived controller decision
  ([`.planning/family-b-decision-2026-05-14.md`](../../.planning/family-b-decision-2026-05-14.md))
  — orthogonal axis; Family B remains the committed publication
  mechanism.
- The earlier RB-cadence calibration postmortem
  ([calibration-fix-postmortem.md](calibration-fix-postmortem.md))
  — its findings about `rb-generation-probability` are unchanged.
- The simulator source code outside the EB-sizing fix — the pricing
  kernel, the BCa bootstrap library, the actor model, and the
  topology generator are unchanged.

## Re-run schedule

The critical path is `robustness-canonical-variance` + `robustness-sign-flip-variance`
+ `robustness-multiplier-floor-16-companion`. These three suites are the
backbone of the CIP claims and together fit comfortably inside a
half-day of compute at parallelism 8. The seven goldens-pinned
phase-2 suites can be batched alongside them. The 12-suite
demand-regime matrix is lower priority and can be backfilled
asynchronously.

The exact run commands are in §"Running the re-runs" below.

### Running the re-runs

All commands assume `pwd = sim-rs/`.

**Critical path** (the load-bearing robustness suites + the 7 goldens-
pinned phase-2 suites — should be one batch under a shared
`ROBUSTNESS_RUN_ID`):

```sh
# Robustness critical path
scripts/run-robustness-suites.sh 8 \
    parameters/phase-2-sweep/suites/robustness-canonical-variance.yaml \
    parameters/phase-2-sweep/suites/robustness-sign-flip-variance.yaml \
    parameters/phase-2-sweep/suites/robustness-multiplier-floor-16-companion.yaml

# Goldens-pinned phase-2 suites (the audit-of-record set)
for s in phase-2-eip1559-robustness phase-2-eip1559-smoothing \
         phase-2-priority-only-rb-reserved phase-2-priority-only-unreserved \
         phase-2-two-lane-both-dynamic phase-2-rb-scarcity \
         phase-2-urgency-inversion; do
    cargo run --release --bin experiment-suite -- run \
        --parallelism 8 \
        parameters/phase-2-sweep/suites/${s}.yaml
done

# Regenerate suite-level goldens against the new event streams
UPDATE_GOLDENS=1 cargo test --release -- --ignored determinism
git add parameters/phase-2-sweep/suites/.goldens/
git commit -m "M5 goldens regenerated against linear-with-tx-references"
git tag -a m5-goldens-post-eb-fix -m "Re-pin against CIP-0164 EB sizing"
```

**Should-re-run** (the 12-suite demand-regime matrix; backfill
asynchronously):

```sh
for demand in congested moderate realistic sundaeswap; do
    for family in singlelane priority-only both-dynamic; do
        cargo run --release --bin experiment-suite -- run \
            --parallelism 8 \
            parameters/phase-2-sweep/suites/phase-2-${demand}-${family}.yaml
    done
done
```

**Smoke-suite under congested demand** (new probe authored
2026-05-21 specifically to surface the urgency separation that low
sundaeswap demand was hiding):

```sh
cargo run --release --bin experiment-suite -- run \
    --parallelism 8 \
    parameters/phase-2-sweep/suites/robustness-fifo-smoke-congested.yaml
```

After every (job, seed) lands, `experiment-suite verify` confirms the
pricing-event-stream SHA-256 hashes match the persisted values:

```sh
for s in parameters/phase-2-sweep/suites/*.yaml; do
    cargo run --release --bin experiment-suite -- verify "$s"
done
```

## Followup work after the re-runs land

Once the new BCa CIs are computed against the post-fix runs:

1. Refresh the six Headline Claims in
   [`cip-author-summary.md`](../../cip-evidence/cip-author-summary.md)
   with new BCa CI numerics. Mechanism *direction* may or may not
   survive — the previously-hidden urgency separation under
   contended priority partition (now visible under congested demand
   per [`robustness-fifo-smoke-congested`](../../sim-rs/parameters/phase-2-sweep/suites/robustness-fifo-smoke-congested.yaml))
   may reorder some menu-arm claims.
2. Refresh the 9 directly-affected CLM rows
   (CLM-05..13) in
   [`coverage-check.md`](../../cip-evidence/audit-documents/coverage-check.md).
3. Refresh the per-suite trust verdicts in
   [`validity-threats.md`](../../cip-evidence/audit-documents/validity-threats.md);
   the trust framework is unchanged but per-suite verdicts may
   shift now that the suites actually produce distinguishable
   behaviour.
4. Add the `eb-max-size-bytes = 512 kB` triple to
   [`cardano-realism-audit.md`](../../cip-evidence/audit-documents/cardano-realism-audit.md).
5. Remove the SUPERSEDED banners across the documents listed in
   §"Documents that cite invalidated numerics" above.
6. Run `cip-evidence/consistency-audit/verify-consistency.sh` and
   confirm `OVERALL: PASS`.
7. Apply the `phase-2-cip-evidence-v1` git tag (it was deliberately
   not applied at Phase 5 close — see `cip-author-summary.md` §"HAND-03
   execution note") against the post-re-run commit.
