//! Implementação em memória de `SecurityPolicyRepository`.
//!
//! Referência semântica e base para testes. Thread-safe via `RwLock`.
//! Para persistência durável usar o adapter `security-sqlite`.

use std::collections::HashMap;
use std::sync::RwLock;

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::{
    validate_policy, Delegation, DelegationId, DelegationRequest, ListOptions, Policy,
    RevocationRequest, SecurityError, SecurityPolicyRepository,
};

struct StoredPolicy {
    policy: Policy,
    revoked: bool,
    revoked_by: Option<String>,
}

struct Inner {
    policies: HashMap<String, StoredPolicy>,
    delegations: HashMap<String, Delegation>,
}

pub struct InMemorySecurityPolicyRepository {
    inner: RwLock<Inner>,
}

impl InMemorySecurityPolicyRepository {
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(Inner {
                policies: HashMap::new(),
                delegations: HashMap::new(),
            }),
        }
    }
}

impl Default for InMemorySecurityPolicyRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl SecurityPolicyRepository for InMemorySecurityPolicyRepository {
    async fn save_policy(&self, policy: &Policy, _now: DateTime<Utc>) -> Result<(), SecurityError> {
        validate_policy(policy)?;
        let mut guard = self
            .inner
            .write()
            .map_err(|_| SecurityError::RepoUnavailable("lock poisoned".into()))?;

        if let Some(existing) = guard.policies.get(&policy.policy_id) {
            if existing.policy.version != policy.version {
                return Err(SecurityError::AlreadyExists(format!(
                    "policy_id '{}' já existe com version '{}'",
                    policy.policy_id, existing.policy.version
                )));
            }
            return Ok(());
        }

        guard.policies.insert(
            policy.policy_id.clone(),
            StoredPolicy {
                policy: policy.clone(),
                revoked: false,
                revoked_by: None,
            },
        );
        Ok(())
    }

    async fn get_policy(&self, policy_id: &str) -> Result<Option<Policy>, SecurityError> {
        let guard = self
            .inner
            .read()
            .map_err(|_| SecurityError::RepoUnavailable("lock poisoned".into()))?;
        Ok(guard.policies.get(policy_id).map(|s| s.policy.clone()))
    }

    async fn list_active_policies(
        &self,
        opts: Option<ListOptions>,
    ) -> Result<Vec<Policy>, SecurityError> {
        let guard = self
            .inner
            .read()
            .map_err(|_| SecurityError::RepoUnavailable("lock poisoned".into()))?;
        let mut all: Vec<Policy> = guard
            .policies
            .values()
            .filter(|s| !s.revoked)
            .map(|s| s.policy.clone())
            .collect();
        all.sort_by(|a, b| a.policy_id.cmp(&b.policy_id));
        Ok(apply_page(all, opts))
    }

    async fn revoke_policy(
        &self,
        req: &RevocationRequest,
        _now: DateTime<Utc>,
    ) -> Result<(), SecurityError> {
        req.validate()?;
        let mut guard = self
            .inner
            .write()
            .map_err(|_| SecurityError::RepoUnavailable("lock poisoned".into()))?;
        let stored = guard
            .policies
            .get_mut(&req.policy_id)
            .ok_or_else(|| SecurityError::PolicyNotFound(req.policy_id.clone()))?;
        stored.revoked = true;
        stored.revoked_by = Some(req.revoked_by.clone());
        Ok(())
    }

    async fn delegate_permission(
        &self,
        req: &DelegationRequest,
        now: DateTime<Utc>,
    ) -> Result<Delegation, SecurityError> {
        req.validate()?;
        let id = Uuid::new_v4().to_string();
        let delegation = Delegation {
            delegation_id: DelegationId(id.clone()),
            principal: req.principal.clone(),
            operation: req.operation.clone(),
            resource: req.resource.clone(),
            granted_by: req.granted_by.clone(),
            granted_at: now,
            valid_from: req.valid_from,
            valid_to: req.valid_to,
            conditions: req.conditions.clone(),
            revoked: false,
            granted_via: req.granted_via.clone(),
        };
        let mut guard = self
            .inner
            .write()
            .map_err(|_| SecurityError::RepoUnavailable("lock poisoned".into()))?;
        guard.delegations.insert(id, delegation.clone());
        Ok(delegation)
    }

    async fn list_delegations(
        &self,
        principal: &str,
        now: DateTime<Utc>,
        opts: Option<ListOptions>,
    ) -> Result<Vec<Delegation>, SecurityError> {
        let guard = self
            .inner
            .read()
            .map_err(|_| SecurityError::RepoUnavailable("lock poisoned".into()))?;
        let mut all: Vec<Delegation> = guard
            .delegations
            .values()
            .filter(|d| d.principal == principal && d.is_active_at(now))
            .cloned()
            .collect();
        all.sort_by(|a, b| a.delegation_id.0.cmp(&b.delegation_id.0));
        Ok(apply_page(all, opts))
    }

    async fn revoke_delegation(
        &self,
        delegation_id: &DelegationId,
        _now: DateTime<Utc>,
    ) -> Result<(), SecurityError> {
        let mut guard = self
            .inner
            .write()
            .map_err(|_| SecurityError::RepoUnavailable("lock poisoned".into()))?;

        // Verificar que a delegação existe e não está já revogada
        match guard.delegations.get(delegation_id.as_str()) {
            None => {
                return Err(SecurityError::DelegationNotFound(
                    delegation_id.as_str().into(),
                ))
            }
            Some(d) if d.revoked => {
                return Err(SecurityError::DelegationNotFound(
                    delegation_id.as_str().into(),
                ))
            }
            _ => {}
        }

        // BFS para recolher todos os IDs na cadeia de delegação
        let mut stack = vec![delegation_id.as_str().to_string()];
        let mut all_ids: Vec<String> = vec![];

        while let Some(current) = stack.pop() {
            all_ids.push(current.clone());
            // Filhos directos: delegações não revogadas cujo granted_via aponta para `current`
            let children: Vec<String> = guard
                .delegations
                .values()
                .filter(|d| {
                    !d.revoked
                        && d.granted_via.as_ref().map(|g| g.as_str()) == Some(current.as_str())
                })
                .map(|d| d.delegation_id.as_str().to_string())
                .collect();
            stack.extend(children);
        }

        // Revogar todos
        for id in all_ids {
            if let Some(d) = guard.delegations.get_mut(&id) {
                d.revoked = true;
            }
        }
        Ok(())
    }
}

fn apply_page<T>(mut items: Vec<T>, opts: Option<ListOptions>) -> Vec<T> {
    let Some(o) = opts else { return items };
    if o.offset >= items.len() {
        return vec![];
    }
    items.drain(..o.offset);
    items.truncate(o.limit);
    items
}
