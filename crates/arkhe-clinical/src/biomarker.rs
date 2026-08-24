//! Análise de biomarcadores RT-QuIC
//!
//! RT-QuIC (Real-Time Quaking-Induced Conversion) é o padrão ouro para diagnóstico de DCJ
//! Sensibilidade: 92-97% | Especificidade: 100% (S-level sources)

use crate::AuditTimestamp;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use statrs::statistics::Statistics;
use statrs::distribution::Beta;
use statrs::statistics::Distribution;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RtQuicResult {
    pub sample_id: String,
    pub patient_id: String,
    pub collection_date: AuditTimestamp,
    pub analysis_date: AuditTimestamp,
    pub seed_positive: bool,
    pub lag_time_minutes: f64,
    pub max_fluorescence: f64,
    pub slope: f64,
    pub endpoint_titer: f64,
    pub replicates_positive: u8,
    pub replicates_total: u8,
    pub quality: QualityFlag,
    pub notes: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QualityFlag {
    Valid,
    Borderline,
    Invalid,
    Contaminated,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RtQuicAnalysis {
    pub trial_id: String,
    pub total_samples: usize,
    pub positive_samples: usize,
    pub negative_samples: usize,
    pub sensitivity_estimate: f64,
    pub specificity_estimate: f64,
    pub mean_lag_time: f64,
    pub mean_max_fluorescence: f64,
    pub positivity_rate: f64,
    pub confidence_interval: (f64, f64),
    pub timestamp: AuditTimestamp,
}

impl RtQuicAnalysis {
    pub fn from_results(trial_id: &str, results: &[RtQuicResult]) -> Self {
        let total_samples = results.len();
        let positive_samples = results.iter().filter(|r| r.seed_positive).count();
        let negative_samples = total_samples - positive_samples;

        let lag_times: Vec<f64> = results.iter().map(|r| r.lag_time_minutes).collect();
        let max_fluorescences: Vec<f64> = results.iter().map(|r| r.max_fluorescence).collect();

        let alpha = 1.0 + positive_samples as f64;
        let beta = 1.0 + negative_samples as f64;
        let beta_dist = Beta::new(alpha, beta).unwrap();
        let sensitivity_estimate = beta_dist.mean().unwrap_or(0.0);
        let (lower, upper) = if total_samples > 0 {
            let se = (sensitivity_estimate * (1.0 - sensitivity_estimate) / total_samples as f64).sqrt();
            let lower = (sensitivity_estimate - 1.96 * se).max(0.0);
            let upper = (sensitivity_estimate + 1.96 * se).min(1.0);
            (lower, upper)
        } else {
            (0.0, 0.0)
        };

        let mean_lag_time = if lag_times.is_empty() { 0.0 } else { lag_times.mean() };
        let mean_max_fluorescence = if max_fluorescences.is_empty() { 0.0 } else { max_fluorescences.mean() };

        Self {
            trial_id: trial_id.to_string(),
            total_samples,
            positive_samples,
            negative_samples,
            sensitivity_estimate,
            specificity_estimate: 1.0,
            mean_lag_time,
            mean_max_fluorescence,
            positivity_rate: if total_samples > 0 { positive_samples as f64 / total_samples as f64 } else { 0.0 },
            confidence_interval: (lower, upper),
            timestamp: Utc::now(),
        }
    }

    pub fn is_within_expected_range(&self) -> bool {
        self.sensitivity_estimate >= 0.92 && self.sensitivity_estimate <= 0.97
    }

    pub fn positive_predictive_value(&self, prevalence: f64) -> f64 {
        let se = self.sensitivity_estimate;
        let sp = self.specificity_estimate;
        (se * prevalence) / (se * prevalence + (1.0 - sp) * (1.0 - prevalence))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomarkerEvent {
    pub patient_id: String,
    pub timestamp: AuditTimestamp,
    pub biomarker_type: BiomarkerType,
    pub value: f64,
    pub unit: String,
    pub flags: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BiomarkerType {
    RtQuic,
    Ttau,
    Ptau,
    Nfl,
    Prp,
    Mri,
}
