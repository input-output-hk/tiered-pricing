#!/usr/bin/env python3
"""Evaluate the held-out confirmation of the cert-void pilot.

Two co-primary paired contrasts, both on launch-day overall retained value:
the cert-gated sample-and-hold standard signal, and the third-of-an-EB
no-cert window contribution, each against the recommended W20/W5 signals.
Each uses a two-sided 97.5% Student-t interval (Bonferroni across the two
endpoints) and must agree with the expected direction in at least nine of
ten paired seeds. Both must pass. Everything else is descriptive.

The script refuses to run unless the pre-registered chain is intact: the
pilot report and source patch match their predeclared hashes, the manifest
carries the expected analysis plan, the pre-run hash ledger matches the
files used for this evaluation, and the simulator binary is the pilot's.

Usage:
  python3 scripts/compare_cert_void_confirm.py \
    --root sweep-results/cert-void-confirm \
    --simulator-sha256 HEX64 \
    --markdown-output sweep-results/cert-void-confirm/comparison.md \
    --json-output sweep-results/cert-void-confirm/comparison.json
"""

import argparse
import hashlib
import json
import math
from pathlib import Path
from typing import Any

PROJECT_DIR = Path(__file__).resolve().parent.parent

PILOT_REPORT = "sweep-results/cert-void-smoke/comparison.json"
PILOT_REPORT_SHA256 = "254f59113bea4166087cdc11afa7f60a8c9f8ab949088f737af3e9c34726750e"
PILOT_SOURCE_PATCH = "sweep-results/cert-void-smoke/source.patch"
PILOT_SOURCE_PATCH_SHA256 = "132cdb7a7d7d6cd336b540b7283758064b4500fce5565dbce1e074d95f98bf10"

BASE = "current-w20-w5"
CONTRASTS = (
    ("cert-only-minus-current", "Cert-gated hold − current", "standard-cert-only-w5"),
    ("void-third-minus-current", "Void third-EB − current", "standard-void-third-w5"),
)
LOADS = ["low", "mid-load", "severe-congestion", "eb-capacity-stress", "launch-day"]
METRICS = [
    ("value.retainedLovelace", 1e-9, "overall retained", "G lovelace"),
    ("value.retainedRatio", 1, "retained ratio", "ratio"),
    ("value.urgent.retainedLovelace", 1e-9, "urgent retained", "G lovelace"),
    ("throughput.txPerSlot", 1, "throughput", "tx/slot"),
    ("inclusion.standard.serviceRate", 1, "standard service rate", "ratio"),
    ("inclusion.urgent.serviceRate", 1, "urgent service rate", "ratio"),
    ("latency.standard.meanBlocks", 1, "standard mean wait", "blocks"),
    ("latency.urgent.meanBlocks", 1, "urgent mean wait", "blocks"),
    ("price.settledCoefficientRange", 1, "settled coeff range", "coeff"),
    ("price.oscillationExcessTravel", 1, "excess quote travel", "log-coeff"),
]

CANONICAL_SEEDS = list(range(420, 430))
CANONICAL_SLOTS = 2000
REQUIRED_DIRECTION_COUNT = 9

# The design fixes n = 10 paired seeds, so df = 9 throughout.
T_9875_DF9 = 2.685010847  # two-sided 97.5% (Bonferroni across two endpoints)
T_975_DF9 = 2.262157163  # two-sided 95% (descriptive secondaries)

EXPECTED_ANALYSIS_PLAN = {
    "stage": "held-out-confirmation",
    "pilot": {
        "report": PILOT_REPORT,
        "reportSha256": PILOT_REPORT_SHA256,
        "sourcePatchSha256": PILOT_SOURCE_PATCH_SHA256,
        "seeds": [300, 301, 302, 303, 304],
        "excludedFromInference": True,
    },
    "primaryProcedure": (
        "Two co-primary paired contrasts use two-sided 97.5% Student-t confidence "
        "intervals (Bonferroni-adjusted across the two endpoints). Each endpoint "
        "must also agree in the expected direction in at least 9 of 10 paired "
        "seeds. Both endpoints must pass."
    ),
    "primaryEndpoints": [
        {
            "id": "launch-cert-only-retained",
            "load": "launch-day",
            "contrast": "cert-only-minus-current",
            "metric": "value.retainedLovelace",
            "expectedDirection": "negative",
        },
        {
            "id": "launch-void-third-retained",
            "load": "launch-day",
            "contrast": "void-third-minus-current",
            "metric": "value.retainedLovelace",
            "expectedDirection": "negative",
        },
    ],
    "secondaryProcedure": (
        "All other load, metric, and contrast results are descriptive coherence "
        "and safety checks with ordinary two-sided 95% paired-t confidence "
        "intervals. They are not additional confirmatory tests."
    ),
    "scope": (
        "Passing supports two claims about the standard controller under this "
        "simulator calibration: updating only on certified-EB applications "
        "underprices sustained ramping demand, and a no-cert window contribution "
        "as large as a third of an empty EB floor-pins the quote under the same "
        "demand. Together with the descriptive dial sweeps it supports keeping "
        "the Ranking Block's own capacity as the no-cert contribution. It does "
        "not establish welfare, real-world optimality, or that the RB-sized "
        "contribution is globally optimal; the sub-RB region moved outcomes by "
        "less than half a percent in the exploratory pilot and is not tested "
        "here."
    ),
}

