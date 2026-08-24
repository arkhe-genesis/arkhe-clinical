//! Gestão de ensaios clínicos e pacientes

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::{
    AuditTimestamp,
    audit::{AuditTrail, AuditAction, AuditSeverity},
    biomarker::{RtQuicResult, BiomarkerEvent},
    error::ClinicalError,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrialStatus {
    Draft,
    Active,
    Recruiting,
    FollowUp,
    DataAnalysis,
    Completed,
    Terminated,
    Paused,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PatientStatus {
    Screened,
    Enrolled,
    Active,
    Completed,
    Withdrawn,
    LostToFollowUp,
    Deceased,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClinicalTrial {
    pub id: String,
    pub name: String,
    pub phase: Phase,
    pub intervention: Intervention,
    pub status: TrialStatus,
    pub target_enrollment: usize,
    pub enrolled_patients: Vec<String>,
    pub start_date: AuditTimestamp,
    pub end_date: Option<AuditTimestamp>,
    pub eligibility_criteria: EligibilityCriteria,
    pub endpoints: Vec<Endpoint>,
    pub dsmb_members: Vec<String>,
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Phase {
    Phase1,
    Phase2,
    Phase3,
    Phase4,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Intervention {
    Drug { name: String, dose: String, route: String },
    MonoclonalAntibody { name: String, dose: String, target: String },
    GeneTherapy { vector: String, target_gene: String },
    Placebo,
    Other { description: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EligibilityCriteria {
    pub inclusion: Vec<String>,
    pub exclusion: Vec<String>,
    pub min_age: Option<u8>,
    pub max_age: Option<u8>,
    pub required_diagnosis: Option<String>,
    pub prnp_codon_129: Option<String>,
    pub min_mmse: Option<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Endpoint {
    pub id: String,
    pub name: String,
    pub endpoint_type: EndpointType,
    pub timepoint_days: u32,
    pub analysis_method: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EndpointType {
    Primary,
    Secondary,
    Exploratory,
    Safety,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Patient {
    pub id: String,
    pub trial_id: String,
    pub status: PatientStatus,
    pub enrollment_date: AuditTimestamp,
    pub withdrawal_date: Option<AuditTimestamp>,
    pub demographic: Demographic,
    pub clinical_data: ClinicalData,
    pub biomarkers: Vec<BiomarkerEvent>,
    pub rt_quic_results: Vec<RtQuicResult>,
    pub adverse_events: Vec<AdverseEvent>,
    pub treatment_arm: Option<String>,
    pub randomization_date: Option<AuditTimestamp>,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Demographic {
    pub age_at_enrollment: u8,
    pub gender: Gender,
    pub ethnicity: Option<String>,
    pub region: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Gender {
    Male,
    Female,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClinicalData {
    pub diagnosis: String,
    pub diagnosis_date: AuditTimestamp,
    pub mmse_score: Option<u8>,
    pub cdr_plus_score: Option<f64>,
    pub functional_staging: Option<u8>,
    pub prnp_mutation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdverseEvent {
    pub id: String,
    pub patient_id: String,
    pub event_type: AdverseEventType,
    pub severity: AdverseSeverity,
    pub description: String,
    pub start_date: AuditTimestamp,
    pub end_date: Option<AuditTimestamp>,
    pub related_to_intervention: bool,
    pub sae: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AdverseEventType {
    Neurological,
    Psychiatric,
    Cardiovascular,
    Gastrointestinal,
    Infection,
    LabAbnormality,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AdverseSeverity {
    Mild,
    Moderate,
    Severe,
    LifeThreatening,
    Fatal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrialStats {
    pub trial_id: String,
    pub enrolled: usize,
    pub completed: usize,
    pub withdrawn: usize,
    pub deceased: usize,
    pub completion_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TrialEvent {
    Created,
    Started,
    Paused,
    Completed,
}

pub struct TrialManager {
    trials: Arc<RwLock<HashMap<String, ClinicalTrial>>>,
    audit: Arc<AuditTrail>,
}

impl TrialManager {
    pub fn new(audit: Arc<AuditTrail>) -> Self {
        Self {
            trials: Arc::new(RwLock::new(HashMap::new())),
            audit,
        }
    }

    pub async fn create_trial(&self, trial: ClinicalTrial, user_id: &str, user_role: &str) -> Result<(), ClinicalError> {
        let mut trials = self.trials.write().await;
        if trials.contains_key(&trial.id) {
            return Err(ClinicalError::InvalidData("Trial already exists".to_string()));
        }

        let trial_id = trial.id.clone();
        trials.insert(trial.id.clone(), trial);

        self.audit.append(
            user_id,
            user_role,
            AuditAction::TrialCreated,
            AuditSeverity::Info,
            &format!("Trial created: {}", trial_id)
        ).await?;

        Ok(())
    }
}
