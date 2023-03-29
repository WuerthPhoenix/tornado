use crate::auth::{
    roles_contain_any_permission, AuthContextTrait, AuthService, Permission, WithOwner,
    FORBIDDEN_MISSING_REQUIRED_PERMISSIONS, JWT_TOKEN_HEADER_SUFFIX,
};
use crate::error::ApiError;
use actix_web::HttpRequest;
use base64::{engine::general_purpose::STANDARD as base64, Engine as _};
use log::*;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use tornado_engine_api_dto::auth_v2::{AuthHeaderV2, AuthV2};

pub const FORBIDDEN_NOT_OWNER: &str = "NOT_OWNER";

#[derive(Debug, Clone, PartialEq)]
pub struct AuthContextV2<'a> {
    pub auth: AuthV2,
    pub valid: bool,
    permission_roles_map: &'a BTreeMap<Permission, Vec<String>>,
}

impl AuthContextTrait for AuthContextV2<'_> {
    // Returns an error if the user is not the owner of the object
    fn is_owner<T: WithOwner>(&self, obj: &T) -> Result<&Self, ApiError> {
        self.is_authenticated()?;
        let owner = obj.get_owner_id();
        if self.auth.user == owner {
            Ok(self)
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

impl<'a> AuthContextV2<'a> {
    pub fn new(auth: AuthV2, permission_roles_map: &'a BTreeMap<Permission, Vec<String>>) -> Self {
        AuthContextV2 { valid: !auth.user.is_empty(), auth, permission_roles_map }
    }

    pub fn from_header(
        mut auth_header: AuthHeaderV2,
        auth_key: &str,
        permission_roles_map: &'a BTreeMap<Permission, Vec<String>>,
    ) -> Result<Self, ApiError> {
        let authorization =
            auth_header.auths.remove(auth_key).ok_or(ApiError::InvalidAuthKeyError {
                message: format!("Authentication header does not contain auth key: {}", auth_key),
            })?;
        let auth =
            AuthV2 { user: auth_header.user, authorization, preferences: auth_header.preferences };
        Ok(AuthContextV2 { valid: !auth.user.is_empty(), auth, permission_roles_map })
    }

    // Returns an error if user is not authenticated
    pub fn is_authenticated(&self) -> Result<&Self, ApiError> {
        if !self.valid {
            return Err(ApiError::UnauthenticatedError {});
        };
        Ok(self)
    }

    // Returns an error if user does not have the permission
    pub fn has_permission(&self, permission: &Permission) -> Result<&Self, ApiError> {
        self.has_any_permission(&[permission])
    }

    // Returns an error if user does not have at least one of the permissions
    pub fn has_any_permission(&self, permissions: &[&Permission]) -> Result<&Self, ApiError> {
        self.is_authenticated()?;

        if roles_contain_any_permission(
            self.permission_roles_map,
            &self.auth.authorization.roles,
            permissions,
        ) {
            Ok(self)
        } else {
            Err(ApiError::ForbiddenError {
                code: FORBIDDEN_MISSING_REQUIRED_PERMISSIONS.to_owned(),
                params: HashMap::new(),
                message: format!(
                    "User [{}] does not have the required permissions [{:?}]",
                    self.auth.user, permissions
                ),
            })
        }
    }
}

#[derive(Clone)]
pub struct AuthServiceV2 {
    pub permission_roles_map: Arc<BTreeMap<Permission, Vec<String>>>,
}

impl AuthServiceV2 {
    pub fn new(permission_roles_map: Arc<BTreeMap<Permission, Vec<String>>>) -> Self {
        Self { permission_roles_map }
    }

    pub fn auth_from_request(
        &self,
        req: &HttpRequest,
        auth_key: &str,
    ) -> Result<AuthContextV2, ApiError> {
        let auth_header = AuthService::token_string_from_request(req)
            .and_then(|token| Self::auth_header_from_token_string(token))?;
        let auth_ctx =
            AuthContextV2::from_header(auth_header, auth_key, &self.permission_roles_map)?;
        Ok(auth_ctx)
    }

    pub fn auth_header_from_token_string(token: &str) -> Result<AuthHeaderV2, ApiError> {
        let auth_str = AuthService::decode_token_from_base64(token)?;
        let auth_header =
            serde_json::from_str(&auth_str).map_err(|err| ApiError::InvalidTokenError {
                message: format!("Invalid JSON token content. Err: {:?}", err),
            })?;
        trace!("Auth header built from request: [{:?}]", auth_header);
        Ok(auth_header)
    }

    /// Generates the auth token
    fn auth_to_token_string(auth: &AuthHeaderV2) -> Result<String, ApiError> {
        let auth_str =
            serde_json::to_string(&auth).map_err(|err| ApiError::InternalServerError {
                cause: format!("Cannot serialize auth into string. Err: {:?}", err),
            })?;
        Ok(base64.encode(auth_str.as_bytes()))
    }

    pub fn auth_to_token_header(auth: &AuthHeaderV2) -> Result<String, ApiError> {
        Ok(format!("{}{}", JWT_TOKEN_HEADER_SUFFIX, AuthServiceV2::auth_to_token_string(auth)?))
    }
}

#[cfg(test)]
pub mod test {

    use super::*;
    use actix_web::http::header;
    use actix_web::test::TestRequest;
    use tornado_engine_api_dto::auth::UserPreferences;
    use tornado_engine_api_dto::auth_v2::Authorization;

    fn permission_map() -> BTreeMap<Permission, Vec<String>> {
        let mut permission_roles_map = BTreeMap::new();
        permission_roles_map.insert(Permission::ConfigEdit, vec!["edit".to_owned()]);
        permission_roles_map
            .insert(Permission::ConfigView, vec!["edit".to_owned(), "view".to_owned()]);
        permission_roles_map
            .insert(Permission::RuntimeConfigEdit, vec!["runtime_config_edit".to_owned()]);
        permission_roles_map
            .insert(Permission::RuntimeConfigView, vec!["runtime_config_view".to_owned()]);
        permission_roles_map
    }
    pub fn test_auth_service_v2() -> AuthServiceV2 {
        let permission_roles_map = permission_map();
        AuthServiceV2::new(Arc::new(permission_roles_map))
    }

    #[test]
    fn from_header_should_return_auth_v2_for_an_existing_auth_key() {
        // Arrange
        let mut auths = HashMap::new();
        auths.insert(
            "auth_key_0".to_owned(),
            Authorization { path: vec!["node".to_owned()], roles: vec!["edit".to_owned()] },
        );
        auths.insert(
            "auth_key_1".to_owned(),
            Authorization { path: vec!["root".to_owned()], roles: vec!["view".to_owned()] },
        );

        let auth_header = AuthHeaderV2 { user: "user".to_string(), auths, preferences: None };
        let auth_key = "auth_key_1";
        let permission_roles_map = BTreeMap::new();

        // Act
        let result =
            AuthContextV2::from_header(auth_header, auth_key, &permission_roles_map).unwrap();

        // Assert
        let expected = AuthContextV2 {
            auth: AuthV2 {
                user: "user".to_string(),
                authorization: Authorization {
                    path: vec!["root".to_owned()],
                    roles: vec!["view".to_owned()],
                },
                preferences: None,
            },
            valid: true,
            permission_roles_map: &permission_roles_map,
        };
        assert_eq!(result, expected);
    }

    #[test]
    fn from_header_should_return_non_valid_auth_for_empty_user() {
        // Arrange
        let mut auths = HashMap::new();
        auths.insert(
            "auth_key_0".to_owned(),
            Authorization { path: vec!["node".to_owned()], roles: vec!["edit".to_owned()] },
        );
        auths.insert(
            "auth_key_1".to_owned(),
            Authorization { path: vec!["root".to_owned()], roles: vec!["view".to_owned()] },
        );
        let auth_header = AuthHeaderV2 { user: "".to_string(), auths, preferences: None };
        let auth_key = "auth_key_1";
        let permission_roles_map = BTreeMap::new();

        // Act
        let result =
            AuthContextV2::from_header(auth_header, auth_key, &permission_roles_map).unwrap();

        // Assert
        let expected = AuthContextV2 {
            auth: AuthV2 {
                user: "".to_string(),
                authorization: Authorization {
                    path: vec!["root".to_owned()],
                    roles: vec!["view".to_owned()],
                },
                preferences: None,
            },
            valid: false,
            permission_roles_map: &permission_roles_map,
        };
        assert_eq!(result, expected);
    }

    #[test]
    fn is_authenticated_should_return_error_if_auth_is_not_valid() {
        // Arrange
        let auth_context = AuthContextV2 {
            auth: AuthV2 {
                user: "".to_string(),
                authorization: Authorization { path: vec![], roles: vec![] },
                preferences: None,
            },
            valid: false,
            permission_roles_map: &Default::default(),
        };

        // Act
        let result = auth_context.is_authenticated();

        // Assert
        assert!(result.is_err());
    }

    #[test]
    fn is_authenticated_should_return_ok_if_auth_is_valid() {
        // Arrange
        let auth_context = AuthContextV2 {
            auth: AuthV2 {
                user: "my_user".to_string(),
                authorization: Authorization { path: vec![], roles: vec![] },
                preferences: None,
            },
            valid: true,
            permission_roles_map: &Default::default(),
        };

        // Act
        let result = auth_context.is_authenticated();

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn has_permission_should_return_ok_or_error_if_user_has_or_does_not_have_permission() {
        // Arrange
        let auth_context = AuthContextV2 {
            auth: AuthV2 {
                user: "my_user".to_string(),
                authorization: Authorization { path: vec![], roles: vec!["view".to_owned()] },
                preferences: None,
            },
            valid: true,
            permission_roles_map: &permission_map(),
        };

        // Act & Assert
        assert!(auth_context.has_permission(&Permission::ConfigView).is_ok());
        assert!(auth_context.has_permission(&Permission::ConfigEdit).is_err());
    }

    #[test]
    fn has_permission_and_has_any_permission_should_return_err_if_auth_is_not_valid() {
        // Arrange
        let auth_context = AuthContextV2 {
            auth: AuthV2 {
                user: "".to_string(),
                authorization: Authorization { path: vec![], roles: vec!["view".to_owned()] },
                preferences: None,
            },
            valid: false,
            permission_roles_map: &permission_map(),
        };

        // Act & Assert
        assert!(auth_context.has_permission(&Permission::ConfigView).is_err());
        assert!(auth_context.has_any_permission(&[&Permission::ConfigView]).is_err());
    }

    #[test]
    fn has_any_permission_should_return_ok_or_error_if_user_has_or_does_not_have_any_permission() {
        // Arrange
        let auth_context = AuthContextV2 {
            auth: AuthV2 {
                user: "my_user".to_string(),
                authorization: Authorization { path: vec![], roles: vec!["view".to_owned()] },
                preferences: None,
            },
            valid: true,
            permission_roles_map: &permission_map(),
        };

        // Act & Assert
        assert!(auth_context.has_any_permission(&[&Permission::ConfigView]).is_ok());
        assert!(auth_context
            .has_any_permission(&[&Permission::ConfigEdit, &Permission::ConfigView])
            .is_ok());
        assert!(auth_context
            .has_any_permission(&[&Permission::ConfigEdit, &Permission::RuntimeConfigView])
            .is_err());
    }

    #[test]
    fn auth_header_from_token_string_should_return_parse_token() {
        // Arrange
        let header = r#"{
  "user": "mario",
  "auths": {
    "tenantA1": {
      "path": ["root"],
      "roles": ["view", "edit", "test_event_execute_actions"]
    },
    "tenantA2": {
      "path": ["root", "filter2", "tenantA"],
      "roles": ["view", "test_event_execute_actions"]
    }
  },
  "preferences": {
    "language": "en_US"
  }
}"#;
        let token = base64::encode(header);

        // Act
        let result = AuthServiceV2::auth_header_from_token_string(&token).unwrap();

        // Assert
        let mut auths = HashMap::new();
        auths.insert(
            "tenantA1".to_owned(),
            Authorization {
                path: vec!["root".to_owned()],
                roles: vec![
                    "view".to_owned(),
                    "edit".to_owned(),
                    "test_event_execute_actions".to_owned(),
                ],
            },
        );
        auths.insert(
            "tenantA2".to_owned(),
            Authorization {
                path: vec!["root".to_owned(), "filter2".to_owned(), "tenantA".to_owned()],
                roles: vec!["view".to_owned(), "test_event_execute_actions".to_owned()],
            },
        );
        let expected = AuthHeaderV2 {
            user: "mario".to_string(),
            auths,
            preferences: Some(UserPreferences { language: Some("en_US".to_owned()) }),
        };
        assert_eq!(result, expected);
    }

    #[test]
    fn auth_header_from_token_string_should_return_error_if_token_is_not_valid() {
        // Arrange
        let header = r#"{
  "user": "mario",
  "auths": {
    "tenantA1": {
      "roles": ["view", "edit", "test_event_execute_actions"]
    },
    "tenantA2": {
      "roles": ["view", "test_event_execute_actions"]
    }
  },
  "preferences": {
    "language": "en_US"
  }
}"#;
        let token = base64::encode(header);

        // Act
        let result = AuthServiceV2::auth_header_from_token_string(&token);

        // Assert
        assert!(result.is_err());
    }

    #[test]
    fn auth_from_request_should_build_auth_from_http_request() {
        let mut auths = HashMap::new();
        auths.insert(
            "auth1".to_owned(),
            Authorization { path: vec!["root".to_owned()], roles: vec!["view".to_owned()] },
        );
        // Arrange
        let permission_map = permission_map();
        let auth_service = AuthServiceV2::new(Arc::new(permission_map.clone()));
        let request = TestRequest::get()
            .insert_header((
                header::AUTHORIZATION,
                AuthServiceV2::auth_to_token_header(&AuthHeaderV2 {
                    user: "admin".to_string(),

                    auths,
                    preferences: None,
                })
                .unwrap(),
            ))
            .to_http_request();

        // Act
        let result = auth_service.auth_from_request(&request, "auth1").unwrap();

        // Assert
        let expected = AuthContextV2::new(
            AuthV2 {
                user: "admin".to_string(),
                authorization: Authorization {
                    path: vec!["root".to_owned()],
                    roles: vec!["view".to_owned()],
                },
                preferences: None,
            },
            &permission_map,
        );

        assert_eq!(result, expected)
    }

    #[test]
    fn should_be_the_owner() {
        let auth = AuthV2 {
            user: "USER_123".to_owned(),
            authorization: Authorization {
                path: vec![],
                roles: vec!["role1".to_owned(), "role2".to_owned()],
            },
            preferences: None,
        };
        let role_permissions = BTreeMap::new();
        let auth_context = AuthContextV2::new(auth, &role_permissions);

        assert!(auth_context
            .is_owner(&Ownable { owner_id: "USER_123".to_owned(), id: "abc".to_owned() })
            .is_ok());
    }

    #[test]
    fn should_not_be_the_owner() {
        let auth = AuthV2 {
            user: "USER_123".to_owned(),
            authorization: Authorization {
                path: vec![],
                roles: vec!["role1".to_owned(), "role2".to_owned()],
            },
            preferences: None,
        };
        let role_permissions = BTreeMap::new();
        let auth_context = AuthContextV2::new(auth, &role_permissions);

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