EXPECTED_VARIANTS = [
    {"name": BASE, "config": "config/variants/standard-target-screen/s75-u50.json"},
    {
        "name": "standard-cert-only-w5",
        "config": "config/variants/window-ablation/standard-cert-only-w5.json",
    },
    {
        "name": "standard-void-third-w5",
        "config": "config/variants/window-ablation/standard-void-third-w5.json",
    },
]


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def fail(message: str) -> None:
    raise SystemExit(f"error: {message}")


def validate_ledger(root: Path) -> dict[str, str]:
    ledger = root / "analysis-plan.sha256"
    if not ledger.exists():
        fail(f"pre-run analysis-plan hash ledger is missing: {ledger}")
    recorded: dict[str, str] = {}
    for line in ledger.read_text(encoding="utf-8").splitlines():
        pieces = line.split(maxsplit=1)
        if len(pieces) != 2:
            fail(f"malformed line in pre-run hash ledger: {line!r}")
        digest, name = pieces
        if name in recorded:
            fail(f"duplicate path in pre-run hash ledger: {name}")
        recorded[name] = digest
    for name, digest in recorded.items():
        candidate = Path(name)
        if not candidate.is_absolute():
            candidate = PROJECT_DIR / name
        if not candidate.exists():
            fail(f"ledger names a missing file: {name}")
        if sha256_file(candidate) != digest:
            fail(
                f"{name} changed after the pre-run ledger was written; "
                "the pre-registered plan is not intact"
            )
    for required in (str((root / "manifest.json").relative_to(PROJECT_DIR)),
                     "scripts/compare_cert_void_confirm.py",
                     "scripts/run_cert_void_confirm.sh"):
        if required not in recorded:
            fail(f"pre-run ledger does not cover {required}")
    return recorded


def validate_manifest(root: Path) -> None:
    manifest = json.loads((root / "manifest.json").read_text(encoding="utf-8"))
    if manifest.get("analysisPlan") != EXPECTED_ANALYSIS_PLAN:
        fail("manifest analysisPlan differs from the plan this evaluator pins")
    for key, expected in (
        ("seedStart", CANONICAL_SEEDS[0]),
        ("seeds", len(CANONICAL_SEEDS)),
        ("slots", CANONICAL_SLOTS),
        ("summaryOnly", True),
        ("randomness", "independent-streams"),
        ("variants", EXPECTED_VARIANTS),
    ):
        if manifest.get(key) != expected:
            fail(f"manifest {key} is {manifest.get(key)!r}, expected {expected!r}")


def validate_pilot(simulator_sha256: str) -> None:
    report = PROJECT_DIR / PILOT_REPORT
    patch = PROJECT_DIR / PILOT_SOURCE_PATCH
    if sha256_file(report) != PILOT_REPORT_SHA256:
        fail("pilot report hash differs from the predeclared artifact")
    if sha256_file(patch) != PILOT_SOURCE_PATCH_SHA256:
        fail("pilot source patch hash differs from the predeclared artifact")
    pilot = json.loads(report.read_text(encoding="utf-8"))
    pilot_simulator = pilot["provenance"]["simulator_sha256"]
    if pilot_simulator != simulator_sha256:
        fail(
            "simulator binary differs from the pilot: "
            f"pilot={pilot_simulator}, confirmation={simulator_sha256}; "
            "rerun the pilot with this binary before treating a run as its "
            "confirmation"
        )


def load_runs(root: Path) -> dict[str, dict[str, dict[int, dict[str, float]]]]:
    runs_by_load: dict[str, dict[str, dict[int, dict[str, float]]]] = {}
    for load in LOADS:
        data = json.loads((root / load / "summary.json").read_text(encoding="utf-8"))
        arms = {
            variant["name"]: {run["seed"]: run["scalars"] for run in variant["runs"]}
            for variant in data["variants"]
        }
        expected_names = {variant["name"] for variant in EXPECTED_VARIANTS}
        if set(arms) != expected_names:
            fail(f"{load}: arms {sorted(arms)} differ from {sorted(expected_names)}")
        for name, by_seed in arms.items():
            if sorted(by_seed) != CANONICAL_SEEDS:
                fail(f"{load}/{name}: seeds {sorted(by_seed)} are not {CANONICAL_SEEDS}")
        runs_by_load[load] = arms
    return runs_by_load


