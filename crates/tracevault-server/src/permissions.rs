use std::collections::HashSet;

// Re-export Permission enum from core
pub use tracevault_core::permissions::Permission;

/// Valid roles in the system.
pub const VALID_ROLES: &[&str] = &["owner", "admin", "policy_admin", "developer", "auditor"];

pub fn is_valid_role(role: &str) -> bool {
    VALID_ROLES.contains(&role)
}

pub fn role_permissions(role: &str) -> HashSet<Permission> {
    use Permission::*;
    match role {
        "owner" => HashSet::from([
            TracePush, TraceViewAll, TraceViewOwn, PolicyManage,
            AuditLogView, UserManage, OrgSettingsManage,
            ComplianceManage, ComplianceView,
            CodeBrowse, StoryGenerate, StoryView,
        ]),
        "admin" => HashSet::from([
            TracePush, TraceViewAll, TraceViewOwn, PolicyManage,
            AuditLogView, UserManage, OrgSettingsManage,
            ComplianceManage, ComplianceView,
            CodeBrowse, StoryGenerate, StoryView,
        ]),
        "policy_admin" => HashSet::from([
            TracePush, TraceViewAll, TraceViewOwn, PolicyManage,
            AuditLogView, ComplianceView,
            CodeBrowse, StoryGenerate, StoryView,
        ]),
        "developer" => HashSet::from([
            TracePush, TraceViewOwn,
            CodeBrowse, StoryGenerate, StoryView,
        ]),
        "auditor" => HashSet::from([
            TraceViewAll, TraceViewOwn, AuditLogView, ComplianceView,
            CodeBrowse, StoryView,
        ]),
        _ => HashSet::new(),
    }
}

pub fn has_permission(role: &str, perm: Permission) -> bool {
    role_permissions(role).contains(&perm)
}
