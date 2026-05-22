#!/usr/bin/env python3
"""
Robustness analysis pass.

Reads per-(job, seed) run_summary.json from sim-rs/output/robustness/<suite>-<run-id>/
and emits per-cell .json artefacts + a results.md per test under
.planning/realism-tests/<test>/.

Algorithms:
  * paired_bca_ci   — paired-sample Bias-corrected and accelerated (BCa)
    bootstrap CI on the mean of `delta[i] = samples_a[i] - samples_b[i]`.
    Matches the Rust library in sim-cli/src/metrics/paired_bootstrap.rs
    (algorithm + numerics; the Python RNG namespace is distinct, so
    Python-emitted CIs are NOT bit-identical to Rust-emitted CIs — but
    both are deterministic given their respective bootstrap_seed).
  * paired_delta_summary — median, IQR, sign-coherence.
  * hash-diversity gate — distinct count of pricing_event_stream_sha256
    over the N seeds; required == N for BACKED.

Usage:
  python3 scripts/analyse-robustness.py <run-id>

Run from sim-rs/.
"""

import json
import math
import random
import statistics
import sys
from pathlib import Path

# ---- BCa implementation (Python port of sim-cli/src/metrics/paired_bootstrap.rs) ----

N_BOOTSTRAP = 9999
ALPHA = 0.05

def _inv_norm_cdf(p: float) -> float:
    """Inverse standard normal CDF (Beasley-Springer / Moro approximation)."""
    p = max(min(p, 1.0 - 1e-9), 1e-9)
    # Statistics module exposes NormalDist.inv_cdf in 3.8+.
    return statistics.NormalDist(0.0, 1.0).inv_cdf(p)

def _norm_cdf(x: float) -> float:
    return statistics.NormalDist(0.0, 1.0).cdf(x)

def _percentile(sorted_xs, q):
    n = len(sorted_xs)
    if n == 1:
        return sorted_xs[0]
    q = max(0.0, min(1.0, q))
    idx = q * (n - 1)
    lo = int(math.floor(idx))
    hi = int(math.ceil(idx))
    if lo == hi:
        return sorted_xs[lo]
    frac = idx - lo
    return sorted_xs[lo] * (1 - frac) + sorted_xs[hi] * frac

def paired_bca_ci(samples_a, samples_b, alpha=ALPHA, bootstrap_seed=0):
    assert len(samples_a) == len(samples_b), "paired samples must have equal lengths"
    assert len(samples_a) > 0
    deltas = [a - b for a, b in zip(samples_a, samples_b)]
    n = len(deltas)
    point = sum(deltas) / n
    rng = random.Random(bootstrap_seed)
    boot = []
    for _ in range(N_BOOTSTRAP):
        idxs = [rng.randrange(0, n) for _ in range(n)]
        boot.append(sum(deltas[i] for i in idxs) / n)
    boot.sort()
    prop_below = sum(1 for x in boot if x < point) / N_BOOTSTRAP
    z0 = _inv_norm_cdf(prop_below)
    # jackknife
    sum_d = sum(deltas)
    jack = [(sum_d - d) / (n - 1) for d in deltas] if n > 1 else [point]
    jm = sum(jack) / len(jack)
    num = sum((jm - x) ** 3 for x in jack)
    den = 6.0 * (sum((jm - x) ** 2 for x in jack)) ** 1.5
    a_hat = 0.0 if abs(den) < 1e-12 else num / den
    za_lo = _inv_norm_cdf(alpha / 2)
    za_hi = _inv_norm_cdf(1 - alpha / 2)
    q_lo = max(1e-9, min(1.0 - 1e-9, _norm_cdf(z0 + (z0 + za_lo) / (1 - a_hat * (z0 + za_lo)))))
    q_hi = max(1e-9, min(1.0 - 1e-9, _norm_cdf(z0 + (z0 + za_hi) / (1 - a_hat * (z0 + za_hi)))))
    return {
        "point": point,
        "lower": _percentile(boot, q_lo),
        "upper": _percentile(boot, q_hi),
        "alpha": alpha,
        "n_bootstrap": N_BOOTSTRAP,
        "bootstrap_seed": bootstrap_seed,
    }

