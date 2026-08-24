//! Invariantes clínicos — GCP, ICH-E6, 21 CFR Part 11

use serde::{Deserialize, Serialize};

use crate::error::ClinicalError;
use crate::trial::{ClinicalTrial, Patient};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InvariantStatus {
    Pass,
    Fail,
    Pending,
    NotApplicable,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClinicalInvariant {
    pub id: String,
    pub name: String,
    pub description: String,
    pub severity: InvariantSeverity,
    pub check_fn: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InvariantSeverity {
    Critical,
    High,
    Medium,
    Low,
}

pub struct InvariantEngine {
    _invariants: Vec<ClinicalInvariant>,
}

impl InvariantEngine {
    pub fn new() -> Self {
        Self {
            _invariants: Self::default_invariants(),
        }
    }

    pub fn default_invariants() -> Vec<ClinicalInvariant> {
        vec![
            ClinicalInvariant {
                id: "CLN-001".to_string(),
                name: "Audit Trail Completo".to_string(),
                description: "Todas as interações com dados de ensaio clínico devem ser registradas".to_string(),
                severity: InvariantSeverity::Critical,
                check_fn: "check_audit_trail".to_string(),
            },
            ClinicalInvariant {
                id: "CLN-002".to_string(),
                name: "Consentimento Informado".to_string(),
                description: "Todo paciente deve ter consentimento informado assinado".to_string(),
                severity: InvariantSeverity::Critical,
                check_fn: "check_consent".to_string(),
            },
            ClinicalInvariant {
                id: "CLN-003".to_string(),
                name: "Data Lock Irreversível".to_string(),
                description: "Uma vez que o data lock é aplicado, nenhuma modificação é permitida".to_string(),
                severity: InvariantSeverity::Critical,
                check_fn: "check_data_lock".to_string(),
            },
            ClinicalInvariant {
                id: "CLN-004".to_string(),
                name: "Randomização Estratificada".to_string(),
                description: "A randomização deve ser estratificada por fatores prognósticos".to_string(),
                severity: InvariantSeverity::High,
                check_fn: "check_randomization".to_string(),
            },
            ClinicalInvariant {
                id: "CLN-005".to_string(),
                name: "SAEs Reportados".to_string(),
                description: "SAEs devem ser reportados em até 24h no formato CIOMS I".to_string(),
                severity: InvariantSeverity::High,
                check_fn: "check_sae_reporting".to_string(),
            },
            ClinicalInvariant {
                id: "CLN-006".to_string(),
                name: "Pseudonimização".to_string(),
                description: "Dados de pacientes devem ser pseudonimizados".to_string(),
                severity: InvariantSeverity::High,
                check_fn: "check_pseudonym".to_string(),
            },
            ClinicalInvariant {
                id: "CLN-007".to_string(),
                name: "RT-QuIC como Inclusão".to_string(),
                description: "RT-QuIC positivo é critério mandatório para inclusão".to_string(),
                severity: InvariantSeverity::Critical,
                check_fn: "check_rt_quic".to_string(),
            },
            ClinicalInvariant {
                id: "CLN-008".to_string(),
                name: "Critérios de Parada".to_string(),
                description: "Trial deve ser interrompido se P(efficacy) < 0.05 ou > 0.95".to_string(),
                severity: InvariantSeverity::Critical,
                check_fn: "check_stopping_rule".to_string(),
            },
            ClinicalInvariant {
                id: "CLN-009".to_string(),
                name: "Retenção de Dados".to_string(),
                description: "Dados devem ser retidos por mínimo 15 anos".to_string(),
                severity: InvariantSeverity::High,
                check_fn: "check_retention".to_string(),
            },
            ClinicalInvariant {
                id: "CLN-010".to_string(),
                name: "Anonimato na Exportação".to_string(),
                description: "Exportações devem ser anônimas".to_string(),
                severity: InvariantSeverity::High,
                check_fn: "check_export".to_string(),
            },
        ]
    }

    pub async fn check_protocol(&self, trial: &ClinicalTrial) -> Result<(), ClinicalError> {
        if trial.target_enrollment == 0 {
            return Err(ClinicalError::ProtocolDeviation(
                "Target enrollment must be > 0".to_string()
            ));
        }
        if trial.eligibility_criteria.inclusion.is_empty() {
            return Err(ClinicalError::ProtocolDeviation(
                "Inclusion criteria required".to_string()
            ));
        }
        Ok(())
    }

    pub async fn check_eligibility(&self, patient: &Patient, trial: &ClinicalTrial) -> Result<(), ClinicalError> {
        let criteria = &trial.eligibility_criteria;

        if let Some(min_age) = criteria.min_age {
            if patient.demographic.age_at_enrollment < min_age {
                return Err(ClinicalError::ProtocolDeviation(
                    format!("Patient under minimum age: {}", min_age)
                ));
            }
        }

        if let Some(req_diagnosis) = &criteria.required_diagnosis {
            if !patient.clinical_data.diagnosis.contains(req_diagnosis) {
                return Err(ClinicalError::ProtocolDeviation(
                    format!("Patient diagnosis does not match: {}", req_diagnosis)
                ));
            }
        }

        Ok(())
    }
}

impl Default for InvariantEngine {
    fn default() -> Self {
        Self::new()
    }
}
