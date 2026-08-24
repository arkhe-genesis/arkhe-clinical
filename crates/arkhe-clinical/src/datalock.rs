//! Data Lock Irreversível — 21 CFR Part 11
//!
//! Implementa o conceito de "database lock" em ensaios clínicos:
//! - Uma vez travado, os dados são imutáveis
//! - Hash criptográfico do dataset completo
//! - Assinatura digital do sponsor/CRO
//! - Verificação de integridade a qualquer momento

use crate::{AuditTimestamp, ClinicalId};
use chrono::Utc;
#[cfg(feature = "with_audit_signatures")]
use ed25519_dalek::{Signer, SigningKey, Signature, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};

/// Estado do data lock
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LockStatus {
    Unlocked,     // Dados em edição
    Pending,      // Lock solicitado, aguardando verificação
    Locked,       // Dataset travado e imutável
    Breached,     // Violação detectada (integridade comprometida)
}

/// Data Lock de um ensaio clínico
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataLock {
    pub id: ClinicalId,
    pub trial_id: String,
    pub locked_at: AuditTimestamp,
    pub locked_by: String,
    pub lock_reason: String,
    pub dataset_hash: [u8; 32],           // SHA-256 do dataset serializado
    pub audit_trail_hash: [u8; 32],       // Hash do último evento do audit trail
    #[serde(with = "hex")]
    pub signature: [u8; 64],              // Assinatura Ed25519 do dataset_hash
    pub public_key: [u8; 32],             // Chave pública do signatário
    pub status: LockStatus,
    pub verification_count: u64,
    pub last_verified: Option<AuditTimestamp>,
}

/// Gerenciador de data locks
pub struct DataLockManager {
    locks: Vec<DataLock>,
    #[cfg(feature = "with_audit_signatures")]
    signing_key: SigningKey,
}

impl DataLockManager {
    #[cfg(feature = "with_audit_signatures")]
    pub fn new(signing_key: SigningKey) -> Self {
        Self {
            locks: Vec::new(),
            signing_key,
        }
    }

    #[cfg(not(feature = "with_audit_signatures"))]
    pub fn new() -> Self {
        Self {
            locks: Vec::new(),
        }
    }

    /// Aplica data lock em um dataset
    pub fn lock(
        &mut self,
        trial_id: &str,
        dataset: &[u8],
        audit_trail_tail: &[u8],
        locked_by: &str,
        reason: &str,
    ) -> Result<DataLock, LockError> {
        if self.is_locked(trial_id) {
            return Err(LockError::AlreadyLocked);
        }

        let dataset_hash = Self::compute_hash(dataset);
        let audit_hash = Self::compute_hash(audit_trail_tail);

        #[cfg(feature = "with_audit_signatures")]
        let (signature_bytes, public_key_bytes) = {
            let signature = self.signing_key.sign(&dataset_hash);
            let public_key = *self.signing_key.verifying_key().as_bytes();
            (signature.to_bytes(), public_key)
        };

        #[cfg(not(feature = "with_audit_signatures"))]
        let (signature_bytes, public_key_bytes) = ([0u8; 64], [0u8; 32]);

        let lock = DataLock {
            id: ClinicalId::new(),
            trial_id: trial_id.to_string(),
            locked_at: Utc::now(),
            locked_by: locked_by.to_string(),
            lock_reason: reason.to_string(),
            dataset_hash,
            audit_trail_hash: audit_hash,
            signature: signature_bytes,
            public_key: public_key_bytes,
            status: LockStatus::Locked,
            verification_count: 0,
            last_verified: None,
        };

        self.locks.push(lock.clone());
        Ok(lock)
    }

    /// Verifica a integridade de um data lock
    pub fn verify(
        &mut self,
        lock_id: &ClinicalId,
        dataset: &[u8],
    ) -> Result<bool, LockError> {
        let lock = self.locks
            .iter_mut()
            .find(|l| l.id == *lock_id)
            .ok_or(LockError::NotFound)?;

        let computed_hash = Self::compute_hash(dataset);
        let integrity_ok = computed_hash == lock.dataset_hash;

        #[cfg(feature = "with_audit_signatures")]
        let sig_ok = {
            let public_key = VerifyingKey::from_bytes(&lock.public_key)
                .map_err(|_| LockError::InvalidKey)?;
            let sig = Signature::from_bytes(&lock.signature);
            public_key.verify_strict(&lock.dataset_hash, &sig).is_ok()
        };

        #[cfg(not(feature = "with_audit_signatures"))]
        let sig_ok = true;

        lock.verification_count += 1;
        lock.last_verified = Some(Utc::now());

        if !integrity_ok || !sig_ok {
            lock.status = LockStatus::Breached;
            return Ok(false);
        }

        Ok(true)
    }

    pub fn is_locked(&self, trial_id: &str) -> bool {
        self.locks
            .iter()
            .any(|l| l.trial_id == trial_id && l.status == LockStatus::Locked)
    }

    pub fn active_lock(&self, trial_id: &str) -> Option<&DataLock> {
        self.locks
            .iter()
            .find(|l| l.trial_id == trial_id && l.status == LockStatus::Locked)
    }

    fn compute_hash(data: &[u8]) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(data);
        hasher.finalize().into()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum LockError {
    #[error("Trial já possui data lock ativo")]
    AlreadyLocked,
    #[error("Data lock não encontrado")]
    NotFound,
    #[error("Chave pública inválida")]
    InvalidKey,
    #[error("Dataset vazio")]
    EmptyDataset,
}