def paired_delta_summary(samples_a, samples_b):
    assert len(samples_a) == len(samples_b)
    deltas = [a - b for a, b in zip(samples_a, samples_b)]
    s = sorted(deltas)
    median = _percentile(s, 0.5)
    iqr = _percentile(s, 0.75) - _percentile(s, 0.25)
    msign = (1 if median > 0 else (-1 if median < 0 else 0))
    agree = sum(1 for d in deltas if d == 0 or (1 if d > 0 else -1) == msign)
    return {
        "n": len(deltas),
        "median": median,
        "iqr": iqr,
        "sign_coherence": agree / len(deltas),
    }

# ---- data loading ----

def load_run_summary(p: Path):
    with open(p) as f:
        d = json.load(f)
    rv = d["priority_retained_value_total"] + d["standard_retained_value_total"]
    return {
        "retained_value": rv,
        "priority_retained_value_total": d["priority_retained_value_total"],
        "standard_retained_value_total": d["standard_retained_value_total"],
        "net_utility_total": d.get("net_utility_total"),
        "retained_value_ratio": d.get("retained_value_ratio"),
        "pricing_event_stream_sha256": d["pricing_event_stream_sha256"],
    }

def collect_job(suite_dir: Path, job_name: str, seeds):
    """Return dict: seed -> run_summary fields, in seed order. Missing seeds raise."""
    out = {}
    for s in seeds:
        p = suite_dir / job_name / str(s) / "run_summary.json"
        if not p.exists():
            return None
        out[s] = load_run_summary(p)
    return out

# ---- TEST-03 / TEST-04 paired bootstrap pipeline ----

def paired_bootstrap_test(suite_dir, cells, control, seeds, out_dir, bootstrap_seed_base):
    """cells: dict[name -> path/job]; control: job-name. Emits per-cell JSON
    and returns a list of (cell, verdict, ci, delta_summary, hash_distinct_count)."""
    out_dir.mkdir(parents=True, exist_ok=True)
    control_data = collect_job(suite_dir, control, seeds)
    if control_data is None:
        return [(c, "data-missing", None, None, None) for c in cells]
    control_rv = [control_data[s]["retained_value"] for s in seeds]
    control_shas = [control_data[s]["pricing_event_stream_sha256"] for s in seeds]
    rows = []
    for idx, (cell_label, cell_job) in enumerate(cells.items()):
        cell_data = collect_job(suite_dir, cell_job, seeds)
        if cell_data is None:
            rows.append((cell_label, "data-missing", None, None, None))
            continue
        cell_rv = [cell_data[s]["retained_value"] for s in seeds]
        cell_shas = [cell_data[s]["pricing_event_stream_sha256"] for s in seeds]
        bseed = bootstrap_seed_base + idx
        ci = paired_bca_ci(cell_rv, control_rv, ALPHA, bseed)
        ds = paired_delta_summary(cell_rv, control_rv)
        # Hash-diversity gate: both arms must have distinct count == N.
        cell_distinct = len(set(cell_shas))
        control_distinct = len(set(control_shas))
        ci_crosses_zero = ci["lower"] <= 0 <= ci["upper"]
        # Verdict per CONTEXT.md D-32: BACKED iff CI excludes zero AND hash gate passes.
        gate_passes = (cell_distinct == len(seeds)) and (control_distinct == len(seeds))
        if not gate_passes:
            verdict = "re-run-needed"
        elif ci_crosses_zero:
            verdict = "WEAK"
        else:
            verdict = "BACKED"
        artifact = {
            "cell": cell_label,
            "cell_job": cell_job,
            "control_job": control,
            "n_seeds": len(seeds),
            "seeds": list(seeds),
            "ci": ci,
            "delta_summary": ds,
            "verdict": verdict,
            "cell_retained_value": cell_rv,
            "control_retained_value": control_rv,
            "cell_pricing_event_stream_sha256s": cell_shas,
            "control_pricing_event_stream_sha256s": control_shas,
            "cell_distinct_hash_count": cell_distinct,
            "control_distinct_hash_count": control_distinct,
            "hash_diversity_gate_passes": gate_passes,
            "ci_crosses_zero": ci_crosses_zero,
        }
        with open(out_dir / f"{cell_label}.json", "w") as f:
            json.dump(artifact, f, indent=2)
        rows.append((cell_label, verdict, ci, ds, cell_distinct))
    return rows

