use crate::auth::{AuthService, JWT_TOKEN_HEADER_SUFFIX, Permission};
use crate::error::ApiError;
use actix_web::HttpRequest;
use log::*;
use std::collections::BTreeMap;
use std::sync::Arc;
use tornado_engine_api_dto::auth_v2::{AuthV2, AuthHeaderV2};

#[derive(Debug, Clone)]
pub struct AuthContextV2<'a> {
    pub auth: AuthV2,
    pub valid: bool,
    permission_roles_map: &'a BTreeMap<Permission, Vec<String>>,
}

impl<'a> AuthContextV2<'a> {
    pub fn from_header(mut auth_header: AuthHeaderV2, auth_key: &str, permission_roles_map: &'a BTreeMap<Permission, Vec<String>>) -> Result<Self, ApiError> {
        let auth_instance = auth_header.auths.remove(auth_key).ok_or(
            Err(ApiError::InvalidAuthKeyError { message: format!("Authentication header does not contain auth key: {}", auth_key) })
        )?;
        let auth = AuthV2 {
            user: auth_header.user,
            auth_instance,
            preferences: auth_header.preferences
        };
        Ok(AuthContextV2 { valid: !auth.user.is_empty(), auth, permission_roles_map })
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

    pub fn auth_from_request(&self, req: &HttpRequest, auth_key: &str) -> Result<AuthContextV2, ApiError> {
        let auth_header = AuthService::token_string_from_request(req)
            .and_then(|token| self.auth_header_from_token_string(token))?;
        let auth_ctx = AuthContextV2::from_header(auth_header, auth_key, &self.permission_roles_map)?;
        Ok(auth_ctx)
    }

    pub fn auth_header_from_token_string(&self, token: &str) -> Result<AuthHeaderV2, ApiError> {
        let auth_str = AuthService::decode_token_from_base64(token)?;
        let auth_header = serde_json::from_str(&auth_str).map_err(|err| ApiError::InvalidTokenError {
            message: format!("Invalid JSON token content. Err: {:?}", err),
        })?;
        trace!("Auth header built from request: [{:?}]", auth_header);
        Ok(auth_header)
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

#[cfg(test)]
pub mod test {
    use super::*;

    #[test]
    fn auth_from_request_should_keep_only_auth_passed(){
        unimplemented!()
    }

    #[test]
    fn auth_from_request_should_keep_all_auths_if_none_is_passed(){
        unimplemented!()
    }
}