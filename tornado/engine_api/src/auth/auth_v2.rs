use crate::auth::{AuthService, JWT_TOKEN_HEADER_SUFFIX, Permission};
use crate::error::ApiError;
use actix_web::HttpRequest;
use log::*;
use std::collections::BTreeMap;
use std::sync::Arc;
use tornado_engine_api_dto::auth_v2::AuthV2;

#[derive(Debug, Clone)]
pub struct AuthContextV2<'a> {
    pub auth: AuthV2,
    pub valid: bool,
    permission_roles_map: &'a BTreeMap<Permission, Vec<String>>,
}

impl<'a> AuthContextV2<'a> {
    pub fn new(auth: AuthV2, permission_roles_map: &'a BTreeMap<Permission, Vec<String>>) -> Self {
        AuthContextV2 { valid: !auth.user.is_empty(), auth, permission_roles_map }
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

    pub fn auth_from_request(&self, req: &HttpRequest) -> Result<AuthContextV2, ApiError> {
        AuthService::token_string_from_request(req)
            .and_then(|token| self.auth_from_token_string(token))
    }

    pub fn auth_from_token_string(&self, token: &str) -> Result<AuthContextV2, ApiError> {
        let auth_str = AuthService::decode_token_from_base64(token)?;
        let auth = serde_json::from_str(&auth_str).map_err(|err| ApiError::InvalidTokenError {
            message: format!("Invalid JSON token content. Err: {:?}", err),
        })?;
        trace!("Auth built from request: [{:?}]", auth);
        Ok(AuthContextV2::new(auth, &self.permission_roles_map))
    }

    /// Generates the auth token
    fn auth_to_token_string(auth: &AuthV2) -> Result<String, ApiError> {
        let auth_str =
            serde_json::to_string(&auth).map_err(|err| ApiError::InternalServerError {
                cause: format!("Cannot serialize auth into string. Err: {:?}", err),
            })?;
        Ok(base64::encode(auth_str.as_bytes()))
    }

    pub fn auth_to_token_header(auth: &AuthV2) -> Result<String, ApiError> {
        Ok(format!("{}{}", JWT_TOKEN_HEADER_SUFFIX, AuthServiceV2::auth_to_token_string(auth)?))
    }
}
