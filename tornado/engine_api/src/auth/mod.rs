pub mod convert;
pub mod web;

use crate::error::ApiError;
use actix_web::HttpRequest;
use log::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use tornado_engine_api_dto::auth::Auth;
use tornado_engine_matcher::config::MatcherConfigDraft;

pub const JWT_TOKEN_HEADER: &str = "Authorization";
pub const JWT_TOKEN_HEADER_SUFFIX: &str = "Bearer ";
pub const JWT_TOKEN_HEADER_SUFFIX_LEN: usize = JWT_TOKEN_HEADER_SUFFIX.len();

pub const FORBIDDEN_NOT_OWNER: &str = "NOT_OWNER";
pub const FORBIDDEN_MISSING_REQUIRED_PERMISSIONS: &str = "MISSING_REQUIRED_PERMISSIONS";

pub trait WithOwner {
    fn get_id(&self) -> &str;
    fn get_owner_id(&self) -> &str;
}

#[derive(Debug, PartialEq, Eq, Clone, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Permission {
    ConfigEdit,
    ConfigView,
}

#[derive(Debug)]
pub struct AuthContext<'a> {
    pub auth: Auth,
    pub valid: bool,
    permission_roles_map: &'a BTreeMap<Permission, Vec<String>>,
}

impl<'a> AuthContext<'a> {
    pub fn new(auth: Auth, permission_roles_map: &'a BTreeMap<Permission, Vec<String>>) -> Self {
        AuthContext { valid: !auth.user.is_empty(), auth, permission_roles_map }
    }

    // Returns an error if user is not authenticated
    pub fn is_authenticated(&self) -> Result<&AuthContext, ApiError> {
        if !self.valid {
            return Err(ApiError::UnauthenticatedError {});
        };
        Ok(&self)
    }

    // Returns an error if user does not have the permission
    pub fn has_permission(&self, permission: &Permission) -> Result<&AuthContext, ApiError> {
        self.has_any_permission(&[permission])
    }

    // Returns an error if user does not have at least one of the permissions
    pub fn has_any_permission(
        &self,
        permissions: &[&Permission],
    ) -> Result<&AuthContext, ApiError> {
        self.is_authenticated()?;

        for permission in permissions {
            if let Some(roles_with_permission) = self.permission_roles_map.get(permission) {
                for user_role in &self.auth.roles {
                    if roles_with_permission.contains(user_role) {
                        return Ok(&self);
                    }
                }
            }
        }
        Err(ApiError::ForbiddenError {
            code: FORBIDDEN_MISSING_REQUIRED_PERMISSIONS.to_owned(),
            params: HashMap::new(),
            message: format!(
                "User [{}] does not have the required permissions [{:?}]",
                self.auth.user, permissions
            ),
        })
    }

    // Returns an error if user does not have the permission
    pub fn get_permissions(&self) -> Vec<&Permission> {
        let mut permissions = vec![];

        if self.is_authenticated().is_ok() {
            for permission in self.permission_roles_map.keys() {
                if self.has_permission(permission).is_ok() {
                    permissions.push(permission);
                }
            }
        }

        permissions
    }

    // Returns an error if the user is not the owner of the object
    pub fn is_owner<T: WithOwner>(&self, obj: &T) -> Result<&AuthContext, ApiError> {
        self.is_authenticated()?;
        let owner = obj.get_owner_id();
        if self.auth.user == owner {
            Ok(&self)
        } else {
            let mut params = HashMap::new();
            params.insert("OWNER".to_owned(), owner.to_owned());
            params.insert("ID".to_owned(), obj.get_id().to_owned());
            Err(ApiError::ForbiddenError {
                code: FORBIDDEN_NOT_OWNER.to_owned(),
                params,
                message: format!(
                    "User [{}] is not the owner of the object. The owner is [{}]",
                    self.auth.user, owner
                ),
            })
        }
    }
}

// Reverts a role->permissions map to a permission->roles map
pub fn roles_map_to_permissions_map(
    role_permissions: BTreeMap<String, Vec<Permission>>,
) -> BTreeMap<Permission, Vec<String>> {
    let mut result = BTreeMap::new();
    for (role, permissions) in role_permissions {
        for permission in permissions {
            result.entry(permission).or_insert_with(|| vec![]).push(role.to_owned())
        }
    }
    result
}

#[derive(Clone)]
pub struct AuthService {
    pub permission_roles_map: Arc<BTreeMap<Permission, Vec<String>>>,
}

impl AuthService {
    pub fn new(permission_roles_map: Arc<BTreeMap<Permission, Vec<String>>>) -> Self {
        Self { permission_roles_map }
    }

