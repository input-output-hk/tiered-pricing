//! Reference-integrity test for the live Phase 2 config tree.
//!
//! Each Phase 2 experiment YAML must:
//!   1. Parse as valid YAML.
//!   2. Have any `pricing.config-path` and `actors.config-path` targets exist on
//!      disk and parse as valid pricing/actor TOMLs.
//!
//! Each pricing / demand TOML under `phase-2-sweep/{pricing,demand}/` must parse.
//!
//! Each suite YAML under `phase-2-sweep/{suites,shards}/` must point at files that
//! exist: `defaults.topology`, `defaults.parameters[]`, `jobs[].parameters[]`, and
//! `jobs[].compare-parameters[]`.
//!
//! Acts as a guard against broken references after config pruning or reshuffles.
//! Does not attempt a full `SimConfiguration::build()` per experiment because some
//! Phase 2 experiments are thin pricing-only overlays designed to be combined with a
//! demand overlay from the surrounding suite.

use std::path::PathBuf;

use serde::Deserialize;
use sim_core::{tx_actors::ActorsFile, tx_pricing::PricingFile};

fn sim_rs_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sim-cli has a parent dir")
        .to_path_buf()
}

#[derive(Debug, Deserialize)]
struct ConfigPathRef {
    #[serde(rename = "config-path")]
    config_path: String,
}

#[derive(Debug, Deserialize)]
struct ExperimentHead {
    pricing: Option<ConfigPathRef>,
    actors: Option<ConfigPathRef>,
}

fn collect_yamls(dir: &std::path::Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir).expect("directory exists") {
        let path = entry.unwrap().path();
        if path.extension().map_or(false, |e| e == "yaml") {
            out.push(path);
        }
    }
    out.sort();
    out
}

fn collect_tomls(dir: &std::path::Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir).expect("directory exists") {
        let path = entry.unwrap().path();
        if path.extension().map_or(false, |e| e == "toml") {
            out.push(path);
        }
    }
    out.sort();
    out
}

#[test]
fn phase_2_sweep_experiment_config_paths_resolve() {
    let root = sim_rs_root();
    let experiments = collect_yamls(&root.join("parameters/phase-2-sweep/experiments"));
    assert!(
        !experiments.is_empty(),
        "expected at least one Phase 2 experiment"
    );

    for experiment in &experiments {
        let raw = std::fs::read_to_string(experiment)
            .unwrap_or_else(|e| panic!("read {}: {e}", experiment.display()));
        let head: ExperimentHead = serde_yaml::from_str(&raw)
            .unwrap_or_else(|e| panic!("parse {}: {e}", experiment.display()));

        if let Some(pricing) = head.pricing {
            let target = root.join(&pricing.config_path);
            assert!(
                target.exists(),
                "{} references missing pricing TOML: {}",
                experiment.display(),
                target.display(),
            );
            PricingFile::from_path(&target).unwrap_or_else(|e| {
                panic!(
                    "{} references pricing TOML that fails to parse: {} ({e})",
                    experiment.display(),
                    target.display(),
                )
            });
        }

        if let Some(actors) = head.actors {
            let target = root.join(&actors.config_path);
            assert!(
                target.exists(),
                "{} references missing actors TOML: {}",
                experiment.display(),
                target.display(),
            );
            ActorsFile::from_path(&target).unwrap_or_else(|e| {
                panic!(
                    "{} references actors TOML that fails to parse: {} ({e})",
                    experiment.display(),
                    target.display(),
                )
            });
        }
    }
}

#[test]
fn phase_2_sweep_pricing_tomls_parse() {
    let dir = sim_rs_root().join("parameters/phase-2-sweep/pricing");
    for path in collect_tomls(&dir) {
        PricingFile::from_path(&path)
            .unwrap_or_else(|e| panic!("failed to parse pricing {}: {e}", path.display()));
    }
}

#[test]
fn phase_2_sweep_demand_tomls_parse() {
    let dir = sim_rs_root().join("parameters/phase-2-sweep/demand");
    for path in collect_tomls(&dir) {
        ActorsFile::from_path(&path)
            .unwrap_or_else(|e| panic!("failed to parse actors {}: {e}", path.display()));
    }
}

#[derive(Debug, Deserialize)]
struct SuiteJob {
    #[serde(default)]
    parameters: Vec<String>,
    #[serde(rename = "compare-parameters", default)]
    compare_parameters: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct SuiteDefaults {
    #[serde(default)]
    topology: Option<String>,
    #[serde(default)]
    parameters: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct SuiteFile {
    #[serde(default)]
    defaults: Option<SuiteDefaults>,
    #[serde(default)]
    jobs: Vec<SuiteJob>,
}

fn assert_suite_paths_resolve(suite_path: &std::path::Path, root: &std::path::Path) {
    let raw = std::fs::read_to_string(suite_path)
        .unwrap_or_else(|e| panic!("read {}: {e}", suite_path.display()));
    let suite: SuiteFile = serde_yaml::from_str(&raw)
        .unwrap_or_else(|e| panic!("parse {}: {e}", suite_path.display()));

    let mut targets: Vec<String> = Vec::new();
    if let Some(defaults) = suite.defaults {
        if let Some(topology) = defaults.topology {
            targets.push(topology);
        }
        targets.extend(defaults.parameters);
    }
    for job in suite.jobs {
        targets.extend(job.parameters);
        targets.extend(job.compare_parameters);
    }

    for rel in targets {
        let target = root.join(&rel);
        assert!(
            target.exists(),
            "{} references missing file: {}",
            suite_path.display(),
            target.display(),
        );
    }
}

#[test]
fn phase_2_sweep_suite_paths_resolve() {
    let root = sim_rs_root();
    let mut suites: Vec<PathBuf> = Vec::new();
    for dir in [
        root.join("parameters/phase-2-sweep/suites"),
        root.join("parameters/phase-2-sweep/shards"),
    ] {
        if dir.exists() {
            suites.extend(collect_yamls(&dir));
        }
    }
    // Top-level Phase 2 suite files that live directly under phase-2-sweep/
    // (siblings of protocol-base.yaml).
    for top_level in ["parameters/phase-2-sweep/phase-2-stage-a.yaml"] {
        let path = root.join(top_level);
        assert!(
            path.exists(),
            "top-level suite file missing: {}",
            path.display()
        );
        suites.push(path);
    }
    assert!(!suites.is_empty(), "expected at least one Phase 2 suite");
    for suite in suites {
        assert_suite_paths_resolve(&suite, &root);
    }
}
