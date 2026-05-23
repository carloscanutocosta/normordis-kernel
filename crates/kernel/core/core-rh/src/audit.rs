//! Bridge para `core-audit`: converte um `UserProfile` num `AuditActor`.

use crate::UserProfile;

pub fn audit_actor_from_user(user: &UserProfile) -> core_audit::AuditActor {
    core_audit::AuditActor::with_metadata(
        user.user_id.as_str(),
        Some(user.display_name.clone()),
        Some("user".to_owned()),
    )
    .expect("valid core-rh user profiles must map to valid audit actors")
}
