//! Retenção de Dados — 21 CFR 312.62
//!
//! Gestão de políticas de retenção para dados de ensaios clínicos

use crate::{AuditTimestamp, ClinicalId};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionPolicy {
    pub id: ClinicalId,
    pub trial_id: String,
    pub retention_years: u32,
    pub start_date: AuditTimestamp,
    pub end_date: Option<AuditTimestamp>,
    pub status: RetentionStatus,
    pub data_archived: bool,
    pub archive_location: Option<String>,
    pub deleted_at: Option<AuditTimestamp>,
    pub deletion_certificate: Option<String>,
    pub notes: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RetentionStatus {
    Active,
    Expired,
    Archived,
    Deleted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionEvent {
    pub timestamp: AuditTimestamp,
    pub user_id: String,
    pub action: RetentionAction,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RetentionAction {
    PolicyCreated,
    PolicyUpdated,
    Archive,
    Delete,
    AlertSent,
    ExtensionGranted,
}

pub struct RetentionManager {
    policies: HashMap<String, RetentionPolicy>,
    events: Vec<RetentionEvent>,
    retention_years_default: u32,
}

impl RetentionManager {
    pub fn new(default_retention_years: u32) -> Self {
        Self {
            policies: HashMap::new(),
            events: Vec::new(),
            retention_years_default: default_retention_years,
        }
    }

    pub fn create_policy(
        &mut self,
        trial_id: &str,
        end_date: AuditTimestamp,
        user_id: &str,
    ) -> Result<RetentionPolicy, RetentionError> {
        if self.policies.contains_key(trial_id) {
            return Err(RetentionError::PolicyAlreadyExists);
        }

        let retention_years = self.retention_years_default.max(15);
        let end_date = end_date + Duration::days(retention_years as i64 * 365);

        let policy = RetentionPolicy {
            id: ClinicalId::new(),
            trial_id: trial_id.to_string(),
            retention_years,
            start_date: Utc::now(),
            end_date: Some(end_date),
            status: RetentionStatus::Active,
            data_archived: false,
            archive_location: None,
            deleted_at: None,
            deletion_certificate: None,
            notes: String::new(),
        };

        self.policies.insert(trial_id.to_string(), policy.clone());
        self.events.push(RetentionEvent {
            timestamp: Utc::now(),
            user_id: user_id.to_string(),
            action: RetentionAction::PolicyCreated,
            description: format!("Retention policy created for trial {}", trial_id),
        });

        Ok(policy)
    }

    pub fn get_policy(&self, trial_id: &str) -> Option<&RetentionPolicy> {
        self.policies.get(trial_id)
    }

    pub fn is_expired(&self, trial_id: &str) -> bool {
        if let Some(policy) = self.policies.get(trial_id) {
            if let Some(end_date) = policy.end_date {
                return Utc::now() > end_date && policy.status == RetentionStatus::Active;
            }
        }
        false
    }

    pub fn expiring_soon(&self, days: i64) -> Vec<(&String, &RetentionPolicy)> {
        let now = Utc::now();
        let threshold = now + Duration::days(days);

        self.policies
            .iter()
            .filter(|(_, p)| {
                if let Some(end_date) = p.end_date {
                    p.status == RetentionStatus::Active
                        && end_date > now
                        && end_date <= threshold
                } else {
                    false
                }
            })
            .collect()
    }

    pub fn archive_data(
        &mut self,
        trial_id: &str,
        location: &str,
        user_id: &str,
    ) -> Result<(), RetentionError> {
        let policy = self.policies
            .get_mut(trial_id)
            .ok_or(RetentionError::PolicyNotFound)?;

        policy.data_archived = true;
        policy.archive_location = Some(location.to_string());
        policy.status = RetentionStatus::Archived;

        self.events.push(RetentionEvent {
            timestamp: Utc::now(),
            user_id: user_id.to_string(),
            action: RetentionAction::Archive,
            description: format!("Data archived for trial {} at {}", trial_id, location),
        });

        Ok(())
    }

    pub fn delete_data(
        &mut self,
        trial_id: &str,
        certificate: &str,
        user_id: &str,
    ) -> Result<(), RetentionError> {
        let is_expired = self.is_expired(trial_id);
        if !is_expired {
            return Err(RetentionError::RetentionPeriodNotExpired);
        }

        let policy = self.policies
            .get_mut(trial_id)
            .ok_or(RetentionError::PolicyNotFound)?;

        policy.deleted_at = Some(Utc::now());
        policy.deletion_certificate = Some(certificate.to_string());
        policy.status = RetentionStatus::Deleted;

        self.events.push(RetentionEvent {
            timestamp: Utc::now(),
            user_id: user_id.to_string(),
            action: RetentionAction::Delete,
            description: format!("Data deleted for trial {} with certificate {}", trial_id, certificate),
        });

        Ok(())
    }

    pub fn stats(&self) -> RetentionStats {
        let total = self.policies.len();
        let active = self.policies.values().filter(|p| p.status == RetentionStatus::Active).count();
        let archived = self.policies.values().filter(|p| p.status == RetentionStatus::Archived).count();
        let deleted = self.policies.values().filter(|p| p.status == RetentionStatus::Deleted).count();
        let expired = self.policies.values().filter(|p| p.status == RetentionStatus::Expired).count();

        RetentionStats {
            total_policies: total,
            active,
            archived,
            deleted,
            expired,
            expiring_soon: self.expiring_soon(30).len(),
        }
    }

    pub fn events(&self) -> &[RetentionEvent] {
        &self.events
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionStats {
    pub total_policies: usize,
    pub active: usize,
    pub archived: usize,
    pub deleted: usize,
    pub expired: usize,
    pub expiring_soon: usize,
}

#[derive(Debug, thiserror::Error)]
pub enum RetentionError {
    #[error("Política de retenção já existe")]
    PolicyAlreadyExists,
    #[error("Política de retenção não encontrada")]
    PolicyNotFound,
    #[error("Período de retenção não expirou")]
    RetentionPeriodNotExpired,
    #[error("Trial não encontrado")]
    TrialNotFound,
}
