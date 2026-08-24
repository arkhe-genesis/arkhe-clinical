//! Anonimato na Exportação — GDPR, ICH E6(R2) §4.9.5
//!
//! Exportação de dados com anonimização irreversível

use crate::{AuditTimestamp, ClinicalId};
use chrono::Utc;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnonymizationLevel {
    Pseudonymized,
    Anonymized,
    Aggregated,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportConfig {
    pub anonymization_level: AnonymizationLevel,
    pub include_biomarkers: bool,
    pub include_clinical_data: bool,
    pub include_adverse_events: bool,
    pub date_precision: DatePrecision,
    pub age_grouping: AgeGrouping,
    pub export_format: ExportFormat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DatePrecision {
    Exact,
    Month,
    Year,
    Relative,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgeGrouping {
    Exact,
    Decade,
    Quintile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExportFormat {
    Csv,
    Json,
    Parquet,
    Fhir,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportRecord {
    pub id: ClinicalId,
    pub trial_id: String,
    pub exported_at: AuditTimestamp,
    pub exported_by: String,
    pub config: ExportConfig,
    pub anonymization_level: AnonymizationLevel,
    pub file_hash: Option<String>,
    pub file_size: Option<u64>,
    pub purpose: String,
    pub recipient: String,
    pub approved_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnonymizedPatient {
    pub subject_code: String,
    pub age_group: Option<String>,
    pub sex: Option<String>,
    pub diagnosis: Option<String>,
    pub mmse_score: Option<u8>,
    pub survival_days: Option<u64>,
    pub rt_quic_positive: Option<bool>,
    pub adverse_events: Vec<AnonymizedAdverseEvent>,
    pub biomarkers: Vec<AnonymizedBiomarker>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnonymizedAdverseEvent {
    pub event_type: String,
    pub severity: String,
    pub days_from_start: Option<u64>,
    pub outcome: String,
    pub related_to_treatment: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnonymizedBiomarker {
    pub name: String,
    pub value: f64,
    pub unit: String,
    pub days_from_start: Option<u64>,
}

pub struct ExportAnonymizer {
    anonymization_level: AnonymizationLevel,
}

impl ExportAnonymizer {
    pub fn new(level: AnonymizationLevel) -> Self {
        Self { anonymization_level: level }
    }

    pub fn anonymize(
        &self,
        patients: &[crate::trial::Patient],
        config: &ExportConfig,
    ) -> Vec<AnonymizedPatient> {
        patients.iter()
            .map(|p| self.anonymize_patient(p, config))
            .collect()
    }

    pub fn anonymize_patient(
        &self,
        patient: &crate::trial::Patient,
        config: &ExportConfig,
    ) -> AnonymizedPatient {
        let subject_code = match config.anonymization_level {
            AnonymizationLevel::Pseudonymized => patient.id.clone(),
            AnonymizationLevel::Anonymized => {
                format!("S-{}", patient.id.chars().take(4).collect::<String>())
            }
            AnonymizationLevel::Aggregated => "AGG".to_string(),
        };

        let age_group = match config.age_grouping {
            AgeGrouping::Exact => Some(patient.demographic.age_at_enrollment.to_string()),
            AgeGrouping::Decade => {
                let age = patient.demographic.age_at_enrollment;
                Some(format!("{}-{}", (age / 10) * 10, (age / 10) * 10 + 9))
            }
            AgeGrouping::Quintile => {
                let age = patient.demographic.age_at_enrollment;
                if age < 30 { Some("18-29".to_string()) }
                else if age < 50 { Some("30-49".to_string()) }
                else if age < 60 { Some("50-59".to_string()) }
                else if age < 70 { Some("60-69".to_string()) }
                else { Some("70+".to_string()) }
            }
        };

        let aes = patient.adverse_events.iter()
            .filter(|_ae| config.include_adverse_events)
            .map(|ae| AnonymizedAdverseEvent {
                event_type: format!("{:?}", ae.event_type),
                severity: format!("{:?}", ae.severity),
                days_from_start: Some((ae.start_date - patient.enrollment_date).num_days() as u64),
                outcome: "Unknown".to_string(),
                related_to_treatment: Some(ae.related_to_intervention),
            })
            .collect();

        let biomarkers = patient.biomarkers.iter()
            .filter(|_b| config.include_biomarkers)
            .map(|b| AnonymizedBiomarker {
                name: format!("{:?}", b.biomarker_type),
                value: b.value,
                unit: b.unit.clone(),
                days_from_start: Some((b.timestamp - patient.enrollment_date).num_days() as u64),
            })
            .collect();

        let survival_days = if patient.status == crate::trial::PatientStatus::Deceased {
            patient.withdrawal_date.map(|d| (d - patient.enrollment_date).num_days() as u64)
        } else {
            None
        };

        AnonymizedPatient {
            subject_code,
            age_group,
            sex: match config.anonymization_level {
                AnonymizationLevel::Pseudonymized => Some(format!("{:?}", patient.demographic.gender)),
                _ => None,
            },
            diagnosis: if config.include_clinical_data {
                Some(patient.clinical_data.diagnosis.clone())
            } else {
                None
            },
            mmse_score: patient.clinical_data.mmse_score,
            survival_days,
            rt_quic_positive: patient.rt_quic_results.last().map(|r| r.seed_positive),
            adverse_events: aes,
            biomarkers,
        }
    }

    pub fn generate_dataset_hash(data: &[u8]) -> String {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(data);
        format!("{:x}", hasher.finalize())
    }
}

pub struct ExportManager {
    exports: Vec<ExportRecord>,
}

impl ExportManager {
    pub fn new() -> Self {
        Self { exports: Vec::new() }
    }

    pub fn register_export(
        &mut self,
        trial_id: &str,
        config: &ExportConfig,
        exported_by: &str,
        purpose: &str,
        recipient: &str,
        file_data: &[u8],
        approved_by: Option<&str>,
    ) -> ExportRecord {
        let record = ExportRecord {
            id: ClinicalId::new(),
            trial_id: trial_id.to_string(),
            exported_at: Utc::now(),
            exported_by: exported_by.to_string(),
            config: config.clone(),
            anonymization_level: config.anonymization_level,
            file_hash: Some(ExportAnonymizer::generate_dataset_hash(file_data)),
            file_size: Some(file_data.len() as u64),
            purpose: purpose.to_string(),
            recipient: recipient.to_string(),
            approved_by: approved_by.map(|s| s.to_string()),
        };

        self.exports.push(record.clone());
        record
    }

    pub fn list_exports(&self, trial_id: &str) -> Vec<&ExportRecord> {
        self.exports
            .iter()
            .filter(|e| e.trial_id == trial_id)
            .collect()
    }
}

impl Default for ExportManager {
    fn default() -> Self {
        Self::new()
    }
}
