//! Suite YAML schema. M3.
//!
//! A `Suite` declares a `suite_name`, an `output_dir`, a list of
//! `seeds`, and a list of `jobs`. Each job binds a pricing TOML to
//! the suite's shared protocol/topology/demand defaults; per-job
//! `overrides` can override `slots`, `seeds`, or any other top-level
//! field individually. The runner expands the (job × seed) cartesian
//! product into independent simulator runs.

use std::path::PathBuf;

use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Suite {
    pub suite_name: String,
    pub output_dir: PathBuf,
    pub seeds: Vec<u64>,
    pub default_slots: u64,
    pub default_topology: PathBuf,
    pub default_protocol: PathBuf,
    pub default_demand: PathBuf,
    pub jobs: Vec<Job>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Job {
    pub name: String,
    pub pricing: PathBuf,
    #[serde(default)]
    pub overrides: JobOverrides,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct JobOverrides {
    #[serde(default)]
    pub slots: Option<u64>,
    #[serde(default)]
    pub seeds: Option<Vec<u64>>,
    #[serde(default)]
    pub demand: Option<PathBuf>,
    #[serde(default)]
    pub topology: Option<PathBuf>,
    #[serde(default)]
    pub protocol: Option<PathBuf>,
}

impl Suite {
    pub fn load(path: &std::path::Path) -> Result<Self> {
        let text = std::fs::read_to_string(path)?;
        Ok(serde_yaml::from_str(&text)?)
    }

    /// (job × seed) pairs in the order they should be executed.
    pub fn job_seed_pairs(&self) -> Vec<(usize, u64)> {
        let mut out = Vec::new();
        for (idx, job) in self.jobs.iter().enumerate() {
            let seeds = job.overrides.seeds.as_ref().unwrap_or(&self.seeds);
            for s in seeds {
                out.push((idx, *s));
            }
        }
        out
    }
}
