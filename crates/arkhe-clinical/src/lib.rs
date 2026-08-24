//! ARKHE Clinical — Plataforma para ensaios clínicos em Doença de Creutzfeldt-Jakob
//!
//! Este crate implementa:
//! - Invariantes regulatórios (ICH-GCP, 21 CFR Part 11)
//! - Audit trail imutável e à prova de violação
//! - Análise de biomarcadores RT-QuIC (sensibilidade 92-97%, especificidade 100%)
//! - Gestão de ensaios clínicos e pacientes
//! - Análise Bayesiana adaptativa para doenças raras
//! - Consentimento informado (ICH E6(R2) §4.8)
//! - Data Lock irreversível (21 CFR Part 11)
//! - Randomização estratificada (ICH E6(R2) §4.9)
//! - SAE Reporting com CIOMS I (ICH E2A)
//! - Pseudonimização (GDPR Art. 4(5))
//! - Retenção de dados (21 CFR 312.62)
//! - Anonimato na exportação (GDPR, ICH E6(R2) §4.9.5)

pub mod audit;
pub mod biomarker;
pub mod trial;
pub mod invariant;
pub mod error;
pub mod consent;
pub mod datalock;
pub mod randomization;
pub mod safety;
pub mod pseudonym;
pub mod retention;
pub mod export;

pub use audit::{AuditTrail, AuditEntry, AuditAction, AuditSeverity};
pub use biomarker::{RtQuicResult, RtQuicAnalysis, BiomarkerEvent};
pub use trial::{ClinicalTrial, TrialStatus, Patient, PatientStatus, TrialEvent};
pub use invariant::{ClinicalInvariant, InvariantEngine, InvariantStatus};
pub use error::ClinicalError;
pub use consent::{InformedConsent, ConsentRegistry, ConsentStatus, ConsentDocument};
pub use datalock::{DataLock, DataLockManager, LockStatus};
pub use randomization::{StratifiedRandomizer, RandomizationResult, StratumFactor};
pub use safety::{SafetyMonitor, AdverseEvent, CiomsForm, SafetySummary};
pub use pseudonym::{PseudonymManager, Pseudonym, PseudonymAccess};
pub use retention::{RetentionManager, RetentionPolicy, RetentionStatus};
pub use export::{ExportAnonymizer, ExportManager, ExportConfig, AnonymizationLevel, ExportRecord};

use serde::{Deserialize, Serialize};

/// Versão do crate
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Identificador clínico (wrapper para UUID)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ClinicalId(pub uuid::Uuid);

impl ClinicalId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }
}

impl std::fmt::Display for ClinicalId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.simple())
    }
}

/// Timestamp com formato padrão para auditoria
pub type AuditTimestamp = chrono::DateTime<chrono::Utc>;

/// Genótipo PRNP
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PrnpGenotype {
    MM129,
    MV129,
    VV129,
    E200K,
    V210I,
    Other,
}