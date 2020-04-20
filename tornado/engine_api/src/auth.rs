use actix_web::HttpRequest;
use crate::error::ApiError;
use tornado_engine_api_dto::auth::Auth;
use log::*;
use std::collections::HashMap;

pub const JWT_TOKEN_HEADER: &str = "Authorization";
pub const JWT_TOKEN_HEADER_SUFFIX: &str = "Bearer ";
pub const JWT_TOKEN_HEADER_SUFFIX_LEN: usize = JWT_TOKEN_HEADER_SUFFIX.len();

pub struct AuthContext<'a> {
    pub auth: Auth,
    pub valid: bool,
    permission_roles_map: &'a HashMap<String, Vec<String>>,
}

impl <'a> AuthContext<'a> {

    pub fn new(auth: Auth, permission_roles_map: &'a HashMap<String, Vec<String>>) -> Self {
        AuthContext{
            valid: !auth.user.is_empty(),
            auth,
            permission_roles_map,
        }
    }

    pub fn is_authenticated(&self) -> Result<&AuthContext, ApiError> {
        if !self.valid {
            return Err(ApiError::UnauthenticatedError {});
        };
        Ok(&self)
    }

    pub fn has_permission(&self, permission: &str) -> Result<&AuthContext, ApiError> {
        self.is_authenticated()?;

        if let Some(roles_with_permission) = self.permission_roles_map.get(permission) {
            for user_role in &self.auth.roles {
                if roles_with_permission.contains(user_role) {
                    return Ok(&self);
                }
            }
        }
        Err(ApiError::ForbiddenError {
                message: format!(
                    "User [{}] does not have the required permission [{}]",
                    self.auth.user, permission
                ),
            })
        }

}

#[derive(Clone)]
pub struct AuthService {
    pub permission_roles_map: HashMap<String, Vec<String>>
}

impl AuthService {
    pub fn new(permission_roles_map: HashMap<String, Vec<String>>) -> Self {
        Self {
            permission_roles_map
        }
    }

    pub fn token_string_from_request<'a>(
        &self,
        req: &'a HttpRequest,
    ) -> Result<&'a str, ApiError> {
        if let Some(header) = req.headers().get(JWT_TOKEN_HEADER) {
            return header
                .to_str()
                .map_err(|err| ApiError::ParseAuthHeaderError {
                    message: format!("{}", err),
                })
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
        self.token_string_from_request(req)
            .and_then(|token| self.auth_from_token_string(token))
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

}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn auth_service_should_decode_base64_token() -> Result<(), ApiError> {
        // Arrange
        let expected_auth = Auth {
            user: "12456abc".to_owned(),
            roles: vec!["role_a".to_owned(), "role_b".to_owned()]
        };

        let token = "ewogInVzZXIiOiAiMTI0NTZhYmMiLAogInJvbGVzIjogWyJyb2xlX2EiLCAicm9sZV9iIl0KfQ==";

        let mut permission_roles_map = HashMap::new();
        permission_roles_map.insert("EDIT".to_owned(), vec!["role_a".to_owned()]);

        let auth_service = AuthService::new(permission_roles_map.clone());

        // Act
        let auth_context = auth_service.auth_from_token_string(token)?;

        // Assert
        assert_eq!(expected_auth, auth_context.auth);
        assert_eq!(&permission_roles_map, auth_context.permission_roles_map);
        assert!(auth_context.is_authenticated().is_ok());
        assert!(auth_context.has_permission("EDIT").is_ok());
        assert!(auth_context.has_permission("VIEW").is_err());

        Ok(())
    }

    #[test]
    fn auth_service_should_return_error_for_wrong_base64_token() -> Result<(), ApiError> {
        // Arrange
        let token = "MickeyMouseLovesMinnie";
        let auth_service = AuthService::new(HashMap::new());

        // Act
        let result = auth_service.auth_from_token_string(token);

        // Assert
        assert!(result.is_err());

        Ok(())
    }

    #[test]
    fn auth_context_should_be_valid() {
        let auth = Auth {
            user: "username".to_owned(),
            roles: vec![]
        };
        let permission_roles_map = HashMap::new();
        let auth_context = AuthContext::new(auth, &permission_roles_map);

        assert!(auth_context.valid);
        assert!(auth_context.is_authenticated().is_ok());

    }

    #[test]
    fn auth_context_should_be_not_valid_if_missing_username() {
        let auth = Auth {
            user: "".to_owned(),
            roles: vec![]
        };
        let permission_roles_map = HashMap::new();
        let auth_context = AuthContext::new(auth, &permission_roles_map);

        assert!(!auth_context.valid);
        assert!(auth_context.is_authenticated().is_err());

    }

    #[test]
    fn auth_context_should_return_whether_user_has_permissions() -> Result<(), ApiError> {
        let auth = Auth {
            user: "user".to_owned(),
            roles: vec!["role1".to_owned(), "role2".to_owned()]
        };
        let mut permission_roles_map = HashMap::new();
        permission_roles_map.insert("EDIT".to_owned(), vec!["role1".to_owned()]);
        permission_roles_map.insert("VIEW".to_owned(), vec!["role1".to_owned(), "role2".to_owned()]);
        permission_roles_map.insert("ADMIN".to_owned(), vec!["role3".to_owned()]);

        let auth_context = AuthContext::new(auth, &permission_roles_map);

        assert!(auth_context.valid);
        assert!(auth_context.is_authenticated().is_ok());
        assert!(auth_context.has_permission("EDIT").is_ok());
        assert!(auth_context.has_permission("VIEW").is_ok());
        assert!(auth_context.has_permission("EDIT")?.has_permission("VIEW").is_ok());
        assert!(auth_context.has_permission("ADMIN").is_err());

        Ok(())
    }

    #[test]
    fn invalid_auth_context_should_never_have_permissions() -> Result<(), ApiError> {
        let auth = Auth {
            user: "".to_owned(),
            roles: vec!["role1".to_owned(), "role2".to_owned()]
        };
        let mut permission_roles_map = HashMap::new();
        permission_roles_map.insert("EDIT".to_owned(), vec!["role1".to_owned()]);

        let auth_context = AuthContext::new(auth, &permission_roles_map);

        assert!(!auth_context.valid);
        assert!(auth_context.is_authenticated().is_err());
        assert!(auth_context.has_permission("EDIT").is_err());
        assert!(auth_context.has_permission("VIEW").is_err());
        assert!(auth_context.has_permission("ADMIN").is_err());

        Ok(())
    }

}