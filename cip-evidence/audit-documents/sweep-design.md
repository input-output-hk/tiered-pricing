# Sweep design — phase-2 dynamic-pricing simulator

This document is the source-of-truth for *how the phase-2 experimental
sweep is organised*. It names the design pattern, pins the constants
held fixed across every job, identifies the baseline reference
configuration that every other job is read relative to, and tabulates
every suite as a row: what question it asks, what axis it varies, what
levels are swept, what is held constant, and what signal answers the
question.

## Abbreviations used in this document

Per the project convention (CLAUDE.md §"Conventions / gotchas"), every
abbreviation is spelled out on first use. Collected here for reference:

| Abbreviation | Expansion |
|---|---|
| BCa | Bias-corrected and accelerated (the bootstrap interval method used in the robustness suites) |
| CI | Confidence Interval |
| CIP | Cardano Improvement Proposal |
| EB | Endorser Block (the linear-Leios endorsement block) |
| EIP-1559 | Ethereum Improvement Proposal 1559 — the dynamic fee-pricing rule from Ethereum |
| Family B | The committed publication controller: chain-derived, exactly one update step per canonical block (see `.planning/family-b-decision-2026-05-14.md`) |
| MaxFeePolicy | Actor-side configuration: how much headroom the actor leaves between the lane quote at submission and the user's `max_fee_lovelace` |
| RB | Ranking Block (the Cardano-style protocol block, capacity ~90 KB) |
| RSK-NN | Realism Risk identifier (used in [`realism-risks-register.md`](realism-risks-register.md)) |
| YAML | Yet Another Markup Language (the configuration file format used for every suite, demand, pricing and protocol overlay) |

## 1. Design pattern — mechanism-comparison with embedded factorials

The sweep has three nested layers.

