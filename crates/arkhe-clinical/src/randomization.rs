//! Randomização Estratificada — ICH E6(R2) §4.9
//!
//! Implementa randomização estratificada com minimização para ensaios
//! em doenças raras (DCJ), onde o tamanho amostral é pequeno.

use crate::PrnpGenotype;
use rand::{Rng, SeedableRng};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StratumFactor {
    PrnpGenotype(PrnpGenotype),
    AgeGroup(AgeGroup),
    MmseStage(MmseStage),
    Site,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AgeGroup {
    Under60,
    Over60,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MmseStage {
    Mild,     // MMSE ≥ 20
    Moderate, // MMSE 10-19
    Severe,   // MMSE < 10
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stratum {
    pub factors: Vec<StratumFactor>,
    pub arm_allocations: HashMap<String, u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RandomizationResult {
    pub subject_code: String,
    pub assigned_arm: String,
    pub stratum_key: String,
    pub method: RandomizationMethod,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub seed: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RandomizationMethod {
    Simple,
    Block,
    Minimization,
    Stratified,
}

pub struct StratifiedRandomizer {
    arms: Vec<String>,
    allocation_ratio: Vec<f64>,
    strata: HashMap<String, Stratum>,
    history: Vec<RandomizationResult>,
    seed: u64,
}

impl StratifiedRandomizer {
    pub fn new(arms: Vec<String>, allocation_ratio: Vec<f64>, seed: u64) -> Result<Self, RandomizationError> {
        if arms.len() != allocation_ratio.len() {
            return Err(RandomizationError::MismatchedArms);
        }
        let sum: f64 = allocation_ratio.iter().sum();
        if (sum - 1.0).abs() > 1e-6 {
            return Err(RandomizationError::InvalidRatio);
        }

        Ok(Self {
            arms,
            allocation_ratio,
            strata: HashMap::new(),
            history: Vec::new(),
            seed,
        })
    }

    pub fn classify_stratum(
        &self,
        genotype: PrnpGenotype,
        age: u8,
        mmse: u8,
        site: &str,
    ) -> String {
        let age_group = if age < 60 { AgeGroup::Under60 } else { AgeGroup::Over60 };
        let mmse_stage = match mmse {
            0..=9 => MmseStage::Severe,
            10..=19 => MmseStage::Moderate,
            _ => MmseStage::Mild,
        };

        format!(
            "{}_{:?}_{:?}_{}",
            Self::genotype_str(genotype),
            age_group,
            mmse_stage,
            site
        )
    }

    pub fn randomize(
        &mut self,
        subject_code: &str,
        genotype: PrnpGenotype,
        age: u8,
        mmse: u8,
        site: &str,
    ) -> Result<RandomizationResult, RandomizationError> {
        let stratum_key = self.classify_stratum(genotype, age, mmse, site);

        let factors = vec![
                StratumFactor::PrnpGenotype(genotype),
                StratumFactor::AgeGroup(if age < 60 { AgeGroup::Under60 } else { AgeGroup::Over60 }),
                StratumFactor::MmseStage(match mmse {
                    0..=9 => MmseStage::Severe,
                    10..=19 => MmseStage::Moderate,
                    _ => MmseStage::Mild,
                }),
                StratumFactor::Site,
        ];

        let stratum = self.strata.entry(stratum_key.clone()).or_insert(Stratum {
            factors,
            arm_allocations: HashMap::new(),
        });

        let mut best_arm = self.arms[0].clone();
        let mut best_imbalance = f64::MAX;

        for (i, arm) in self.arms.iter().enumerate() {
            let current_count = *stratum.arm_allocations.get(arm).unwrap_or(&0) as f64;
            let expected = self.allocation_ratio[i] * stratum.arm_allocations.values().sum::<u32>() as f64;
            let imbalance = (current_count + 1.0 - expected).abs();

            if imbalance < best_imbalance {
                best_imbalance = imbalance;
                best_arm = arm.clone();
            }
        }

        let mut rng = rand::rngs::StdRng::seed_from_u64(self.seed + self.history.len() as u64);
        if rng.gen::<f64>() < 0.2 {
            let idx = rng.gen_range(0..self.arms.len());
            best_arm = self.arms[idx].clone();
        }

        *stratum.arm_allocations.entry(best_arm.clone()).or_insert(0) += 1;

        let result = RandomizationResult {
            subject_code: subject_code.to_string(),
            assigned_arm: best_arm,
            stratum_key,
            method: RandomizationMethod::Minimization,
            timestamp: chrono::Utc::now(),
            seed: self.seed,
        };

        self.history.push(result.clone());
        Ok(result)
    }

    pub fn check_balance(&self, max_ratio: f64) -> Vec<BalanceViolation> {
        let mut violations = Vec::new();

        for (key, stratum) in &self.strata {
            let counts: Vec<u32> = stratum.arm_allocations.values().copied().collect();
            if counts.len() < 2 {
                continue;
            }
            let max_count = counts.iter().max().copied().unwrap_or(1);
            let min_count = counts.iter().min().copied().unwrap_or(1);

            if max_count as f64 / min_count as f64 > max_ratio {
                violations.push(BalanceViolation {
                    stratum_key: key.clone(),
                    max_count,
                    min_count,
                    ratio: max_count as f64 / min_count as f64,
                });
            }
        }

        violations
    }

    pub fn history(&self) -> &[RandomizationResult] {
        &self.history
    }

    fn genotype_str(g: PrnpGenotype) -> &'static str {
        match g {
            PrnpGenotype::MM129 => "MM",
            PrnpGenotype::MV129 => "MV",
            PrnpGenotype::VV129 => "VV",
            PrnpGenotype::E200K => "E200K",
            PrnpGenotype::V210I => "V210I",
            PrnpGenotype::Other => "OTHER",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceViolation {
    pub stratum_key: String,
    pub max_count: u32,
    pub min_count: u32,
    pub ratio: f64,
}

#[derive(Debug, thiserror::Error)]
pub enum RandomizationError {
    #[error("Número de braços e proporções não correspondem")]
    MismatchedArms,
    #[error("Proporções de alocação não somam 1.0")]
    InvalidRatio,
    #[error("Estrato não encontrado")]
    StratumNotFound,
}
