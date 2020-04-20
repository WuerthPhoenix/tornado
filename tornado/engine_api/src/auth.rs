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