1. **Mechanism arms (outer layer).** The phase-2 down-select left
   four live transaction-fee mechanism candidates plus a single-lane
   EIP-1559 control (see [`mechanism-design.md` §"Live mechanisms"](../../docs/phase-2/mechanism-design.md#L107)).
   Each mechanism is a different conditional structure: single-lane
   has one fee coefficient, two-lane mechanisms have two coefficients
   with a multiplier-floor invariant between them, the RB-reserved
   variants enforce a partition rule on chain, and the un-reserved
   variants enforce nothing on chain. These are not knob settings
   that can be swept linearly; they are different mechanisms with
   different validity rules, controller signals, and refund
   semantics.

2. **Calibration axes within an arm (middle layer).** Once a
   mechanism arm is fixed, each suite varies one or two calibration
   knobs across a small set of levels and runs the Cartesian product.
   The EIP-1559 robustness suite sweeps two knobs (step-size
   denominator `D` and target utilisation) within the single-lane
   mechanism; the priority-only suites sweep the multiplier floor
   within their respective partition variants; the RB-scarcity suite
   sweeps the RB body capacity within the RB-reserved priority-only
   mechanism. The factorial structure is small (typically 3–5 jobs
   per suite) because the goal is to characterise the mechanism arm
   under representative calibrations, not to perform a full
   sensitivity analysis.

3. **Demand profile (orthogonal layer).** The 12 demand-regime
   suites cross three mechanism families (single-lane, priority-only,
   both-dynamic) against four demand profiles (paper-like-congested,
   paper-like-moderate, paper-like-realistic, sundaeswap-moderate).
   Each (mechanism × demand) cell carries the full per-family
   calibration sweep (8 jobs for single-lane, 15 for priority-only,
   10 for both-dynamic; precise per-suite job counts in §5 below),
   so the demand-regime suites are not small probes — they re-run
   the calibration matrix at each demand profile. The seven
   goldens-pinned suites in §4 are *question-isolated subsets* of
   the paper-like-congested row of this matrix.

The sweep is designed to compare mechanism arms under a representative
range of calibrations and demand regimes, and to characterise how
each mechanism arm behaves under stress.

## 2. Constants — held fixed across every phase-2 and robustness job

The following knobs are pinned identically on every job in the design.
A change to any of these would invalidate the cross-suite comparison.

| Constant | Value | Source-of-truth |
|---|---|---|
| Topology | `topology-realistic-100.yaml` — 100 nodes, mainnet-shaped stake distribution (Cardano mainnet epoch 582, retrieved 2026-05-14). The `robustness-pool-number-sensitivity` suite overrides per-job to `topology-realistic-150.yaml` (150 nodes, same mass-stratified curve). | [`cardano-realism-audit.md` §"Topology and actor model"](cardano-realism-audit.md) |
| Protocol base | `protocol-base.yaml` (carries `min_fee_a = 44 lovelace/byte`, `min_fee_b = 155,381 lovelace/transaction`, `mempool-max-total-size-bytes = 2 × eb_referenced_txs_max_size_bytes`) | [`mechanism-design.md` §"Era floor"](../../docs/phase-2/mechanism-design.md#L62) |
| EB sizing | `leios-variant: linear-with-tx-references`. The EB wire object is bounded at `eb-max-size-bytes = 512 000` bytes (CIP-0164 Table 7's S_EB — the bounded 32-byte-reference structure); the referenced-transaction total is bounded independently at `eb-referenced-txs-max-size-bytes = 12 000 000` bytes (CIP-0164's S_EB-tx). | [`docs/phase-2/eb-sizing-fix-postmortem.md`](../../docs/phase-2/eb-sizing-fix-postmortem.md) |
| Ranking-block (RB) generation probability | `0.05` (one RB ≈ every 20 slots — clears the 13-slot linear-Leios endorsement window so EBs land on chain) | CLAUDE.md §"Calibration choices"; [`docs/phase-2/calibration-fix-postmortem.md`](../../docs/phase-2/calibration-fix-postmortem.md) |
| Default slot count per (job, seed) | `2000` slots. The `robustness-run-length` suite varies this to 4000 and 8000 slots for the four CIP menu arms. | [`robustness-run-length.yaml`](../../sim-rs/parameters/phase-2-sweep/suites/robustness-run-length.yaml) |
| Lane-signal-source for un-reserved priority | Option 1: `priority_paying_bytes / total_block_capacity` | CLAUDE.md §"Calibration choices"; [`mechanism-design.md` lines 207–211](../../docs/phase-2/mechanism-design.md#L207) |
| Standard-side signal source (both-dynamic) | Capacity-weighted aggregate of `standard_paying_bytes` against `eb_referenced_txs_max_size_bytes` over endorser blocks; no standard sample on RB-reserved ranking blocks | CLAUDE.md §"Calibration choices"; [`mechanism-design.md` line 238](../../docs/phase-2/mechanism-design.md#L238) |
| Block-build scan order | `priority_first` for canonical suites; `fifo` only in the robustness FIFO smoke probe | [`robustness-fifo-smoke.yaml`](../../sim-rs/parameters/phase-2-sweep/suites/robustness-fifo-smoke.yaml) |
| Anti-standard cap | Not implemented in the quick FIFO smoke probe; strict reserved-FIFO block building remains a follow-up | [`mechanism-design.md` §"Methodology: simulator approximations"](../../docs/phase-2/mechanism-design.md#L301) |
| Controller cadence | One controller step per canonical block (Family B chain-derived) | [`.planning/family-b-decision-2026-05-14.md`](../../.planning/family-b-decision-2026-05-14.md) |
| EIP-1559 update rule | `c ← c × (1 + clamp((aggregateUtil − target) / (target × D), −1/D, +1/D))`, floored at `c = 1` | [`mechanism-design.md` §"Single-lane EIP-1559"](../../docs/phase-2/mechanism-design.md#L119) |
| Capacity-weighted aggregate utilisation window (for capacity-varying signals) | Length 32 priced blocks; the RB-reserved priority controller uses length 1 (mathematically reduces to per-block fill rate against the priority partition) | CLAUDE.md §"Calibration choices" |

The full enumeration of every spec-open knob the simulator pinned to a
default value, together with the cost of re-calibrating each, is in
[`mechanism-design.md` §"Calibration choices"](../../docs/phase-2/mechanism-design.md#L336)
and CLAUDE.md §"Calibration choices".

## 3. Baseline reference configuration

Every welfare claim in the phase-2 evidence base is read as a delta
against this baseline. The baseline is the single-lane EIP-1559
control under sundaeswap-moderate demand on the 100-node
mainnet-shaped topology — the same configuration the robustness
canonical-variance suite uses for its paired-bootstrap control.

| Layer | File | Knob | Value | Rationale |
|---|---|---|---|---|
| Topology | `topology-realistic-100.yaml` | nodes | 100 (mainnet-shaped) | Pinned per §2 |
| Protocol | `protocol-base.yaml` | `min_fee_a` | 44 lovelace/byte | Era floor |
| Protocol | `protocol-base.yaml` | `min_fee_b` | 155,381 lovelace/transaction | Era floor |
| Protocol | `protocol-base.yaml` | `mempool-max-total-size-bytes` | 2 × `eb_referenced_txs_max_size_bytes` | Spec finite-mempool default |
| Protocol | `protocol-base.yaml` | `rb-generation-probability` | 0.05 | Clears linear-Leios endorsement window |
| Demand | `demand/sundaeswap_moderate.yaml` | actor profile | SundaeSwap-derived 11-component profile (3 background-transfer components + 3 background-DeFi components + 3 DEX-launch retail components + 2 arbitrage-bot components), phased baseline / DEX-spike / cooldown / recovery at 10 / 50 / 30 / 15 transactions per slot | Robustness paired-bootstrap default |
| Pricing | `pricing/eip1559_d8_target0.5_window32.yaml` | mechanism arm | single-lane EIP-1559 | Robustness control |
| Pricing | `pricing/eip1559_d8_target0.5_window32.yaml` | step-size denominator `D` | 8 | Matches Ethereum mainnet (EIP-1559 spec); see [`cardano-realism-audit.md`](cardano-realism-audit.md) |
| Pricing | `pricing/eip1559_d8_target0.5_window32.yaml` | target utilisation | 0.5 | Matches Ethereum mainnet (EIP-1559 spec) |
| Pricing | `pricing/eip1559_d8_target0.5_window32.yaml` | window length | 32 priced blocks | Picked arbitrarily as a round number; the `phase-2-eip1559-smoothing` suite (§4 suite 2) sweeps {16, 32, 64} to bracket the choice. See `RSK-un-anchored-controller-knobs` in [`realism-risks-register.md`](realism-risks-register.md) for the per-knob rationale. |
| Multiplier floor (two-lane only; not applicable at baseline) | — | — | n/a (no priority lane in single-lane) | — |
| Slots per run | every suite | `default-slots` | 2000 | Pinned per §2 |
| Seeds | robustness canonical-variance | `seeds` | 1..20 | Sample size chosen by the TEST-02 wall-clock scoping suite (`robustness-scoping.yaml`, N=5 seeds × 1 job) to fit within the ~30 min × parallelism total-compute budget per cell |

This baseline corresponds exactly to the job
`control_eip1559_d8_t50_w32` in
[`robustness-canonical-variance.yaml`](../../sim-rs/parameters/phase-2-sweep/suites/robustness-canonical-variance.yaml).
The four menu options in the robustness canonical-variance suite are
each named `menu_<option>_x4`.

The mechanism-characterisation suites (§4 below) use a *different*
default demand profile — `paper_like_congested.yaml` rather than
`sundaeswap_moderate.yaml`. This is a mechanism-characterisation
versus robustness-validity distinction: the mechanism-characterisation
suites use a stylised reference load to characterise mechanism
behaviour under maximum stress; the robustness suites use the
empirically-anchored SundaeSwap-derived profile for paired-bootstrap
welfare claims. The two demand profiles are sketched in §5 below.

## 4. The main suite design table — phase-2 mechanism-characterisation (7 suites)

The seven suites below are the goldens-pinned phase-2 mechanism-characterisation
suites — the ones whose `pricing_event_stream.sha256` hashes are
pinned in
[`sim-rs/parameters/phase-2-sweep/suites/.goldens/`](../../sim-rs/parameters/phase-2-sweep/suites/.goldens/).
These are the suites that characterise *what mechanism arms do under
stress*.

Each row is one suite. Every job in every suite shares the constants
in §2 plus the suite's own `default-protocol`, `default-topology`,
and `default-demand` (the phase-2 default is
`paper_like_congested.yaml`). The "Axis varied" column names what the
suite sweeps; the "Levels swept" column names the concrete values; the
"Other knobs held at" column names everything else not explicitly
varied (the suite-baseline).

| # | Suite | Mechanism arm | Question | Axis varied | Levels swept | Other knobs held at | Signal |
|---|---|---|---|---|---|---|---|
| 1 | `phase-2-eip1559-robustness` | Single-lane EIP-1559 | Does the EIP-1559 controller remain stable across plausible step-size and target choices? | Step-size denominator `D` × target utilisation (factorial 3 × 3 partial = 5 jobs) | `D` ∈ {4, 8, 16} at target=0.5; target ∈ {0.25, 0.5, 0.75} at D=8 | Window length = 32; `paper_like_congested` demand; multiplier floor n/a (single-lane) | Quote trajectory stability vs. demand; absence of runaway under any `(D, target)` combination |
| 2 | `phase-2-eip1559-smoothing` | Single-lane EIP-1559 | How much does window length matter for controller smoothness? | Capacity-weighted aggregate window length (3 jobs) | window ∈ {16, 32, 64} priced blocks | `D` = 8; target = 0.5; `paper_like_congested` demand | Quote-trajectory variance across window lengths; absence of sharp regime change |
| 3 | `phase-2-priority-only-rb-reserved` | Two-lane, priority-only-static, RB-reserved partition | How does the multiplier floor between standard and priority lanes affect priority demand and welfare under the RB-reserved partition rule? | Multiplier floor (3 jobs) | multiplier_floor ∈ {4, 8, 16} | `paper_like_congested` demand; default RB body cap | Per-component inclusion rate on priority lane; controller drift; cross-job welfare ordering |
| 4 | `phase-2-priority-only-unreserved` | Two-lane, priority-only-static, no partition | Same as suite 3 but without the on-chain partition rule — how does soft priority via `priority_first` ordering compare to RB-reserved? | Multiplier floor (3 jobs) | multiplier_floor ∈ {4, 8, 16} | `paper_like_congested` demand; default RB body cap | Same as suite 3, plus cross-suite comparison RB-reserved vs un-reserved |
| 5 | `phase-2-two-lane-both-dynamic` | Two-lane, both lanes dynamic | When the standard lane also adapts to load, does that improve or hurt welfare relative to priority-only-static? Both partition variants tested. | Partition variant × multiplier floor (factorial 2 × 2 = 4 jobs) | partition ∈ {partitioned, un-reserved} × multiplier_floor ∈ {4, 16} | `paper_like_congested` demand; default RB body cap | Cross-job welfare ordering; standard-lane controller drift behaviour; multiplier-floor binding analysis |
| 6 | `phase-2-rb-scarcity` | Two-lane, priority-only-static, RB-reserved partition (one fixed pricing job, four protocol overlays) | How does priority service degrade as RB body capacity (and therefore priority partition capacity) is reduced? | RB body capacity (4 jobs) | RB body ∈ {90112 (baseline), 45056 (half), 30000 (third), 22528 (quarter)} bytes | multiplier_floor = 4 (lowered from 16 to lift priority demand into capacity stress); `paper_like_congested` demand | Cross-job priority inclusion gradient; degradation pattern (smooth vs. sharp regime change) |
| 7 | `phase-2-urgency-inversion` | Two-lane, both-dynamic, partitioned (one fixed pricing job, two demand profiles where the second is a one-component `MaxFeePolicy` overlay of the first) | Does urgency separation survive actor mis-pricing? Specifically, when a short-half-life actor leaves zero quote-drift headroom in its `MaxFeePolicy`, does the priority lane still deliver urgency separation, or does the actor get evicted? | `MaxFeePolicy` on the hard-deadline component, applied via a demand-file overlay (2 jobs) | `MaxFeePolicy` ∈ {`ScaledOverLaneQuote{4, 1}` (correctly priced, default 4× headroom, demand = `paper_like_congested.yaml`), `ScaledOverLaneQuote{1, 1}` (mis-priced, zero headroom, demand = `paper_like_mispriced.yaml` — identical to `paper_like_congested.yaml` except for component 0's `MaxFeePolicy`)} | `multiplier_floor` = 4 (lowered from 16 — same rationale as suite 6); all other components default to `MaxFeePolicy{4, 1}` | Eviction count under quote drift; refund envelope; per-component inclusion rate |

### Notes on the main table

- **Suites 6 and 7 lower the multiplier floor to 4.** Every other
  phase-2 mechanism-characterisation suite reports a multiplier-floor
  sweep that includes 16 (the spec default). Suites 6 and 7 pin the
  floor at 4 only, because at multiplier_floor = 16 the priority
  lane stays so much pricier than standard that the urgency components
  the suite needs (hard-deadline, active-DeFi) do not self-select
  into priority and the controller never drifts — so the scarcity
  (suite 6) and mis-pricing (suite 7) phenomena never appear in the
  signal. The robustness multiplier-floor-16-companion suite re-runs
  these two cells at floor = 16 to surface the regime-dependence
  (see §6 below).
- **Suites 1 and 2 share the EIP-1559 single-lane pricing family.**
  Together they form a 2-dimensional factorial across `D`, target,
  and window length within the single-lane arm — but they are
  authored as separate suites so each can be read on its own. The
  pricing YAMLs are not duplicated; the same
  `eip1559_d8_target0.5_window32.yaml` is the baseline cell of both
  suites.
- **Three seeds per job is the suite default.** Every
  phase-2 mechanism-characterisation suite runs seeds `[1, 2, 3]` —
  enough for cross-seed consistency but not enough to license a
  tight Confidence Interval (CI). The robustness suites raise the seed count to
  20 for the canonical-variance and sign-flip-variance probes; see
  `RSK-three-seed-statistical-power` in
  [`realism-risks-register.md`](realism-risks-register.md).

## 5. The demand-regime sweep table — phase-2 (12 suites)

The 12 suites below cross **three mechanism families** (single-lane,
priority-only, both-dynamic) against **four demand profiles**
(paper-like-congested, paper-like-moderate, paper-like-realistic,
sundaeswap-moderate). They are not goldens-pinned. Each cell carries
the *full per-mechanism-family calibration sweep* at that demand
profile — these are not small cross-cells, they are large per-family
matrices. The seven goldens-pinned suites in §4 are
question-isolated subsets of the `paper_like_congested` row of this
matrix; see "Relation to §4" below.

The four demand profiles in one sentence each (source files in
[`sim-rs/parameters/phase-2-sweep/demand/`](../../sim-rs/parameters/phase-2-sweep/demand/)):

- **`paper_like_congested.yaml`**: Phased load across the 2000-slot
  run — ramp-up (slots 0–400: 300 tx/slot), sustained overload
  (slots 400–1200: 600 tx/slot), recovery (slots 1200–2000: 200 tx/slot).
  At ~1 KB mean transaction size the overload phase delivers ~600 KB/slot,
  exceeding the 90 KB RB body cap and driving EB traffic, lane
  separation, quote drift, and the multiplier-floor invariant. Three
  weighted value-decay components (hard-deadline arbitrage, median
  half-life 60 s; active DeFi, median 5 min; patient traffic, median
  1 h). The default for the phase-2 mechanism-characterisation
  suites in §4.
- **`paper_like_moderate.yaml`**: Same three-component value-decay
  mix but at a flat 25 transactions per slot (~25 KB/slot, well
  below the 90 KB RB body cap — no congestion at this volume).
- **`paper_like_realistic.yaml`**: Flat 150 transactions per slot
  (~150 KB/slot). DeFi-heavy stress-day composition (~10 % hard-
  deadline arbitrage, ~40 % active DeFi, ~50 % patient traffic).
  Same three-component value-decay families as the other paper-like
  profiles.
- **`sundaeswap_moderate.yaml`**: Empirically-anchored profile derived
  from the SundaeSwap mainnet launch event (January 2022). Phased
  baseline (slots 0–400: ~10 tx/slot), DEX spike (slots 400–800:
  ~50 tx/slot), cooldown (slots 800–1200: ~30 tx/slot), recovery
  (slots 1200–2000: ~15 tx/slot). Eleven components — 3 background-
  transfer, 3 background-DeFi, 3 DEX-launch retail, 2 arbitrage-bot —
  with finer-grained value, size and half-life calibrations than
  the paper-like profiles. The default for the robustness suites. See
  `RSK-sundaeswap-demand-staleness` in
  [`realism-risks-register.md`](realism-risks-register.md) for the
  recency disclosure.

The 12 suites by mechanism family × demand profile:

| Mechanism family ↓ / Demand profile → | paper-like-congested | paper-like-moderate | paper-like-realistic | sundaeswap-moderate |
|---|---|---|---|---|
| Single-lane EIP-1559 (8 jobs per suite: `baseline_flat_fee` + 7 EIP-1559 cells across `D` × target × window) | `phase-2-congested-singlelane` | `phase-2-moderate-singlelane` | `phase-2-realistic-singlelane` | `phase-2-sundaeswap-singlelane` |
| Two-lane, priority-only-static (15 jobs per suite: 3 `multiplier_floor` levels × 2 partition variants — RB-reserved and un-reserved — + 9 RB-cap-overlay jobs on the RB-reserved sub-family) | `phase-2-congested-priority-only` | `phase-2-moderate-priority-only` | `phase-2-realistic-priority-only` | `phase-2-sundaeswap-priority-only` |
| Two-lane, both-dynamic (10 jobs per suite: 2 `multiplier_floor` levels × 2 partition variants + 6 RB-cap-overlay jobs on the partitioned sub-family) | `phase-2-congested-both-dynamic` | `phase-2-moderate-both-dynamic` | `phase-2-realistic-both-dynamic` | `phase-2-sundaeswap-both-dynamic` |

Seeds are `[1, 2, 3]` everywhere. None of these suites is
goldens-pinned. Total (job, seed) coverage across the 12-suite
matrix: 12 × {8, 15, 10}-per-row × 3 seeds = 396 (job, seed) pairs
(132 single-lane + 180 priority-only + 120 both-dynamic).

### Relation to §4

The seven goldens-pinned suites in §4 are subsets of the
`paper_like_congested` row of this matrix, plus the two suites that
pin `multiplier_floor = 4`:

| §4 suite | Subset of |
|---|---|
| `phase-2-eip1559-robustness` | `phase-2-congested-singlelane` (jobs `eip1559_{d4,d8,d16}_t50_w32`, `eip1559_d8_t25_w32`, `eip1559_d8_t75_w32`) |
| `phase-2-eip1559-smoothing` | `phase-2-congested-singlelane` (jobs `eip1559_d8_t50_w{16,32,64}`) |
| `phase-2-priority-only-rb-reserved` | `phase-2-congested-priority-only` (jobs `rb_reserved_x{4,8,16}`) |
| `phase-2-priority-only-unreserved` | `phase-2-congested-priority-only` (jobs `unreserved_x{4,8,16}`) |
| `phase-2-two-lane-both-dynamic` | `phase-2-congested-both-dynamic` (jobs `partitioned_x{4,16}`, `unreserved_x{4,16}`) |
| `phase-2-rb-scarcity` | `phase-2-congested-priority-only` (jobs `rb_reserved_x4` and `rb_reserved_x4_rb_{half,third,quarter}`) |
| `phase-2-urgency-inversion` | `phase-2-congested-both-dynamic` (job `partitioned_x4` at default demand, plus one `paper_like_mispriced` demand overlay — the only job in §4 that is not bit-identical to a §5 job) |

So the 7 goldens-pinned suites are essentially named, hash-pinned
re-runs of slices of the `paper_like_congested` matrix row. The
purpose of the duplication is to isolate one calibration question
per suite for reading and for golden-hash pinning, separate from the
large per-family suites.

## 6. The validity-tests table — robustness suites (6 validity suites + 1 smoke probe)

The robustness suites are not mechanism-characterisation experiments —
they are **validity probes** on top of the phase-2 sweep. Each
addresses a specific threat to the phase-2 findings (sample-size
budget, sign-flip cells where the welfare delta against the
accumulator-controller historical baseline changed sign under
Family B, canonical-variance verdicts at higher N, regime dependence
of the multiplier floor, pool-count generalisation, finite-run-length
boundary, block-build scan-order sensitivity).

| # | Suite | Question | Seeds | Demand | Jobs (authored) | What it varies | Paired control |
|---|---|---|---|---|---|---|---|
| 1 | `robustness-scoping` (TEST-02) | Per-(job, seed) wall-clock measurement so the BCa-bootstrap suites can pick N (from {10, 15, 18, 20}) subject to a ~30 min × parallelism total-compute budget per cell. Not a BCa CI itself. | 5 | `paper_like_congested` | 1 (job: `multiplier_x4`, un-reserved priority-only-static) | n/a (wall-clock only) | n/a |
| 2 | `robustness-canonical-variance` (TEST-04) | At N = 20 seeds, which of the four CIP menu options have a BCa 95% CI on `retained_value` delta that excludes zero against the single-lane EIP-1559 baseline? | 20 | `sundaeswap_moderate` | 5 (4 menu options + 1 control: `menu_{rb_reserved,unreserved}_{priority_only_static,both_dynamic}_x4` + `control_eip1559_d8_t50_w32`) | Mechanism arm (4 menu options) | `control_eip1559_d8_t50_w32` (the §3 baseline) |
| 3 | `robustness-sign-flip-variance` (TEST-03) | At N = 20 seeds under Family B, do the four "sign-flip" cells (jobs whose accumulator-vs-chain-derived welfare delta historically changed sign) produce statistically-significant welfare deltas against their respective single-lane EIP-1559 baselines? | 20 | `sundaeswap_moderate` | 6 (4 sign-flip cells + 2 baselines: one at default protocol, one on the RB-quarter overlay) | EIP-1559 step-size denominator `D` (cell 1: D=4); EIP-1559 target utilisation (cell 2: target=0.25); RB body cap (cells 3, 4: rb_quarter) | `control_eip1559_d8_t50_w32_base` (default protocol); `control_eip1559_d8_t50_w32_rb_quarter` (RB-quarter overlay) |
| 4 | `robustness-multiplier-floor-16-companion` (TEST-07a) | The `phase-2-rb-scarcity` and `phase-2-urgency-inversion` suites in §4 pin `multiplier_floor = 4`. Re-run those cells at floor = 16 (the spec default) — does the welfare finding hold or invert? | 5 | `paper_like_congested` (baseline + RB-overlay cells); `paper_like_mispriced` (the `urgency_inversion_x16_mispriced_high_urgency` cell only, via demand overlay) | 6 (4 RB-scarcity cells `rb_scarcity_x16_{baseline,rb_half,rb_third,rb_quarter}` + 2 urgency-inversion cells `urgency_inversion_x16_{correctly_priced,mispriced_high_urgency}`) | `multiplier_floor` (4 → 16) | Per-cell against the floor = 4 version from the corresponding source suite in §4 |
| 5 | `robustness-pool-number-sensitivity` (TEST-05) | Does the menu-option welfare ordering generalise from the 100-node `topology-realistic-100` to the 150-node `topology-realistic-150`? | 5 | `sundaeswap_moderate` | 33 | Topology pool count (100 → 150), with the full mechanism-family × calibration sweep at each pool count | Per-cell against the corresponding 100-pool phase-2 demand-regime suite |
| 6 | `robustness-run-length` (TEST-06) | Does the steady-state criterion hold for the four CIP menu arms across run lengths {2000, 4000, 8000} slots? | 10 | `sundaeswap_moderate` | 12 | Run length (2000 → 4000 → 8000 slots) across the four CIP menu arms | n/a (within-menu-arm steady-state criterion, not a paired bootstrap) |
| 7 | `robustness-fifo-smoke` | Quick diagnostic: under FIFO mempool scanning, do the four x4 two-lane menu arms remain comparable enough to justify a fuller sensitivity run? | 3 | `sundaeswap_moderate` | 6 (FIFO for the four menu arms + flat-fee baseline + one EIP-1559 control; priority-first counterparts already live in `robustness-canonical-variance`) | Block-build scan order fixed to `fifo` across all four two-lane menu arms | Descriptive comparison against both controls and against existing priority-first canonical outputs; no BCa claim at N = 3 |

## 7. What this design does NOT vary

The design holds these knobs at the values pinned in §2 across every
job. Each is disclosed as deferred work in
[`realism-risks-register.md`](realism-risks-register.md) under the
identifier listed. Re-calibrating any of them would invalidate the
golden hashes and require a re-pinning pass.

| Knob held fixed | Value | Reason it is not swept | Disclosure |
|---|---|---|---|
| Pool count | 100 (per §2). The 150-pool variant is authored in `robustness-pool-number-sensitivity` but not run across the full mechanism × calibration matrix. | `topology-realistic-100` is mainnet-shaped by mass-stratified downsample at epoch 582; pool counts above 150 (toward CIP-0164's 600-pool migration regime, or the present-day approximate-3000-pool mainnet) are out of scope for this evidence base. | `RSK-pool-count` |
| Stake snapshot | Epoch 582 (2026-05-14) | Cardano mainnet stake redistributes slowly; snapshot freshness is bounded by re-running the topology-generator at a later epoch via `sim-rs/scripts/generate-realistic-100-topology.py` | `RSK-calibration-stale-stake-snapshot` |
| Run length | 2000 slots (per §2). Longer runs (4000, 8000 slots) are authored in `robustness-run-length` for the four CIP menu arms. | The 2000-slot default gives ~100 controller-window updates (window-length 32 × ~20 s per-block-cadence ≈ 10 simulated minutes), comparable to the controller-drift timescale but not a directly-tested steady-state regime. | `RSK-steady-state-run-length` |
| Cross-architecture determinism | x86_64 / glibc only | The pricing kernel is integer / rational / 128-bit unsigned arithmetic and bit-stable in principle; the upstream non-pricing substrate retains floating-point and is not cross-arch hardened | `RSK-substrate-scope` (a); `RSK-cross-arch-determinism` |
| Mempool cap magnitude | `2 × eb_referenced_txs_max_size_bytes` (~133× mainnet today) | Future Cardano mempool conventions are unknown; the cap is a spec default but is not empirically calibrated against a deployed system | `RSK-mempool-cap-magnitude` |
| Lane-signal-source (un-reserved priority) | Option 1: `priority_paying_bytes / total_block_capacity` | The mechanism-design spec lists three options; the simulator picked one and did not sweep | `RSK-un-anchored-controller-knobs` |
| Standard-side signal source (both-dynamic) | Capacity-weighted `standard_paying_bytes` over EBs only (RBs do not fire a standard sample in RB-reserved variants) | Same as above | `RSK-un-anchored-controller-knobs` |
| Strict reserved-FIFO scanner (anti-standard cap) | Not implemented | The quick FIFO smoke suite disables priority sorting but does not implement the stricter reserved scanner that caps standard service while walking the mempool | `mechanism-design.md` §"Methodology: simulator approximations" residual gap |
| Demand mix calibration | Order-of-magnitude correct; not bit-calibrated against Q1 2026 mainnet | The `paper_like_*` profiles are stylised reference loads; `sundaeswap_moderate` is the empirically-anchored profile but is from January 2022 | `RSK-demand-mix-bit-calibration`; `RSK-demand-non-stationarity`; `RSK-sundaeswap-demand-staleness` |
| Actor model | Utility-maximising; no strategic-bidder / Maximum Extractable Value / bribery modelling | The mechanism-design framing is non-adversarial; adversarial regimes are out of phase-2 scope | `RSK-substrate-scope` (c) |
| `MaxFeePolicy` default | `ScaledOverLaneQuote{4, 1}` (4× quote-drift headroom) | This is an actor-side default, not a mechanism property; the `phase-2-urgency-inversion` suite is the one suite that does sweep it (correctly priced vs mis-priced) | `RSK-max-fee-policy-default` |

## 8. Coverage gaps

Calibration cells the design does not cover. Each is an observation
about what is not in the factorial, not a bug.

| Cell not covered | Why |
|---|---|
| Multiplier floor swept under more than one demand profile within a single mechanism arm | The 12 demand-regime suites cross mechanism × demand, but multiplier floor within a single (mechanism × demand) cell is constant |
| Window length swept under mechanism arms other than single-lane EIP-1559 | The smoothing suite varies window length only within single-lane EIP-1559; the two-lane controllers use window length 32 across all suites |
| RB body capacity swept under un-reserved priority-only or single-lane mechanisms | The `phase-2-rb-scarcity` suite sweeps RB body capacity only under RB-reserved priority-only (where the RB partition is the load-bearing mechanism); other mechanism arms are not in the RB-scarcity factorial |
| `MaxFeePolicy` swept across mechanism arms other than partitioned both-dynamic | The `phase-2-urgency-inversion` suite varies `MaxFeePolicy` only under partitioned both-dynamic (where the controller drifts enough to create eviction risk on mis-priced actors); other mechanism arms would not produce the urgency-inversion phenomenon at the same calibration |
| High seed count (N ≥ 20) for the phase-2 mechanism-characterisation suites | The robustness suites raise seeds to 20 only for the canonical-variance and sign-flip-variance probes; the phase-2 suites keep seeds = 3 (see `RSK-three-seed-statistical-power`) |

The robustness suites partially compensate for the seed-count
gap and (via TEST-07a, the multiplier-floor-16 companion) for the
multiplier-floor coverage at specific cells — but the compensation
is targeted, not a complete factorial.

## 9. References

- Mechanism specification: [`docs/phase-2/mechanism-design.md`](../../docs/phase-2/mechanism-design.md)
- Implementation plan: [`docs/phase-2/implementation-plan.md`](../../docs/phase-2/implementation-plan.md)
- Per-milestone handoffs: [`docs/phase-2/m1-handoff.md`](../../docs/phase-2/m1-handoff.md) through `m5-handoff.md`
- Methodology overview: [`methodology-overview.md`](methodology-overview.md)
- Calibration source-of-truth: [`cardano-realism-audit.md`](cardano-realism-audit.md)
- Realism risks register: [`realism-risks-register.md`](realism-risks-register.md)
- Family B controller decision: [`.planning/family-b-decision-2026-05-14.md`](../../.planning/family-b-decision-2026-05-14.md)
- Calibration fix post-mortem (RB cadence): [`docs/phase-2/calibration-fix-postmortem.md`](../../docs/phase-2/calibration-fix-postmortem.md)
- EB-sizing fix post-mortem: [`docs/phase-2/eb-sizing-fix-postmortem.md`](../../docs/phase-2/eb-sizing-fix-postmortem.md)
- Operator-facing repo orientation: [`CLAUDE.md`](../../CLAUDE.md) §"Calibration choices", §"Running the suites"