    pub fn token_string_from_request<'a>(&self, req: &'a HttpRequest) -> Result<&'a str, ApiError> {
        if let Some(header) = req.headers().get(JWT_TOKEN_HEADER) {
            return header
                .to_str()
                .map_err(|err| ApiError::ParseAuthHeaderError { message: format!("{}", err) })
                .and_then(|header| {
                    trace!("Token found in request: [{}]", header);
                    if header.len() > JWT_TOKEN_HEADER_SUFFIX_LEN {
                        Ok(&header[JWT_TOKEN_HEADER_SUFFIX_LEN..])
                    } else {
                        Err(ApiError::ParseAuthHeaderError {
                            message: format!("Unexpected auth header: {}", header),
                        })
                    }
                });
        };
        Err(ApiError::MissingAuthTokenError)
    }

    pub fn auth_from_request(&self, req: &HttpRequest) -> Result<AuthContext, ApiError> {
        self.token_string_from_request(req).and_then(|token| self.auth_from_token_string(token))
    }

    pub fn auth_from_token_string(&self, token: &str) -> Result<AuthContext, ApiError> {
        let auth_vec = base64::decode(token).map_err(|err| ApiError::InvalidTokenError {
            message: format!("Cannot perform base64::decode of auth token. Err: {}", err),
        })?;
        let auth_str = String::from_utf8(auth_vec).map_err(|err| ApiError::InvalidTokenError {
            message: format!("Invalid UTF8 token content. Err: {}", err),
        })?;
        let auth = serde_json::from_str(&auth_str).map_err(|err| ApiError::InvalidTokenError {
            message: format!("Invalid JSON token content. Err: {}", err),
        })?;
        trace!("Auth built from request: [{:?}]", auth);
        Ok(AuthContext::new(auth, &self.permission_roles_map))
    }

    /// Generates the auth token
    pub fn auth_to_token_string(auth: &Auth) -> Result<String, ApiError> {
        let auth_str =
            serde_json::to_string(&auth).map_err(|err| ApiError::InternalServerError {
                cause: format!("Cannot serialize auth into string. Err: {}", err),
            })?;
        Ok(base64::encode(auth_str.as_bytes()))
    }

    /// Generates the auth HTTP header in the form:
    /// Bearer: <TOKEN>
    pub fn auth_to_token_header(auth: &Auth) -> Result<String, ApiError> {
        Ok(format!("{}{}", JWT_TOKEN_HEADER_SUFFIX, AuthService::auth_to_token_string(&auth)?))
    }
}