def paired_interval(differences: list[float], critical: float) -> tuple[float, float, float]:
    estimate = sum(differences) / len(differences)
    variance = sum((value - estimate) ** 2 for value in differences) / (
        len(differences) - 1
    )
    margin = critical * math.sqrt(variance / len(differences))
    return estimate, estimate - margin, estimate + margin


def exact_two_sided_sign_p(positive: int, negative: int) -> float:
    non_ties = positive + negative
    if non_ties == 0:
        return 1.0
    extreme = max(positive, negative)
    tail = sum(math.comb(non_ties, count) for count in range(extreme, non_ties + 1))
    return min(1.0, 2.0 * tail / (2**non_ties))


def differences_for(
    runs: dict[str, dict[int, dict[str, float]]],
    candidate: str,
    key: str,
    scale: float,
) -> list[float]:
    return [
        scale
        * (
            (runs[candidate][seed].get(key, 0) or 0)
            - (runs[BASE][seed].get(key, 0) or 0)
        )
        for seed in CANONICAL_SEEDS
    ]


def primary_result(
    endpoint: dict[str, str],
    runs_by_load: dict[str, dict[str, dict[int, dict[str, float]]]],
) -> dict[str, Any]:
    candidate = next(
        cand for cid, _label, cand in CONTRASTS if cid == endpoint["contrast"]
    )
    key, scale, label, unit = next(m for m in METRICS if m[0] == endpoint["metric"])
    differences = differences_for(runs_by_load[endpoint["load"]], candidate, key, scale)
    estimate, low, high = paired_interval(differences, T_9875_DF9)
    positive = sum(value > 0 for value in differences)
    negative = sum(value < 0 for value in differences)
    expected_negative = endpoint["expectedDirection"] == "negative"
    expected_count = negative if expected_negative else positive
    interval_pass = high < 0 if expected_negative else low > 0
    direction_pass = expected_count >= REQUIRED_DIRECTION_COUNT
    return {
        **endpoint,
        "label": f"{label} under {endpoint['load']}",
        "unit": unit,
        "candidate": candidate,
        "reference": BASE,
        "mean": estimate,
        "ci97_5_low": low,
        "ci97_5_high": high,
        "per_seed": dict(zip(map(str, CANONICAL_SEEDS), differences)),
        "candidate_higher": positive,
        "reference_higher": negative,
        "ties": len(differences) - positive - negative,
        "expected_direction_count": expected_count,
        "required_direction_count": REQUIRED_DIRECTION_COUNT,
        "exact_two_sided_sign_p": exact_two_sided_sign_p(positive, negative),
        "adjusted_interval_pass": interval_pass,
        "direction_count_pass": direction_pass,
        "status": "pass" if interval_pass and direction_pass else "not-confirmed",
    }


def secondary_tables(
    runs_by_load: dict[str, dict[str, dict[int, dict[str, float]]]],
) -> dict[str, Any]:
    tables: dict[str, Any] = {}
    for load in LOADS:
        table: dict[str, Any] = {}
        for contrast_id, _label, candidate in CONTRASTS:
            metrics: dict[str, Any] = {}
            for key, scale, label, unit in METRICS:
                differences = differences_for(runs_by_load[load], candidate, key, scale)
                estimate, low, high = paired_interval(differences, T_975_DF9)
                metrics[key] = {
                    "label": label,
                    "unit": unit,
                    "mean": estimate,
                    "ci95_low": low,
                    "ci95_high": high,
                    "candidate_higher": sum(value > 0 for value in differences),
                    "reference_higher": sum(value < 0 for value in differences),
                    "ties": sum(value == 0 for value in differences),
                }
            table[contrast_id] = metrics
        tables[load] = table
    return tables


