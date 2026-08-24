//! Consentimento Informado — ICH E6(R2) §4.8
//!
//! Gestão completa do ciclo de vida do consentimento informado:
//! - Criação com versão, hash do texto, assinatura
//! - Revogação com razão e timestamp
//! - Verificação de validade antes de qualquer procedimento
//! - Rastreabilidade completa no audit trail

use crate::{AuditTimestamp, ClinicalId};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};

/// Estado do consentimento informado
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConsentStatus {
    Pending,      // Aguardando assinatura
    Active,       // Válido e em vigor
    Revoked,      // Revogado pelo paciente
    Expired,      // Versão obsoleta (novo protocolo)
    Withdrawn,    // Retirado pelo investigador
}

/// Consentimento informado de um participante
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InformedConsent {
    pub id: ClinicalId,
    pub subject_code: String,
    pub protocol_version: String,
    pub consent_language: String,       // ISO 639-1 (pt, en, de, etc.)
    pub consent_text_hash: [u8; 32],    // SHA-256 do texto completo do CI
    pub signed_at: AuditTimestamp,
    pub valid_until: Option<AuditTimestamp>,
    pub signed_by: String,              // Nome do participante ou representante legal
    pub signed_by_role: SignatoryRole,
    pub witness_name: Option<String>,
    pub witness_id: Option<String>,
    pub status: ConsentStatus,
    pub revoked_at: Option<AuditTimestamp>,
    pub revocation_reason: Option<String>,
    pub investigator_id: String,
    pub site_id: String,
    pub created_at: AuditTimestamp,
    pub updated_at: AuditTimestamp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SignatoryRole {
    Participant,      // Participante adulto capaz
    LegalGuardian,    // Responsável legal (menor, incapaz)
    LegallyAuthorizedRepresentative, // Representante legal autorizado
    Interpreter,      // Intérprete (se participante não fala o idioma)
}

/// Versão do documento de consentimento informado
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsentDocument {
    pub version: String,
    pub language: String,
    pub text: String,
    pub hash: [u8; 32],
    pub effective_date: AuditTimestamp,
    pub irb_approval_date: AuditTimestamp,
    pub irb_reference: String,
}

impl ConsentDocument {
    pub fn new(version: &str, language: &str, text: &str, irb_ref: &str) -> Self {
        let hash = Self::compute_hash(text);
        Self {
            version: version.to_string(),
            language: language.to_string(),
            text: text.to_string(),
            hash,
            effective_date: Utc::now(),
            irb_approval_date: Utc::now(),
            irb_reference: irb_ref.to_string(),
        }
    }

    fn compute_hash(text: &str) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(text.as_bytes());
        hasher.finalize().into()
    }

    pub fn verify_hash(&self) -> bool {
        Self::compute_hash(&self.text) == self.hash
    }
}

/// Registro de consentimento (consent registry)
pub struct ConsentRegistry {
    documents: Vec<ConsentDocument>,
    consents: Vec<InformedConsent>,
}

impl ConsentRegistry {
    pub fn new() -> Self {
        Self {
            documents: Vec::new(),
            consents: Vec::new(),
        }
    }

    /// Registra uma nova versão do documento de CI
    pub fn register_document(&mut self, doc: ConsentDocument) {
        self.documents.push(doc);
    }

    /// Obtém o documento de CI ativo para uma versão e idioma
    pub fn active_document(&self, version: &str, language: &str) -> Option<&ConsentDocument> {
        self.documents
            .iter()
            .filter(|d| d.version == version && d.language == language)
            .max_by_key(|d| d.effective_date)
    }

    /// Cria um novo consentimento para um participante
    pub fn create_consent(
        &mut self,
        subject_code: &str,
        doc: &ConsentDocument,
        signed_by: &str,
        role: SignatoryRole,
        investigator_id: &str,
        site_id: &str,
    ) -> InformedConsent {
        let consent = InformedConsent {
            id: ClinicalId::new(),
            subject_code: subject_code.to_string(),
            protocol_version: doc.version.clone(),
            consent_language: doc.language.clone(),
            consent_text_hash: doc.hash,
            signed_at: Utc::now(),
            valid_until: None,
            signed_by: signed_by.to_string(),
            signed_by_role: role,
            witness_name: None,
            witness_id: None,
            status: ConsentStatus::Active,
            revoked_at: None,
            revocation_reason: None,
            investigator_id: investigator_id.to_string(),
            site_id: site_id.to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        self.consents.push(consent.clone());
        consent
    }

    /// Revoga um consentimento ativo
    pub fn revoke_consent(
        &mut self,
        consent_id: &ClinicalId,
        reason: &str,
    ) -> Result<(), ConsentError> {
        let consent = self.consents
            .iter_mut()
            .find(|c| c.id == *consent_id)
            .ok_or(ConsentError::NotFound)?;

        if consent.status != ConsentStatus::Active {
            return Err(ConsentError::NotActive);
        }

        consent.status = ConsentStatus::Revoked;
        consent.revoked_at = Some(Utc::now());
        consent.revocation_reason = Some(reason.to_string());
        consent.updated_at = Utc::now();

        Ok(())
    }

    /// Verifica se um participante tem consentimento válido
    pub fn is_valid(&self, subject_code: &str) -> bool {
        self.consents
            .iter()
            .filter(|c| c.subject_code == subject_code)
            .any(|c| c.status == ConsentStatus::Active)
    }

    /// Obtém o consentimento ativo mais recente de um participante
    pub fn active_consent(&self, subject_code: &str) -> Option<&InformedConsent> {
        self.consents
            .iter()
            .filter(|c| c.subject_code == subject_code && c.status == ConsentStatus::Active)
            .max_by_key(|c| c.signed_at)
    }

    /// Lista todos os consentimentos de um participante (histórico)
    pub fn consent_history(&self, subject_code: &str) -> Vec<&InformedConsent> {
        self.consents
            .iter()
            .filter(|c| c.subject_code == subject_code)
            .collect()
    }

    /// Verifica se o texto do CI foi alterado desde a assinatura
    pub fn verify_integrity(&self, consent: &InformedConsent, doc: &ConsentDocument) -> bool {
        consent.consent_text_hash == doc.hash && doc.verify_hash()
    }
}

impl Default for ConsentRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConsentError {
    #[error("Consentimento não encontrado")]
    NotFound,
    #[error("Consentimento não está ativo")]
    NotActive,
    #[error("Documento de CI não encontrado")]
    DocumentNotFound,
    #[error("Versão do CI obsoleta")]
    VersionObsolete,
}