impl WithOwner for MatcherConfigDraft {
    fn get_id(&self) -> &str {
        &self.data.draft_id
    }
    fn get_owner_id(&self) -> &str {
        &self.data.user
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn auth_service_should_create_base64_token() -> Result<(), ApiError> {
        // Arrange
        let auth = Auth {
            user: "12456abc".to_owned(),
            roles: vec!["role_a".to_owned(), "role_b".to_owned()],
        };

        let expected_token = "eyJ1c2VyIjoiMTI0NTZhYmMiLCJyb2xlcyI6WyJyb2xlX2EiLCJyb2xlX2IiXX0=";

        // Act
        let token = AuthService::auth_to_token_string(&auth)?;

        // Assert
        assert_eq!(expected_token, token);

        Ok(())
    }

    #[test]
    fn auth_service_should_create_authorization_token() -> Result<(), ApiError> {
        // Arrange
        let auth = Auth {
            user: "12456abc".to_owned(),
            roles: vec!["role_a".to_owned(), "role_b".to_owned()],
        };

        let expected_token_header =
            "Bearer eyJ1c2VyIjoiMTI0NTZhYmMiLCJyb2xlcyI6WyJyb2xlX2EiLCJyb2xlX2IiXX0=";

        // Act
        let token_header = AuthService::auth_to_token_header(&auth)?;

        // Assert
        assert_eq!(expected_token_header, token_header);

        Ok(())
    }

    #[test]
    fn auth_service_should_decode_base64_token() -> Result<(), ApiError> {
        // Arrange
        let expected_auth = Auth {
            user: "12456abc".to_owned(),
            roles: vec!["role_a".to_owned(), "role_b".to_owned()],
        };

        let token = AuthService::auth_to_token_string(&expected_auth)?;

        let mut permission_roles_map = BTreeMap::new();
        permission_roles_map.insert(Permission::ConfigEdit, vec!["role_a".to_owned()]);

        let auth_service = AuthService::new(Arc::new(permission_roles_map.clone()));

        // Act
        let auth_context = auth_service.auth_from_token_string(&token)?;

        // Assert
        assert_eq!(expected_auth, auth_context.auth);
        assert_eq!(&permission_roles_map, auth_context.permission_roles_map);
        assert!(auth_context.is_authenticated().is_ok());
        assert!(auth_context.has_permission(&Permission::ConfigEdit).is_ok());
        assert!(auth_context.has_permission(&Permission::ConfigView).is_err());

        Ok(())
    }

    #[test]
    fn auth_service_should_return_error_for_wrong_base64_token() -> Result<(), ApiError> {
        // Arrange
        let token = "MickeyMouseLovesMinnie";
        let auth_service = AuthService::new(Arc::new(BTreeMap::new()));

        // Act
        let result = auth_service.auth_from_token_string(token);

        // Assert
        assert!(result.is_err());

        Ok(())
    }

    #[test]
    fn auth_context_should_be_valid() {
        let auth = Auth { user: "username".to_owned(), roles: vec![] };
        let permission_roles_map = BTreeMap::new();
        let auth_context = AuthContext::new(auth, &permission_roles_map);

        assert!(auth_context.valid);
        assert!(auth_context.is_authenticated().is_ok());
    }

    #[test]
    fn auth_context_should_be_not_valid_if_missing_username() {
        let auth = Auth { user: "".to_owned(), roles: vec![] };
        let permission_roles_map = BTreeMap::new();
        let auth_context = AuthContext::new(auth, &permission_roles_map);

        assert!(!auth_context.valid);
        assert!(auth_context.is_authenticated().is_err());
    }

    #[test]
    fn auth_context_should_return_whether_user_has_permissions() -> Result<(), ApiError> {
        let auth =
            Auth { user: "user".to_owned(), roles: vec!["role1".to_owned(), "role2".to_owned()] };

        let mut permission_roles_map = BTreeMap::new();
        permission_roles_map.insert(Permission::ConfigEdit, vec!["role1".to_owned()]);
        permission_roles_map
            .insert(Permission::ConfigView, vec!["role1".to_owned(), "role2".to_owned()]);

        let auth_context = AuthContext::new(auth, &permission_roles_map);

        assert!(auth_context.valid);
        assert!(auth_context.is_authenticated().is_ok());
        assert!(auth_context.has_permission(&Permission::ConfigEdit).is_ok());
        assert!(auth_context.has_permission(&Permission::ConfigView).is_ok());
        assert!(auth_context
            .has_permission(&Permission::ConfigEdit)?
            .has_permission(&Permission::ConfigView)
            .is_ok());

        Ok(())
    }

    #[test]
    fn auth_context_should_return_whether_user_has_any_permissions() -> Result<(), ApiError> {
        let auth = Auth { user: "user".to_owned(), roles: vec!["role1".to_owned()] };

        let mut permission_roles_map = BTreeMap::new();
        permission_roles_map.insert(Permission::ConfigEdit.to_owned(), vec!["role1".to_owned()]);
        permission_roles_map.insert(Permission::ConfigView.to_owned(), vec!["role2".to_owned()]);

        let auth_context = AuthContext::new(auth, &permission_roles_map);

        assert!(auth_context.valid);
        assert!(auth_context.is_authenticated().is_ok());
        assert!(auth_context
            .has_any_permission(&[&Permission::ConfigEdit, &Permission::ConfigEdit])
            .is_ok());
        assert!(auth_context
            .has_any_permission(&[&Permission::ConfigView, &Permission::ConfigEdit])
            .is_ok());
        assert!(auth_context
            .has_any_permission(&[&Permission::ConfigEdit, &Permission::ConfigView])
            .is_ok());
        assert!(auth_context
            .has_any_permission(&[&Permission::ConfigView, &Permission::ConfigView])
            .is_err());

        match &auth_context.has_any_permission(&[&Permission::ConfigView, &Permission::ConfigView])
        {
            Err(ApiError::ForbiddenError { code, .. }) => {
                assert_eq!(FORBIDDEN_MISSING_REQUIRED_PERMISSIONS, code)
            }
            _ => assert!(false),
        }

        Ok(())
    }

    #[test]
    fn invalid_auth_context_should_never_have_permissions() -> Result<(), ApiError> {
        let auth =
            Auth { user: "".to_owned(), roles: vec!["role1".to_owned(), "role2".to_owned()] };
        let mut permission_roles_map = BTreeMap::new();
        permission_roles_map.insert(Permission::ConfigView, vec!["role1".to_owned()]);

        let auth_context = AuthContext::new(auth, &permission_roles_map);

        assert!(!auth_context.valid);
        assert!(auth_context.is_authenticated().is_err());
        assert!(auth_context.has_permission(&Permission::ConfigView).is_err());
        assert!(auth_context.has_permission(&Permission::ConfigEdit).is_err());

        Ok(())
    }

    #[test]
    fn should_return_all_user_permissions() -> Result<(), ApiError> {
        let mut permission_roles_map = BTreeMap::new();
        permission_roles_map.insert(
            Permission::ConfigEdit.to_owned(),
            vec!["role1".to_owned(), "role3".to_owned()],
        );
        permission_roles_map.insert(
            Permission::ConfigView.to_owned(),
            vec!["role2".to_owned(), "role3".to_owned()],
        );

        {
            let auth = Auth { user: "user".to_owned(), roles: vec!["role1".to_owned()] };
            let auth_context = AuthContext::new(auth, &permission_roles_map);
            assert_eq!(vec![&Permission::ConfigEdit], auth_context.get_permissions());
        }

        {
            let auth = Auth { user: "user".to_owned(), roles: vec!["role2".to_owned()] };
            let auth_context = AuthContext::new(auth, &permission_roles_map);
            assert_eq!(vec![&Permission::ConfigView], auth_context.get_permissions());
        }

        {
            let auth = Auth {
                user: "user".to_owned(),
                roles: vec!["role1".to_owned(), "role2".to_owned()],
            };
            let auth_context = AuthContext::new(auth, &permission_roles_map);
            assert_eq!(
                vec![&Permission::ConfigEdit, &Permission::ConfigView],
                auth_context.get_permissions()
            );
        }

        {
            let auth = Auth { user: "user".to_owned(), roles: vec!["role3".to_owned()] };
            let auth_context = AuthContext::new(auth, &permission_roles_map);
            assert_eq!(
                vec![&Permission::ConfigEdit, &Permission::ConfigView],
                auth_context.get_permissions()
            );
        }

        {
            let auth = Auth { user: "user".to_owned(), roles: vec!["role4".to_owned()] };
            let auth_context = AuthContext::new(auth, &permission_roles_map);
            assert!(auth_context.get_permissions().is_empty());
        }

        {
            let auth = Auth { user: "user".to_owned(), roles: vec![] };
            let auth_context = AuthContext::new(auth, &permission_roles_map);
            assert!(auth_context.get_permissions().is_empty());
        }

        Ok(())
    }

    #[test]
    fn invalid_auth_context_should_return_empty_all_permissions() -> Result<(), ApiError> {
        let auth =
            Auth { user: "".to_owned(), roles: vec!["role1".to_owned(), "role2".to_owned()] };
        let mut permission_roles_map = BTreeMap::new();
        permission_roles_map.insert(Permission::ConfigView, vec!["role1".to_owned()]);

        let auth_context = AuthContext::new(auth, &permission_roles_map);

        assert!(auth_context.is_authenticated().is_err());
        assert!(auth_context.get_permissions().is_empty());

        Ok(())
    }

    #[test]
    fn should_create_a_permission_roles_map() {
        // Arrange
        let mut role_permissions = BTreeMap::new();
        role_permissions
            .insert("ROLE_1".to_owned(), vec![Permission::ConfigEdit, Permission::ConfigView]);
        role_permissions.insert("ROLE_2".to_owned(), vec![Permission::ConfigEdit]);
        role_permissions.insert("ROLE_3".to_owned(), vec![Permission::ConfigView]);

        let mut permission_roles = BTreeMap::new();
        permission_roles
            .insert(Permission::ConfigEdit, vec!["ROLE_1".to_owned(), "ROLE_2".to_owned()]);
        permission_roles
            .insert(Permission::ConfigView, vec!["ROLE_1".to_owned(), "ROLE_3".to_owned()]);

        // Act
        let result = roles_map_to_permissions_map(role_permissions);

        // Assert
        assert_eq!(permission_roles, result)
    }

    #[test]
    fn should_be_the_owner() {
        let auth = Auth {
            user: "USER_123".to_owned(),
            roles: vec!["role1".to_owned(), "role2".to_owned()],
        };
        let role_permissions = BTreeMap::new();
        let auth_context = AuthContext::new(auth, &role_permissions);

        assert!(auth_context
            .is_owner(&Ownable { owner_id: "USER_123".to_owned(), id: "abc".to_owned() })
            .is_ok());
    }

    #[test]
    fn should_not_be_the_owner() {
        let auth = Auth {
            user: "USER_123".to_owned(),
            roles: vec!["role1".to_owned(), "role2".to_owned()],
        };
        let role_permissions = BTreeMap::new();
        let auth_context = AuthContext::new(auth, &role_permissions);

        let result = auth_context
            .is_owner(&Ownable { owner_id: "USER_567".to_owned(), id: "abc".to_owned() });
        assert!(result.is_err());

        match &result {
            Err(ApiError::ForbiddenError { code, params, .. }) => {
                assert_eq!(FORBIDDEN_NOT_OWNER, code);
                assert_eq!(2, params.len());
                assert_eq!("USER_567", params["OWNER"]);
                assert_eq!("abc", params["ID"]);
            }
            _ => assert!(false),
        }
    }

    struct Ownable {
        id: String,
        owner_id: String,
    }

    impl WithOwner for Ownable {
        fn get_id(&self) -> &str {
            &self.id
        }

        fn get_owner_id(&self) -> &str {
            &self.owner_id
        }
    }
}
