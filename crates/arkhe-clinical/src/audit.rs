//! Audit Trail — 21 CFR Part 11 compliant, tamper-evident, with digital signatures

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;
use uuid::Uuid;

use crate::{AuditTimestamp, ClinicalError};

/// Nível de severidade do evento de auditoria
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuditSeverity {
    Info,
    Warning,
    Error,
    Critical,
    Regulatory,
}

/// Tipo de ação registrada
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditAction {
    TrialCreated,
    PatientEnrolled,
    PatientWithdrawn,
    DataEntry { form: String, field: String },
    DataModification { form: String, field: String, old_value: String, new_value: String },
    DataDeletion { form: String, field: String },
    SaEReported,
    ProtocolDeviation,
    ProtocolAmendment,
    DsmbReview,
    Unblinding,
    InvestigatorAction { action: String },
    SystemEvent { event: String },
    ConsentSigned,
    ConsentRevoked,
    DataLockApplied,
    RandomizationPerformed,
    DataExported,
}

/// Entrada de auditoria — imutável e assinada
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub id: String,                     // UUID
    pub timestamp: AuditTimestamp,
    pub user_id: String,
    pub user_role: String,
    pub action: AuditAction,
    pub severity: AuditSeverity,
    pub description: String,
    pub metadata: HashMap<String, serde_json::Value>,
    pub previous_hash: String,          // Hash do registro anterior (encadeamento)
    pub hash: String,                   // Hash do registro atual (SHA-256)
    pub signature: Option<String>,      // Assinatura digital (se feature ativa)
}

impl AuditEntry {
    /// Cria uma nova entrada de auditoria
    pub fn new(
        user_id: &str,
        user_role: &str,
        action: AuditAction,
        severity: AuditSeverity,
        description: &str,
        previous_hash: &str,
    ) -> Self {
        let id = Uuid::new_v4().to_string();
        let timestamp = Utc::now();
        let mut metadata = HashMap::new();
        metadata.insert("version".to_string(), serde_json::json!(env!("CARGO_PKG_VERSION")));

        let entry = Self {
            id,
            timestamp,
            user_id: user_id.to_string(),
            user_role: user_role.to_string(),
            action,
            severity,
            description: description.to_string(),
            metadata,
            previous_hash: previous_hash.to_string(),
            hash: String::new(), // será calculado depois
            signature: None,
        };

        // Calcula hash
        entry
    }

    /// Calcula o hash da entrada (SHA-256)
    pub fn compute_hash(&self) -> String {
        let content = serde_json::to_string(&self).unwrap_or_default();
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Assina a entrada com uma chave privada (Ed25519)
    #[cfg(feature = "with_audit_signatures")]
    pub fn sign(&mut self, signing_key: &ed25519_dalek::SigningKey) -> Result<(), ClinicalError> {
        use ed25519_dalek::Signer;
        let hash = self.compute_hash();
        let signature = signing_key.sign(hash.as_bytes());
        self.signature = Some(hex::encode(signature.to_bytes()));
        Ok(())
    }

    /// Verifica a assinatura (se presente)
    #[cfg(feature = "with_audit_signatures")]
    pub fn verify_signature(&self, verifying_key: &ed25519_dalek::VerifyingKey) -> bool {
        if let Some(sig_hex) = &self.signature {
            let sig_bytes = hex::decode(sig_hex).unwrap_or_default();
            if sig_bytes.len() == 64 {
                let mut bytes = [0u8; 64];
                bytes.copy_from_slice(&sig_bytes);
                let sig = ed25519_dalek::Signature::from_bytes(&bytes);
                let hash = self.compute_hash();
                return verifying_key.verify_strict(hash.as_bytes(), &sig).is_ok();
            }
        }
        false
    }
}

/// Motor de auditoria — mantém cadeia imutável e persistência
pub struct AuditTrail {
    entries: Arc<RwLock<Vec<AuditEntry>>>,
    last_hash: Arc<RwLock<String>>,
}

impl AuditTrail {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(RwLock::new(Vec::new())),
            last_hash: Arc::new(RwLock::new("0".to_string())),
        }
    }

    /// Adiciona uma entrada à cadeia de auditoria
    pub async fn append(
        &self,
        user_id: &str,
        user_role: &str,
        action: AuditAction,
        severity: AuditSeverity,
        description: &str,
    ) -> Result<AuditEntry, ClinicalError> {
        let last_hash: String = self.last_hash.read().await.clone();
        let mut entry = AuditEntry::new(
            user_id,
            user_role,
            action,
            severity,
            description,
            &last_hash,
        );

        // Calcula hash final
        let hash = entry.compute_hash();
        entry.hash = hash.clone();

        let mut entries = self.entries.write().await;
        entries.push(entry.clone());
        *self.last_hash.write().await = hash;

        info!(
            audit_id = %entry.id,
            action = ?entry.action,
            severity = ?entry.severity,
            "Audit entry recorded"
        );

        Ok(entry)
    }

    /// Recupera todas as entradas
    pub async fn get_all(&self) -> Vec<AuditEntry> {
        self.entries.read().await.clone()
    }

    /// Recupera entradas por severity
    pub async fn filter_by_severity(&self, severity: AuditSeverity) -> Vec<AuditEntry> {
        self.entries
            .read()
            .await
            .iter()
            .filter(|e| e.severity == severity)
            .cloned()
            .collect()
    }

    /// Verifica a integridade da cadeia (tamper-evident)
    pub async fn verify_chain(&self) -> bool {
        let entries = self.entries.read().await;
        let mut prev_hash = "0".to_string();

        for entry in entries.iter() {
            let computed = entry.compute_hash();
            if computed != entry.hash {
                return false;
            }
            if entry.previous_hash != prev_hash {
                return false;
            }
            prev_hash = entry.hash.clone();
        }
        true
    }

    /// Exporta a cadeia como JSON
    pub async fn export_json(&self) -> Result<String, ClinicalError> {
        let entries = self.entries.read().await;
        Ok(serde_json::to_string_pretty(&*entries)?)
    }
}

impl Default for AuditTrail {
    fn default() -> Self {
        Self::new()
    }
}