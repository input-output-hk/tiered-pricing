//! Paired-sample Bias-corrected and accelerated (BCa) bootstrap confidence
//! intervals on the mean of paired deltas `delta[i] = samples_a[i] - samples_b[i]`.
//!
//! Phase-3 multi-seed evidence layer. Pure post-processing on `RunSummary`
//! reporting scalars; this module does not feed back into simulation, so it
//! cannot perturb M2/M3/M5 goldens (CLAUDE.md §"Numeric representation contract").
//!
//! References: DiCiccio & Efron, "Bootstrap confidence intervals" (Statist.
//! Sci. 1996); Hesterberg, "What Teachers Should Know About the Bootstrap"
//! (American Statistician 2015). Resampling RNG: `rand::rngs::StdRng`
//! (ChaCha-based, value-stable within `rand` 0.9.x — CLAUDE.md §"Determinism
//! scope"). The bootstrap-seed namespace is disjoint from simulator seeds
//! (CONTEXT.md D-23).

use rand::Rng;
use rand::SeedableRng;
use rand::rngs::StdRng;
use serde::{Deserialize, Serialize};
use statrs::distribution::{ContinuousCDF, Normal};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CiResult {
    pub point: f64,
    pub lower: f64,
    pub upper: f64,
    pub alpha: f64,
    pub n_bootstrap: u32,
    pub bootstrap_seed: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DeltaSummary {
    pub n: usize,
    pub median: f64,
    pub iqr: f64,
    pub sign_coherence: f64,
}

pub const N_BOOTSTRAP: u32 = 9999;

fn check_paired(a: &[f64], b: &[f64]) {
    assert_eq!(
        a.len(),
        b.len(),
        "paired samples must have equal lengths: a={}, b={}",
        a.len(),
        b.len()
    );
    assert!(!a.is_empty(), "paired samples must be non-empty");
    for x in a.iter().chain(b.iter()) {
        assert!(x.is_finite(), "paired samples must be finite (got {})", x);
    }
}

fn deltas(a: &[f64], b: &[f64]) -> Vec<f64> {
    a.iter().zip(b.iter()).map(|(x, y)| x - y).collect()
}

fn mean(xs: &[f64]) -> f64 {
    xs.iter().sum::<f64>() / xs.len() as f64
}

/// Linear-interpolation percentile over an ascending-sorted slice; `q` in `[0, 1]`.
fn percentile(sorted: &[f64], q: f64) -> f64 {
    let n = sorted.len();
    if n == 1 {
        return sorted[0];
    }
    let idx = q.clamp(0.0, 1.0) * (n - 1) as f64;
    let lo = idx.floor() as usize;
    let hi = idx.ceil() as usize;
    if lo == hi {
        sorted[lo]
    } else {
        sorted[lo] * (1.0 - (idx - lo as f64)) + sorted[hi] * (idx - lo as f64)
    }
}

/// Paired-sample BCa bootstrap CI on the mean of `samples_a[i] - samples_b[i]`.
/// `alpha` = two-sided significance (0.05 → 95% CI). Deterministic given seed.
pub fn paired_bca_ci(
    samples_a: &[f64],
    samples_b: &[f64],
    alpha: f64,
    bootstrap_seed: u64,
) -> CiResult {
    check_paired(samples_a, samples_b);
    assert!(
        alpha > 0.0 && alpha < 1.0,
        "alpha must be in (0.0, 1.0); got {}",
        alpha
    );

    let d = deltas(samples_a, samples_b);
    let n = d.len();
    let point = mean(&d);

    let mut rng = StdRng::seed_from_u64(bootstrap_seed);
    let mut boot: Vec<f64> = (0..N_BOOTSTRAP)
        .map(|_| {
            let mut sum = 0.0;
            for _ in 0..n {
                sum += d[rng.random_range(0..n)];
            }
            sum / n as f64
        })
        .collect();
    boot.sort_by(|x, y| x.partial_cmp(y).unwrap());

    let normal = Normal::new(0.0, 1.0).unwrap();
    let prop_below = boot.iter().filter(|&&x| x < point).count() as f64 / N_BOOTSTRAP as f64;
    let z0 = normal.inverse_cdf(prop_below.clamp(1e-9, 1.0 - 1e-9));

    // Acceleration a-hat via jackknife on the paired-delta mean.
    let sum_d: f64 = d.iter().sum();
    let denom_n = (n - 1).max(1) as f64;
    let jack: Vec<f64> = (0..n).map(|i| (sum_d - d[i]) / denom_n).collect();
    let jm = mean(&jack);
    let num: f64 = jack.iter().map(|x| (jm - x).powi(3)).sum();
    let den = 6.0 * jack.iter().map(|x| (jm - x).powi(2)).sum::<f64>().powf(1.5);
    let a_hat = if den.abs() < 1e-12 { 0.0 } else { num / den };

    let za_lo = normal.inverse_cdf(alpha / 2.0);
    let za_hi = normal.inverse_cdf(1.0 - alpha / 2.0);
    let q_lo = normal
        .cdf(z0 + (z0 + za_lo) / (1.0 - a_hat * (z0 + za_lo)))
        .clamp(1e-9, 1.0 - 1e-9);
    let q_hi = normal
        .cdf(z0 + (z0 + za_hi) / (1.0 - a_hat * (z0 + za_hi)))
        .clamp(1e-9, 1.0 - 1e-9);

    CiResult {
        point,
        lower: percentile(&boot, q_lo),
        upper: percentile(&boot, q_hi),
        alpha,
        n_bootstrap: N_BOOTSTRAP,
        bootstrap_seed,
    }
}

/// Median, Inter-Quartile Range (IQR), and sign-coherence of paired deltas.
/// Sign-coherence = fraction whose sign agrees with the median's; `0.0`
/// deltas are treated as agreeing (stability convention).
pub fn paired_delta_summary(samples_a: &[f64], samples_b: &[f64]) -> DeltaSummary {
    check_paired(samples_a, samples_b);
    let d = deltas(samples_a, samples_b);
    let mut sorted = d.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let median = percentile(&sorted, 0.5);
    let iqr = percentile(&sorted, 0.75) - percentile(&sorted, 0.25);
    let median_sign = median.signum();
    let agreeing = d
        .iter()
        .filter(|&&x| x == 0.0 || x.signum() == median_sign)
        .count();
    DeltaSummary {
        n: d.len(),
        median,
        iqr,
        sign_coherence: agreeing as f64 / d.len() as f64,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Box-Muller standard-normal sample (avoids a `rand_distr` dep).
    fn standard_normal(rng: &mut StdRng) -> f64 {
        let u1: f64 = rng.random::<f64>().max(1e-300);
        let u2: f64 = rng.random::<f64>();
        (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
    }

    #[test]
    fn paired_bca_ci_is_deterministic_given_seed() {
        let a: Vec<f64> = (1..=30).map(|x| x as f64 + 0.5).collect();
        let b: Vec<f64> = (1..=30).map(|x| x as f64).collect();
        assert_eq!(
            paired_bca_ci(&a, &b, 0.05, 42),
            paired_bca_ci(&a, &b, 0.05, 42)
        );
    }

    #[test]
    fn paired_bca_ci_recovers_known_mean_on_paired_gaussian() {
        // a[i] = N(1, 1), b[i] = N(0, 1) independent → paired delta ~ N(1, 2).
        let mut rng = StdRng::seed_from_u64(7);
        let n = 200;
        let a: Vec<f64> = (0..n).map(|_| standard_normal(&mut rng) + 1.0).collect();
        let b: Vec<f64> = (0..n).map(|_| standard_normal(&mut rng)).collect();
        let r = paired_bca_ci(&a, &b, 0.05, 99);
        // Tolerance: at n=200, SE(mean delta) ≈ sqrt(2/200) ≈ 0.1; point within 0.2 is generous.
        assert!(r.lower < 1.0 && r.upper > 1.0, "95% CI [{}, {}] must straddle true mean 1.0", r.lower, r.upper);
        assert!((r.point - 1.0).abs() < 0.2, "point estimate {} must be within 0.2 of 1.0", r.point);
    }

    #[test]
    #[should_panic(expected = "paired samples must have equal lengths")]
    fn paired_bca_ci_panics_on_length_mismatch() {
        let _ = paired_bca_ci(&[1.0, 2.0, 3.0, 4.0, 5.0], &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0], 0.05, 1);
    }

    #[test]
    #[should_panic(expected = "paired samples must be non-empty")]
    fn paired_bca_ci_panics_on_empty_input() {
        let _ = paired_bca_ci(&[], &[], 0.05, 1);
    }

    #[test]
    #[should_panic(expected = "paired samples must be finite")]
    fn paired_bca_ci_panics_on_non_finite() {
        let _ = paired_bca_ci(&[1.0, 2.0, f64::NAN], &[1.0, 2.0, 3.0], 0.05, 1);
    }

    #[test]
    fn paired_delta_summary_sign_coherence_full_agreement() {
        let s = paired_delta_summary(&[10.0, 11.0, 12.0, 13.0], &[1.0, 2.0, 3.0, 4.0]);
        assert_eq!(s.sign_coherence, 1.0);
        assert_eq!(s.n, 4);
    }

    #[test]
    fn paired_delta_summary_iqr_uniform() {
        // deltas = [1..9]; q25=3.0, q75=7.0, iqr=4.0, median=5.0.
        let a: Vec<f64> = (1..=9).map(|x| x as f64).collect();
        let s = paired_delta_summary(&a, &vec![0.0; 9]);
        assert!((s.median - 5.0).abs() < 1e-9);
        assert!((s.iqr - 4.0).abs() < 1e-9);
    }

    #[test]
    fn ci_result_serializes_to_json() {
        let r = CiResult { point: 1.0, lower: 0.5, upper: 1.5, alpha: 0.05, n_bootstrap: N_BOOTSTRAP, bootstrap_seed: 42 };
        let json = serde_json::to_string(&r).unwrap();
        assert!(json.contains("\"bootstrap_seed\":42"));
        assert!(json.contains("\"alpha\":0.05"));
    }
}
