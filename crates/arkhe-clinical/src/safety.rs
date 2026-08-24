//! Safety Reporting — ICH E2A / CIOMS I
//!
//! Gestão de Eventos Adversos Graves (SAEs) com:
//! - Templates CIOMS I para reporte regulatório
//! - Alertas automáticos baseados em severidade e temporalidade
//! - Rastreabilidade completa desde o onset até o desfecho

use crate::{AuditTimestamp, ClinicalId};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[derive(Hash)]
pub enum AeSeverity {
    Mild,
    Moderate,
    Severe,
    LifeThreatening,
    Fatal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Causality {
    Unrelated,
    Unlikely,
    Possible,
    Probable,
    Definite,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AeOutcome {
    Recovered,
    Recovering,
    NotRecovered,
    RecoveredWithSequelae,
    Fatal,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdverseEvent {
    pub id: ClinicalId,
    pub subject_code: String,
    pub description: String,
    pub onset_date: AuditTimestamp,
    pub stop_date: Option<AuditTimestamp>,
    pub severity: AeSeverity,
    pub causality: Causality,
    pub expected: bool,
    pub outcome: AeOutcome,
    pub sae: bool,
    pub sae_criteria: Vec<SaeCriterion>,
    pub action_taken: Vec<ActionTaken>,
    pub reported_to_sponsor: Option<AuditTimestamp>,
    pub reported_to_regulatory: Option<AuditTimestamp>,
    pub cioms_form: Option<CiomsForm>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SaeCriterion {
    Death,
    LifeThreatening,
    Hospitalization,
    Disability,
    CongenitalAnomaly,
    OtherImportantMedicalEvent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionTaken {
    None,
    DoseReduced,
    DoseIncreased,
    DrugInterrupted,
    DrugWithdrawn,
    Hospitalization,
    ConcomitantTherapy,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CiomsForm {
    pub report_id: String,
    pub reporter_name: String,
    pub reporter_address: String,
    pub reporter_country: String,
    pub patient_initials: String,
    pub patient_sex: String,
    pub patient_date_of_birth: Option<String>,
    pub patient_age_at_event: Option<u8>,
    pub reaction_onset_date: String,
    pub reaction_description: String,
    pub suspect_drugs: Vec<SuspectDrug>,
    pub concomitant_drugs: Vec<String>,
    pub other_therapies: Vec<String>,
    pub relevant_tests: Vec<String>,
    pub medical_history: Vec<String>,
    pub manufacturer_name: String,
    pub manufacturer_reference: String,
    pub report_date: AuditTimestamp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuspectDrug {
    pub name: String,
    pub daily_dose: String,
    pub route: String,
    pub indication: String,
    pub therapy_dates: String,
    pub therapy_duration: String,
}

pub struct SafetyMonitor {
    aes: Vec<AdverseEvent>,
    reporting_deadline_hours: HashMap<AeSeverity, i64>,
}

impl SafetyMonitor {
    pub fn new() -> Self {
        let mut deadlines = HashMap::new();
        deadlines.insert(AeSeverity::Fatal, 24);
        deadlines.insert(AeSeverity::LifeThreatening, 24);
        deadlines.insert(AeSeverity::Severe, 72);
        deadlines.insert(AeSeverity::Moderate, 15 * 24);
        deadlines.insert(AeSeverity::Mild, 15 * 24);

        Self {
            aes: Vec::new(),
            reporting_deadline_hours: deadlines,
        }
    }

    pub fn report_ae(&mut self, ae: AdverseEvent) {
        self.aes.push(ae);
    }

    pub fn pending_saes(&self) -> Vec<&AdverseEvent> {
        self.aes
            .iter()
            .filter(|ae| ae.sae)
            .filter(|ae| ae.reported_to_sponsor.is_none())
            .collect()
    }

    pub fn overdue_saes(&self) -> Vec<(&AdverseEvent, i64)> {
        let now = Utc::now();
        self.aes
            .iter()
            .filter(|ae| ae.sae)
            .filter(|ae| ae.reported_to_sponsor.is_none())
            .filter_map(|ae| {
                let deadline = self.reporting_deadline_hours.get(&ae.severity).copied().unwrap_or(15 * 24);
                let elapsed = (now - ae.onset_date).num_hours();
                if elapsed > deadline {
                    Some((ae, elapsed - deadline))
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn generate_cioms(&self, ae: &AdverseEvent) -> CiomsForm {
        CiomsForm {
            report_id: format!("CIOMS-{}-{}", ae.subject_code, ae.id.0),
            reporter_name: "Principal Investigator".to_string(),
            reporter_address: "Clinical Site Address".to_string(),
            reporter_country: "PT".to_string(),
            patient_initials: ae.subject_code.clone(),
            patient_sex: "Unknown".to_string(),
            patient_date_of_birth: None,
            patient_age_at_event: None,
            reaction_onset_date: ae.onset_date.format("%Y-%m-%d").to_string(),
            reaction_description: ae.description.clone(),
            suspect_drugs: vec![SuspectDrug {
                name: "Investigational Product".to_string(),
                daily_dose: "N/A".to_string(),
                route: "IV".to_string(),
                indication: "sCJD".to_string(),
                therapy_dates: "N/A".to_string(),
                therapy_duration: "N/A".to_string(),
            }],
            concomitant_drugs: vec![],
            other_therapies: vec![],
            relevant_tests: vec![],
            medical_history: vec!["sCJD".to_string()],
            manufacturer_name: "Sponsor Name".to_string(),
            manufacturer_reference: ae.id.0.to_string(),
            report_date: Utc::now(),
        }
    }

    pub fn mark_reported_to_sponsor(&mut self, ae_id: &ClinicalId) {
        if let Some(ae) = self.aes.iter_mut().find(|a| a.id == *ae_id) {
            ae.reported_to_sponsor = Some(Utc::now());
        }
    }

    pub fn safety_summary(&self) -> SafetySummary {
        let total = self.aes.len();
        let saes = self.aes.iter().filter(|a| a.sae).count();
        let fatal = self.aes.iter().filter(|a| a.outcome == AeOutcome::Fatal).count();
        let by_severity: HashMap<String, usize> = self.aes
            .iter()
            .map(|a| (format!("{:?}", a.severity), 1))
            .fold(HashMap::new(), |mut acc, (k, v)| {
                *acc.entry(k).or_insert(0) += v;
                acc
            });

        SafetySummary {
            total_aes: total,
            total_saes: saes,
            fatal_events: fatal,
            by_severity,
            pending_reports: self.pending_saes().len(),
            overdue_reports: self.overdue_saes().len(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetySummary {
    pub total_aes: usize,
    pub total_saes: usize,
    pub fatal_events: usize,
    pub by_severity: HashMap<String, usize>,
    pub pending_reports: usize,
    pub overdue_reports: usize,
}

impl Default for SafetyMonitor {
    fn default() -> Self {
        Self::new()
    }
}