def render_markdown(report: dict[str, Any]) -> str:
    lines = [
        "# Cert-void held-out confirmation",
        "",
        "Two co-primary paired contrasts on launch-day overall retained value, "
        "each against the recommended W20/W5 signals, on fresh seeds "
        f"{CANONICAL_SEEDS[0]}-{CANONICAL_SEEDS[-1]}. Each uses a two-sided 97.5% "
        "paired-t interval (Bonferroni across the two endpoints) and must agree "
        "in the expected direction in at least 9 of 10 seeds. Both must pass.",
        "",
        f"**Overall verdict: {report['verdict'].upper()}**",
        "",
        "| endpoint | mean (97.5% CI) | expected-direction seeds | sign p | result |",
        "|---|---|---|---|---|",
    ]
    for primary in report["primary_endpoints"]:
        lines.append(
            f"| {primary['label']}, {primary['contrast']} "
            f"| {primary['mean']:+.3f} {primary['unit']} "
            f"[{primary['ci97_5_low']:+.3f}, {primary['ci97_5_high']:+.3f}] "
            f"| {primary['expected_direction_count']}/10 "
            f"| {primary['exact_two_sided_sign_p']:.4g} "
            f"| {primary['status']} |"
        )
    lines.append("")
    lines.append("## Descriptive secondaries (95% paired-t; not confirmatory)")
    for load in LOADS:
        lines.append("")
        lines.append(f"### {load}")
        lines.append("")
        lines.append(
            "| metric | unit | "
            + " | ".join(label for _cid, label, _cand in CONTRASTS)
            + " |"
        )
        lines.append("|---|---|" + "---|" * len(CONTRASTS))
        for key, _scale, label, unit in METRICS:
            cells = []
            for contrast_id, _label, _cand in CONTRASTS:
                entry = report["secondary"][load][contrast_id][key]
                cells.append(
                    f"{entry['mean']:+.4f} "
                    f"[{entry['ci95_low']:+.4f}, {entry['ci95_high']:+.4f}] "
                    f"(+{entry['candidate_higher']}/-{entry['reference_higher']})"
                )
            lines.append(f"| {label} | {unit} | " + " | ".join(cells) + " |")
    lines.append("")
    lines.append(f"Evidence scope: {EXPECTED_ANALYSIS_PLAN['scope']}")
    lines.append("")
    lines.append(
        f"Simulator SHA-256 `{report['provenance']['simulator_sha256']}`; "
        f"pilot report SHA-256 `{PILOT_REPORT_SHA256}`."
    )
    return "\n".join(lines) + "\n"


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", required=True, type=Path)
    parser.add_argument("--simulator-sha256", required=True)
    parser.add_argument("--markdown-output", required=True, type=Path)
    parser.add_argument("--json-output", required=True, type=Path)
    args = parser.parse_args()
    simulator_sha256 = args.simulator_sha256.lower()
    if len(simulator_sha256) != 64 or any(
        character not in "0123456789abcdef" for character in simulator_sha256
    ):
        fail("--simulator-sha256 must be 64 hexadecimal characters")
    root = args.root if args.root.is_absolute() else PROJECT_DIR / args.root

    ledger = validate_ledger(root)
    validate_manifest(root)
    validate_pilot(simulator_sha256)
    runs_by_load = load_runs(root)

    primaries = [
        primary_result(endpoint, runs_by_load)
        for endpoint in EXPECTED_ANALYSIS_PLAN["primaryEndpoints"]
    ]
    verdict = (
        "pass"
        if all(primary["status"] == "pass" for primary in primaries)
        else "not-confirmed"
    )
    report = {
        "schema_version": 1,
        "experiment": "cert-void-confirm",
        "analysis_plan": EXPECTED_ANALYSIS_PLAN,
        "seeds": CANONICAL_SEEDS,
        "slots": CANONICAL_SLOTS,
        "difference_direction": "alternative signal minus recommended W20/W5",
        "verdict": verdict,
        "primary_endpoints": primaries,
        "secondary": secondary_tables(runs_by_load),
        "provenance": {
            "simulator_sha256": simulator_sha256,
            "pilot_report_sha256": PILOT_REPORT_SHA256,
            "pilot_source_patch_sha256": PILOT_SOURCE_PATCH_SHA256,
            "pre_run_ledger": ledger,
            "inputs": {
                load: sha256_file(root / load / "summary.json") for load in LOADS
            },
        },
    }
    args.json_output.write_text(
        json.dumps(report, indent=2) + "\n", encoding="utf-8"
    )
    args.markdown_output.write_text(render_markdown(report), encoding="utf-8")
    print(f"verdict: {verdict}")
    for primary in primaries:
        print(
            f"  {primary['id']}: {primary['status']} "
            f"(mean {primary['mean']:+.3f} {primary['unit']}, "
            f"97.5% CI [{primary['ci97_5_low']:+.3f}, {primary['ci97_5_high']:+.3f}], "
            f"direction {primary['expected_direction_count']}/10)"
        )
    print(f"wrote {args.json_output}")
    print(f"wrote {args.markdown_output}")


if __name__ == "__main__":
    main()