def main():
    run_id = sys.argv[1] if len(sys.argv) > 1 else "20260518-084846"
    base = Path("output/robustness")
    realism = Path("../.planning/realism-tests")

    # TEST-03 sign-flip variance (6 jobs × 20 seeds)
    print(f"\n=== TEST-03 sign-flip variance (run-id {run_id}) ===")
    suite_dir = base / f"sign-flip-variance-{run_id}"
    seeds = list(range(1, 21))
    cells_eip1559 = {
        "cell_eip1559_d4_t50_w32": "cell_eip1559_d4_t50_w32",
        "cell_eip1559_d8_t25_w32": "cell_eip1559_d8_t25_w32",
    }
    cells_rb_quarter = {
        "cell_rb_reserved_x4_rb_quarter": "cell_rb_reserved_x4_rb_quarter",
        "cell_partitioned_x4_rb_quarter": "cell_partitioned_x4_rb_quarter",
    }
    out_dir = realism / "multi-seed-variance" / "sign-flip"
    rows_eip = paired_bootstrap_test(suite_dir, cells_eip1559,
                                      "control_eip1559_d8_t50_w32_base",
                                      seeds, out_dir, bootstrap_seed_base=1001)
    rows_quarter = paired_bootstrap_test(suite_dir, cells_rb_quarter,
                                          "control_eip1559_d8_t50_w32_rb_quarter",
                                          seeds, out_dir, bootstrap_seed_base=1003)
    test_03_rows = rows_eip + rows_quarter
    for r in test_03_rows:
        print(f"  {r[0]:<40} → {r[1]}  ci=[{r[2]['lower']:.2e}, {r[2]['upper']:.2e}]  median Δ={r[3]['median']:.2e}  sign-coh={r[3]['sign_coherence']:.2f}  distinct-hash={r[4]}/20" if r[2] else f"  {r[0]:<40} → {r[1]}")

    # TEST-04 canonical variance (5 jobs × 20 seeds: 4 menu + 1 control)
    print(f"\n=== TEST-04 canonical variance (run-id {run_id}) ===")
    suite_dir = base / f"canonical-variance-{run_id}"
    cells_menu = {
        "menu_rb_reserved_priority_only_static_x4": "menu_rb_reserved_priority_only_static_x4",
        "menu_unreserved_priority_only_static_x4": "menu_unreserved_priority_only_static_x4",
        "menu_rb_reserved_both_dynamic_x4": "menu_rb_reserved_both_dynamic_x4",
        "menu_unreserved_both_dynamic_x4": "menu_unreserved_both_dynamic_x4",
    }
    out_dir = realism / "multi-seed-variance" / "canonical"
    test_04_rows = paired_bootstrap_test(suite_dir, cells_menu,
                                         "control_eip1559_d8_t50_w32",
                                         seeds, out_dir, bootstrap_seed_base=2001)
    for r in test_04_rows:
        print(f"  {r[0]:<40} → {r[1]}  ci=[{r[2]['lower']:.2e}, {r[2]['upper']:.2e}]  median Δ={r[3]['median']:.2e}  sign-coh={r[3]['sign_coherence']:.2f}  distinct-hash={r[4]}/20" if r[2] else f"  {r[0]:<40} → {r[1]}")

    # TEST-07a multiplier-floor-16 companion (6 jobs × 5 seeds, qualitative)
    print(f"\n=== TEST-07a multiplier-floor-16 companion (run-id {run_id}) ===")
    suite_dir = base / f"multiplier-floor-16-companion-{run_id}"
    seeds_5 = list(range(1, 6))
    test_07a_jobs = [
        "rb_scarcity_x16_baseline",
        "rb_scarcity_x16_rb_half",
        "rb_scarcity_x16_rb_third",
        "rb_scarcity_x16_rb_quarter",
        "urgency_inversion_x16_correctly_priced",
        "urgency_inversion_x16_mispriced_high_urgency",
    ]
    out_dir = realism / "multiplier-floor-16-companion"
    out_dir.mkdir(parents=True, exist_ok=True)
    test_07a_rows = []
    for jname in test_07a_jobs:
        data = collect_job(suite_dir, jname, seeds_5)
        if data is None:
            test_07a_rows.append((jname, "data-missing", None, None, None))
            continue
        rvs = [data[s]["retained_value"] for s in seeds_5]
        prio_rvs = [data[s]["priority_retained_value_total"] for s in seeds_5]
        std_rvs = [data[s]["standard_retained_value_total"] for s in seeds_5]
        shas = [data[s]["pricing_event_stream_sha256"] for s in seeds_5]
        mean_rv = sum(rvs) / len(rvs)
        median_rv = statistics.median(rvs)
        sd_rv = statistics.stdev(rvs) if len(rvs) > 1 else 0.0
        # Sign-coherence on priority share = priority / total.
        shares = [p / (p + s) if (p + s) > 0 else 0 for p, s in zip(prio_rvs, std_rvs)]
        artifact = {
            "cell": jname,
            "n_seeds": len(seeds_5),
            "seeds": seeds_5,
            "retained_value": rvs,
            "priority_retained_value_total": prio_rvs,
            "standard_retained_value_total": std_rvs,
            "priority_share": shares,
            "mean_retained_value": mean_rv,
            "median_retained_value": median_rv,
            "stdev_retained_value": sd_rv,
            "pricing_event_stream_sha256s": shas,
            "distinct_hash_count": len(set(shas)),
        }
        with open(out_dir / f"{jname}.json", "w") as f:
            json.dump(artifact, f, indent=2)
        test_07a_rows.append((jname, "data-ready", mean_rv, sd_rv, len(set(shas))))
        print(f"  {jname:<48} mean_rv={mean_rv:.3e}  σ={sd_rv:.3e}  distinct-hash={len(set(shas))}/5")

    # TEST-05 partial check (informational only)
    print(f"\n=== TEST-05 pool-number sensitivity (run-id {run_id}) — PARTIAL ===")
    suite_dir = base / f"pool-number-sensitivity-{run_id}"
    if suite_dir.exists():
        manifest = json.load(open(suite_dir / "manifest.json"))
        complete = 0
        for jdata in manifest["jobs"].values():
            for sd in jdata.values():
                if sd["status"] == "completed":
                    complete += 1
        total = sum(len(j) for j in manifest["jobs"].values())
        print(f"  {complete}/{total} runs complete — insufficient for 33-job × 5-seed × 2-pool comparison")

    # TEST-06 partial check (informational only)
    print(f"\n=== TEST-06 run-length / steady-state (run-id {run_id}) — PARTIAL ===")
    suite_dir = base / f"run-length-{run_id}"
    if suite_dir.exists():
        manifest = json.load(open(suite_dir / "manifest.json"))
        for jname, jdata in manifest["jobs"].items():
            complete = sum(1 for s in jdata.values() if s["status"] == "completed")
            if complete > 0:
                print(f"  {jname}: {complete}/{len(jdata)} seeds done")

    return test_03_rows, test_04_rows, test_07a_rows

if __name__ == "__main__":
    main()
