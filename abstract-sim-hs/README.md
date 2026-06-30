# abstract-sim-hs

## Experiment sweeps

Run the workload embedded in each variant config. The existing mechanism
configs use `severe-congestion`:

```console
stack run -- sweep config/sweeps/mechanisms.json
```

Alternatively, select a load-profile file at invocation time. This applies the
same workload to every variant without editing those variant configs.

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

The selected profile is copied into the output directory and recorded in
`summary.json`. Load-profile files are ordinary JSON envelopes containing a
name, an optional description, and a `load` value in the same format accepted
by a simulation config.
