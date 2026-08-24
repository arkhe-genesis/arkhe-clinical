//! Erros do sistema clínico

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ClinicalError {
    #[error("Trial not found: {0}")]
    TrialNotFound(String),

    #[error("Patient not found: {0}")]
    PatientNotFound(String),

    #[error("Protocol deviation: {0}")]
    ProtocolDeviation(String),

    #[error("Invariant violation: {0}")]
    InvariantViolation(String),

    #[error("Audit error: {0}")]
    AuditError(String),

    #[error("Biomarker analysis failed: {0}")]
    BiomarkerError(String),

    #[error("Invalid data: {0}")]
    InvalidData(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Regulatory violation: {0}")]
    RegulatoryViolation(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("YAML error: {0}")]
    Yaml(#[from] yaml_rust::ScanError),
}