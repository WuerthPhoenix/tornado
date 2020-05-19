use crate::auth::Permission;
use tornado_engine_api_dto::auth::{AuthWithPermissionsDto, PermissionDto};

pub fn to_auth_with_permissions_dto(
    user: String,
    permissions: &[&Permission],
) -> AuthWithPermissionsDto {
    AuthWithPermissionsDto {
        user,
        permissions: permissions
            .iter()
            .map(|permission| match permission {
                Permission::ConfigEdit => PermissionDto::ConfigEdit,
                Permission::ConfigView => PermissionDto::ConfigView,
            })
            .collect(),
    }
}
