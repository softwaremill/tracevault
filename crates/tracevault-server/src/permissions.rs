use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Permission {
    TracePush,
    TraceViewAll,
    TraceViewOwn,
    PolicyManage,
    AuditLogView,
    UserManage,
    OrgSettingsManage,
    ComplianceManage,
    ComplianceView,
}

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
        ]),
        "admin" => HashSet::from([
            TracePush, TraceViewAll, TraceViewOwn, PolicyManage,
            AuditLogView, UserManage, OrgSettingsManage,
            ComplianceManage, ComplianceView,
        ]),
        "policy_admin" => HashSet::from([
            TracePush, TraceViewAll, TraceViewOwn, PolicyManage,
            AuditLogView, ComplianceView,
        ]),
        "developer" => HashSet::from([
            TracePush, TraceViewOwn,
        ]),
        "auditor" => HashSet::from([
            TraceViewAll, TraceViewOwn, AuditLogView, ComplianceView,
        ]),
        _ => HashSet::new(),
    }
}

pub fn has_permission(role: &str, perm: Permission) -> bool {
    role_permissions(role).contains(&perm)
}
