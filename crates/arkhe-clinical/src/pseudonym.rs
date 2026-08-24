//! Pseudonimização — GDPR Art. 4(5), LGPD Art. 13
//!
//! Gestão de pseudônimos persistentes para participantes de ensaios clínicos

use crate::{AuditTimestamp, ClinicalId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pseudonym {
    pub subject_code: String,
    pub pseudonym: String,
    pub real_identifier: Option<String>,
    pub created_at: AuditTimestamp,
    pub updated_at: AuditTimestamp,
    pub active: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PseudonymAccess {
    Full,
    Restricted,
    GenerateOnly,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PseudonymMap {
    pub id: ClinicalId,
    pub trial_id: String,
    pub mappings: HashMap<String, Pseudonym>,
    pub encrypted_at: Option<AuditTimestamp>,
    pub access_log: Vec<PseudonymAccessLog>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PseudonymAccessLog {
    pub timestamp: AuditTimestamp,
    pub user_id: String,
    pub user_role: String,
    pub action: PseudonymAction,
    pub subject_code: Option<String>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PseudonymAction {
    View,
    Create,
    Update,
    Revoke,
}

pub struct PseudonymManager {
    trial_maps: HashMap<String, PseudonymMap>,
    access_control: HashMap<String, PseudonymAccess>,
}

impl PseudonymManager {
    pub fn new() -> Self {
        Self {
            trial_maps: HashMap::new(),
            access_control: HashMap::new(),
        }
    }

    pub fn create_map(&mut self, trial_id: &str) -> Result<PseudonymMap, PseudonymError> {
        if self.trial_maps.contains_key(trial_id) {
            return Err(PseudonymError::MapAlreadyExists);
        }

        let map = PseudonymMap {
            id: ClinicalId::new(),
            trial_id: trial_id.to_string(),
            mappings: HashMap::new(),
            encrypted_at: None,
            access_log: Vec::new(),
        };

        self.trial_maps.insert(trial_id.to_string(), map.clone());
        Ok(map)
    }

    pub fn generate_pseudonym(
        &mut self,
        trial_id: &str,
        subject_code: &str,
        real_identifier: Option<&str>,
        user_id: &str,
        user_role: &str,
    ) -> Result<String, PseudonymError> {
        let map = self.trial_maps
            .get_mut(trial_id)
            .ok_or(PseudonymError::MapNotFound)?;

        let count = map.mappings.len() + 1;
        let pseudonym = format!("PSEUDO-{:03}", count);

        let entry = Pseudonym {
            subject_code: subject_code.to_string(),
            pseudonym: pseudonym.clone(),
            real_identifier: real_identifier.map(|s| s.to_string()),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            active: true,
        };

        map.mappings.insert(subject_code.to_string(), entry);
        map.access_log.push(PseudonymAccessLog {
            timestamp: Utc::now(),
            user_id: user_id.to_string(),
            user_role: user_role.to_string(),
            action: PseudonymAction::Create,
            subject_code: Some(subject_code.to_string()),
            reason: Some("New pseudonym generation".to_string()),
        });

        Ok(pseudonym)
    }

    pub fn get_pseudonym(
        &self,
        trial_id: &str,
        subject_code: &str,
        user_id: &str,
        user_role: &str,
        _access_level: PseudonymAccess,
    ) -> Result<Option<String>, PseudonymError> {
        let required = self.access_control
            .get(user_id)
            .or(Some(&PseudonymAccess::Restricted))
            .unwrap();

        if matches!(required, PseudonymAccess::GenerateOnly) {
            return Err(PseudonymError::AccessDenied);
        }

        let map = self.trial_maps
            .get(trial_id)
            .ok_or(PseudonymError::MapNotFound)?;

        let result = map.mappings
            .get(subject_code)
            .map(|p| p.pseudonym.clone());

        Ok(result)
    }

    pub fn resolve_pseudonym(
        &self,
        trial_id: &str,
        pseudonym: &str,
        user_id: &str,
        _user_role: &str,
    ) -> Result<Option<String>, PseudonymError> {
        let required = self.access_control
            .get(user_id)
            .or(Some(&PseudonymAccess::Restricted))
            .unwrap();

        if !matches!(required, PseudonymAccess::Full) {
            return Err(PseudonymError::AccessDenied);
        }

        let map = self.trial_maps
            .get(trial_id)
            .ok_or(PseudonymError::MapNotFound)?;

        let result = map.mappings
            .values()
            .find(|p| p.pseudonym == pseudonym)
            .and_then(|p| p.real_identifier.clone());

        Ok(result)
    }

    pub fn revoke_pseudonym(
        &mut self,
        trial_id: &str,
        subject_code: &str,
        user_id: &str,
        user_role: &str,
    ) -> Result<(), PseudonymError> {
        let map = self.trial_maps
            .get_mut(trial_id)
            .ok_or(PseudonymError::MapNotFound)?;

        if let Some(entry) = map.mappings.get_mut(subject_code) {
            entry.active = false;
            entry.updated_at = Utc::now();
            map.access_log.push(PseudonymAccessLog {
                timestamp: Utc::now(),
                user_id: user_id.to_string(),
                user_role: user_role.to_string(),
                action: PseudonymAction::Revoke,
                subject_code: Some(subject_code.to_string()),
                reason: Some("Pseudonym revoked".to_string()),
            });
            Ok(())
        } else {
            Err(PseudonymError::PseudonymNotFound)
        }
    }

    pub fn set_access_level(&mut self, user_id: &str, level: PseudonymAccess) {
        self.access_control.insert(user_id.to_string(), level);
    }

    pub fn export_audit(&self, trial_id: &str) -> Result<serde_json::Value, PseudonymError> {
        let map = self.trial_maps
            .get(trial_id)
            .ok_or(PseudonymError::MapNotFound)?;

        Ok(serde_json::json!({
            "trial_id": map.trial_id,
            "total_mappings": map.mappings.len(),
            "active_mappings": map.mappings.values().filter(|p| p.active).count(),
            "access_log_entries": map.access_log.len(),
            "encrypted_at": map.encrypted_at,
        }))
    }
}

impl Default for PseudonymManager {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PseudonymError {
    #[error("Mapa de pseudônimos não encontrado")]
    MapNotFound,
    #[error("Mapa já existe")]
    MapAlreadyExists,
    #[error("Pseudônimo não encontrado")]
    PseudonymNotFound,
    #[error("Acesso negado")]
    AccessDenied,
}
