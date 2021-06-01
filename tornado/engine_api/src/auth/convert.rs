use crate::auth::Permission;
use tornado_engine_api_dto::auth::{AuthWithPermissionsDto, PermissionDto, UserPreferences};

pub fn to_auth_with_permissions_dto(
    user: String,
    preferences: Option<UserPreferences>,
    permissions: &[&Permission],
) -> AuthWithPermissionsDto {
    AuthWithPermissionsDto {
        user,
        preferences,
        permissions: permissions
            .iter()
            .map(|permission| match permission {
                Permission::ConfigEdit => PermissionDto::ConfigEdit,
                Permission::ConfigView => PermissionDto::ConfigView,
                Permission::RuntimeConfigEdit => PermissionDto::RuntimeConfigEdit,
                Permission::RuntimeConfigView => PermissionDto::RuntimeConfigView,
            })
            .collect(),
    }
}
