# abstract-sim-hs

## Canonical phase-2 recommendation

The canonical simulator configuration for the phase-2 recommendation is
[`config/variants/trickle-aging/thr-k10.json`](config/variants/trickle-aging/thr-k10.json).
It combines the D16 controllers, 20/5 signal windows, half-RB announcement
threshold, K = 10 age escape, rb-only premium scope, absolute coefficient
floor 1, and no cross-lane multiplier floor. The max-of-two fee-cap rule is
the simulator's rb-only fee semantics rather than a configuration switch.
Its embedded load is a default for direct runs; sweep load overrides do not
change the recommended mechanism.

Run the bounded post-correction launch-day integration check with:

```console
./scripts/smoke_canonical_final.sh
```

This runs ten 2,000-slot seeds without event traces and writes a paired
comparison against the preserved corrected D16/no-K10 reference and its
pre-correction counterpart. The completed launch-day run was exactly equal
to both references across all 550 scalar comparisons (55 per seed); its compact
[evidence record](../docs/phase-2/experiment-results/canonical-final-smoke.json)
preserves the result outside the ignored sweep output.

## Experiment sweeps

Run the workload embedded in each variant config. The existing mechanism
configs use `severe-congestion`:

```console
stack run -- sweep config/sweeps/mechanisms.json
```

Use `--load PRESET` for a curated workload such as `low` or
`severe-congestion`:

```console
stack run -- sweep config/sweeps/mechanisms.json \
  --load low \
  --out sweep-results/mechanisms-low
```

Alternatively, select a load-profile file at invocation time. This applies the
same explicit workload to every variant without editing the source configs.

The sustained severe-congestion profile uses 40 tx/slot outside a 160 tx/slot
interval spanning slots 250–1749:

```console
stack run -- sweep config/sweeps/mechanisms.json \
  --load-profile config/loads/severe-congestion.json \
  --out sweep-results/mechanisms-severe-congestion
```

The EB-capacity stress profile alternates 20 tx/slot recovery intervals with
320 and 400 tx/slot overload intervals, bounded by 40 tx/slot warm-up and
cool-down periods:

```console
stack run -- sweep config/sweeps/mechanisms.json \
  --load-profile config/loads/eb-capacity-stress.json \
  --out sweep-results/mechanisms-eb-capacity-stress
```

Every output directory contains each variant's effective config, including the
selected workload. A selected profile file is also copied into the output and
recorded in `summary.json`. Load-profile files are ordinary JSON envelopes
containing a name, an optional description, and a `load` value in the same
format accepted by a simulation config.

Add `--summary-only` when only the per-seed and aggregate comparison metrics
are needed. The simulation folds the same events into the same metrics but
does not serialise the per-run `events.jsonl` files, which keeps bounded
comparison sweeps to a few kilobytes instead of gigabytes.
