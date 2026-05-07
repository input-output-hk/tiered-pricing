use std::f64::consts::TAU;
use std::path::Path;

use anyhow::{Context, Result};
use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::model::{ActorId, UrgencyProfile};
use crate::tx_pricing::OverflowRetryPolicy;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActorConfig {
    pub name: String,
    pub arrival_rate: f64,
    #[serde(default)]
    pub arrival_pattern: ArrivalPattern,
    pub tx_size: Distribution,
    pub value_distribution: Distribution,
    pub urgency: UrgencyProfile,
    #[serde(default)]
    pub urgency_u_distribution: Option<Distribution>,
    #[serde(default)]
    pub value_urgency_components: Vec<ValueUrgencyComponentConfig>,
    #[serde(default)]
    pub overflow_retry_policy_override: Option<OverflowRetryPolicy>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ArrivalPattern {
    #[serde(alias = "Constant")]
    Constant,
    #[serde(alias = "Bursty")]
    Bursty {
        burst_prob: f64,
        burst_multiplier: f64,
    },
    #[serde(alias = "Phased")]
    Phased { phases: Vec<ArrivalPhase> },
    #[serde(alias = "Scheduled")]
    Scheduled {
        slots: Vec<u64>,
        #[serde(default = "default_scheduled_count")]
        count_per_slot: u64,
    },
    #[serde(alias = "Reactive")]
    Reactive { trigger: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArrivalPhase {
    pub start_slot: u64,
    pub end_slot: Option<u64>,
    pub rate: f64,
}

impl Default for ArrivalPattern {
    fn default() -> Self {
        ArrivalPattern::Constant
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Distribution {
    pub kind: DistributionKind,
    #[serde(default)]
    pub params: Vec<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DistributionKind {
    #[serde(alias = "Constant")]
    Constant,
    #[serde(alias = "Uniform")]
    Uniform,
    #[serde(alias = "Normal")]
    Normal,
    #[serde(alias = "Exponential")]
    Exponential,
    #[serde(alias = "Empirical")]
    Empirical,
}

impl Distribution {
    pub fn sample_u64<R: Rng + ?Sized>(&self, rng: &mut R) -> u64 {
        let value = self.sample_f64(rng);
        if !value.is_finite() {
            return 0;
        }
        value.max(0.0).round() as u64
    }

    pub fn sample_f64<R: Rng + ?Sized>(&self, rng: &mut R) -> f64 {
        match self.kind {
            DistributionKind::Constant => self.params.get(0).copied().unwrap_or(0.0),
            DistributionKind::Uniform => {
                let min = self.params.get(0).copied().unwrap_or(0.0);
                let max = self.params.get(1).copied().unwrap_or(min);
                if max <= min {
                    return min;
                }
                rng.random_range(min..=max)
            }
            DistributionKind::Normal => {
                let mean = self.params.get(0).copied().unwrap_or(0.0);
                let stddev = self.params.get(1).copied().unwrap_or(0.0);
                if stddev <= 0.0 {
                    return mean;
                }
                let u1 = rng.random::<f64>().max(f64::MIN_POSITIVE);
                let u2 = rng.random::<f64>();
                let z0 = (-2.0 * u1.ln()).sqrt() * (TAU * u2).cos();
                mean + z0 * stddev
            }
            DistributionKind::Exponential => {
                let lambda = self.params.get(0).copied().unwrap_or(0.0);
                if lambda <= 0.0 {
                    return 0.0;
                }
                let u = rng.random::<f64>().max(f64::MIN_POSITIVE);
                -u.ln() / lambda
            }
            DistributionKind::Empirical => {
                if self.params.is_empty() {
                    return 0.0;
                }
                let index = rng.random_range(0..self.params.len());
                self.params[index]
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValueUrgencyComponentConfig {
    /// Optional human-readable name for this urgency class (e.g. "high_value_urgent").
    /// Used in per-urgency-class welfare metrics. Defaults to "component_N".
    #[serde(default)]
    pub name: Option<String>,
    pub weight: f64,
    pub value_distribution: Distribution,
    pub urgency_u_distribution: Distribution,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActorsFile {
    #[serde(default)]
    pub actors: Vec<ActorConfig>,
}

impl ActorsFile {
    pub fn from_path(path: &Path) -> Result<Self> {
        let contents = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read actors config {}", path.display()))?;
        let file: ActorsFile = toml::from_str(&contents)
            .with_context(|| format!("failed to parse actors config {}", path.display()))?;
        Ok(file)
    }
}

fn default_scheduled_count() -> u64 {
    1
}

#[derive(Debug, Clone)]
pub struct Actor {
    pub id: ActorId,
    pub name: String,
    pub arrival_rate: f64,
    pub arrival_pattern: ArrivalPattern,
    pub tx_size: Distribution,
    pub value_distribution: Distribution,
    pub urgency: UrgencyProfile,
    pub urgency_u_distribution: Option<Distribution>,
    pub value_urgency_components: Vec<ValueUrgencyComponent>,
    pub overflow_retry_policy_override: Option<OverflowRetryPolicy>,
}

#[derive(Debug, Clone)]
pub struct ValueUrgencyComponent {
    pub name: Option<String>,
    pub weight: f64,
    pub value_distribution: Distribution,
    pub urgency_u_distribution: Distribution,
}

impl Actor {
    pub fn from_config(id: ActorId, config: &ActorConfig) -> Self {
        Self {
            id,
            name: config.name.clone(),
            arrival_rate: config.arrival_rate,
            arrival_pattern: config.arrival_pattern.clone(),
            tx_size: config.tx_size.clone(),
            value_distribution: config.value_distribution.clone(),
            urgency: config.urgency.clone(),
            urgency_u_distribution: config.urgency_u_distribution.clone(),
            value_urgency_components: config
                .value_urgency_components
                .iter()
                .map(|component| ValueUrgencyComponent {
                    name: component.name.clone(),
                    weight: component.weight,
                    value_distribution: component.value_distribution.clone(),
                    urgency_u_distribution: component.urgency_u_distribution.clone(),
                })
                .collect(),
            overflow_retry_policy_override: config.overflow_retry_policy_override.clone(),
        }
    }
}

pub fn build_actors(configs: &[ActorConfig]) -> Vec<Actor> {
    configs
        .iter()
        .enumerate()
        .map(|(index, config)| Actor::from_config(ActorId::new(index as u64), config))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::ActorsFile;

    #[test]
    fn parses_actor_overflow_retry_policy_override() {
        let raw = r#"
[[actors]]
name = "a"
arrival_rate = 1.0
tx_size = { kind = "constant", params = [100] }
value_distribution = { kind = "constant", params = [1000] }
urgency = { kind = "indifferent" }

[actors.overflow_retry_policy_override]
enabled = true
source = "local_actor_submissions"
curve_metric = "retained_value_ratio"
backoff_mode = "exponential"
max_delay_slots = 32

[[actors.overflow_retry_policy_override.bands]]
min_retained_ratio = 0.0
max_retained_ratio = 1.0
max_attempts = 1
base_delay_slots = 2
"#;
        let parsed: ActorsFile = toml::from_str(raw).expect("actors config should parse");
        assert_eq!(parsed.actors.len(), 1);
        let override_policy = parsed.actors[0]
            .overflow_retry_policy_override
            .as_ref()
            .expect("override should be present");
        assert_eq!(override_policy.max_delay_slots, 32);
        assert_eq!(override_policy.bands.len(), 1);
    }
}
