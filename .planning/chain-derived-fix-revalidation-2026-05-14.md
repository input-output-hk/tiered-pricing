# Chain-derived one-step-lag fix — revalidation note

Date: 2026-05-14
Fix applied: `current_chain_tip_quote` in
`sim-rs/sim-core/src/sim/linear_leios.rs` now calls
`compute_chain_derived_quote_for_child_of(tip)` instead of returning
`rb.derived_quote` directly. See investigation:
`.planning/chain-derived-bug-investigation.md`.

## TL;DR

**The fix is correct and a real improvement — but it is NOT the
complete explanation of the chain-derived divergence.** A second
divergence source remains. STOP and surface per binding constraint.

- Unit-test goldens: all 130 unit tests pass; the new regression test
  `admission_uses_post_step_quote_at_chain_tip` fails on the pre-fix
  implementation and passes on the post-fix implementation.
- M5 suite goldens: 4 of 7 flipped under the fix; all 7 reproduce
  cleanly after regen.
- Smoke batch (`sundaeswap-batch-20260514-134450`, 33 jobs at seed=1):
  zero-SB job `eip1559_d4_t50_w32` trajectory now **byte-identical to
  the accumulator for slots 0-75**, then diverges again from slot 100
  onward. End-of-run c_priority remains 12,639 vs accumulator 130,553
  (off by 10.3×). Welfare sign-flip remains (net_utility = -2.32e+10).

## Reference case `sundaeswap-singlelane / eip1559_d4_t50_w32` (zero SB)

End-of-run (slot 1999):

| Run            | c_priority | retained_value | fees_paid   | net_utility |
|----------------|-----------:|---------------:|------------:|------------:|
| accumulator    |    130,553 |     +4.651e+10 |   2.617e+10 |  +2.034e+10 |
| prior-buggy CD |     12,412 |     +1.627e+10 |   3.022e+10 |  -1.395e+10 |
| **new-fixed CD** | **12,639** | **+2.967e+10** | **5.288e+10** | **-2.321e+10** |

Trajectory (every few slots, c_priority):

```
slot   acc   prior   new     comment
  0     44     44     44    cold start (all match)
 25     55     44     55    new == acc (fix lands first step)
 50     69     55     69    new == acc (fix sustains)
 75     69     55     69    new == acc
100    109     69     87    *** new starts diverging from acc ***
200    172    109    137    new still lags acc by ~one step
500  19826   3076   3842    different by 5×
1000 12720  250741 457423   new running massively hot vs acc
1500  2751 2639813 3044813  new still running hot
1999 130553  12412  12639   end-of-run: new ~= prior, both far below acc
```

The one-step-lag was real: slots 25, 50, 75 now match accumulator
exactly under the fix (vs prior 44, 55, 55). But starting at slot ~100
a second divergence appears, and by mid-run the new-fixed trajectory
is closer to the prior-buggy trajectory than to accumulator.

## Aggregate convergence stats (33 jobs at seed=1)

|                 | ±1% | ±5% | ±25% | over 25% |
|-----------------|----:|----:|-----:|---------:|
| All 33 jobs     |   1 |   0 |    2 |       30 |
| Zero-SB (3)     |   1 (baseline_flat_fee) |   0 |    0 |        2 (eip1559_d4_t50_w32, rb_reserved_x4 / partitioned_x4) |
| Pos-SB  (30)    |   0 |   0 |    2 |       28 |

`baseline_flat_fee` continues to match byte-for-byte (no controller
state). Among the three zero-SB jobs, only the flat-fee control
matches; both controller-bearing zero-SB jobs (`eip1559_d4_t50_w32`
and `rb_reserved_x4` / `partitioned_x4` which is the same scenario
rendered twice across families) still diverge by ~810% and ~90%
respectively from accumulator.

## Conclusion

**The chain-derived implementation still has at least one other
divergence source beyond the now-fixed one-step lag.** The fix is
necessary but not sufficient. The early-slot match (slots 25-75)
confirms the lag mechanism was real and the patch closes it; the
mid-run divergence (slot 100+) confirms a second mechanism is in
play.

Likely candidates for the second mechanism (HIGH-level hypotheses,
not yet investigated — surfaced for follow-up):

- **Sample emission ordering through EB-deferred path** — H3 from the
  original investigation was ruled out for the d4 reference case
  because it uses a single-producer topology with synchronous
  endorsement, BUT the smoke uses
  `topology-sundaeswap-realistic.yaml` which is multi-node, where
  deferred-EB validation can fire and the cache-overwrite path
  introduces real path dependence.
- **Window aggregate / sample cache interaction across the EB
  validation race window** — see same comment.
- **A second timing asymmetry** in lane-choice vs charging that the
  current fix didn't address.

## WR-1 status

WR-1 (orphan-block-contamination claim for the accumulator
implementation) **cannot be classified as empirically RESOLVED** based
on this smoke. The fix removed the one-step lag (a separate bug, not
WR-1), but the zero-SB job that should have been the cleanest WR-1
contamination removal test still shows large residual divergence
attributable to other unknowns. Per investigation §"What would shake
confidence": this is exactly the scenario where the math-divergence
concern (and the second-mechanism concern) needs follow-up.

**Recommendation:** do NOT mark WR-1 resolved in REVIEW.md until the
second divergence source is localized and either:
- Fixed (and zero-SB jobs converge to accumulator within ±1%), or
- Shown to be an unavoidable consequence of the chain-derived
  architecture and orthogonal to the accumulator's WR-1 contamination
  claim (in which case WR-1 itself is still a real concern in the
  accumulator implementation, but the smoke comparison cannot prove
  it).

## What the fix does buy

Even without WR-1 resolution, the fix is correct and worth keeping:
- Restores legacy `pricing.current_quote()` semantics at admission,
  lane choice, EB endorsement validation, and EB inclusion charging.
- Removes the admission-vs-charging asymmetry that was a real bug.
- Early-slot trajectory now matches accumulator byte-for-byte on the
  d4 reference case, proving the patch closes the one-step lag.
- Regression test `admission_uses_post_step_quote_at_chain_tip` will
  catch any future regression of this specific bug.
