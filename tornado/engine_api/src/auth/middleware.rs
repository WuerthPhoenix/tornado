use crate::auth::auth_v2::{AuthServiceV2, FORBIDDEN_NOT_OWNER};
use crate::auth::{AuthContextTrait, Permission, WithOwner};
use crate::error::ApiError;
use actix_web::dev::Payload;
use actix_web::web::Data;
use actix_web::{FromRequest, HttpRequest};
use futures_util::future::{ready, Ready};
use log::warn;
use std::collections::HashMap;
use std::marker::PhantomData;

macro_rules! implement_authorization {
    ($auth:ident, $permission:expr) => {
        pub struct $auth;

        impl FromRequest for AuthorizedPath<$auth> {
            type Error = ApiError;
            type Future = Ready<Result<Self, Self::Error>>;

            fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
                ready(from_request(req, &[&$permission]))
            }
        }
    };
}

pub struct AuthorizedPath<Auth> {
    auth: PhantomData<Auth>,
    user: String,
    path: Vec<String>,
}

impl<T> AuthorizedPath<T> {
    pub fn user(&self) -> String {
        self.user.clone()
    }

    pub fn path(&self) -> Vec<&str> {
        self.path.iter().map(String::as_str).collect()
    }

    #[cfg(test)]
    pub fn new(user: String, path: Vec<String>) -> Self {
        Self { auth: Default::default(), user, path }
    }
}

impl<Auth: Send + Sync> AuthContextTrait for &AuthorizedPath<Auth> {
    fn is_owner<T: WithOwner>(&self, obj: &T) -> Result<&Self, ApiError> {
        let owner = obj.get_owner_id();

        if self.user.as_str() != owner {
            let mut params = HashMap::new();
            params.insert("OWNER".to_owned(), owner.to_owned());
            params.insert("ID".to_owned(), obj.get_id().to_owned());
            return Err(ApiError::ForbiddenError {
                code: FORBIDDEN_NOT_OWNER.to_owned(),
                params,
                message: format!(
                    "User [{}] is not the owner of the object. The owner is [{}]",
                    self.user, owner
                ),
            });
        }

        Ok(self)
    }
}

fn from_request<T>(
    req: &HttpRequest,
    permissions: &[&Permission],
) -> Result<AuthorizedPath<T>, ApiError> {
    let Some(auth_service) = req.app_data::<Data<AuthServiceV2>>() else {
        return Err(ApiError::InternalServerError { cause: "AuthServiceV2 was not mounted. This is a bug!".to_string() });
    };

    let Some(param_auth) = req.match_info().get("param_auth") else {
        return Err(ApiError::UnauthenticatedError);
    };

    let auth_context = auth_service.auth_from_request(req, param_auth)?;
    auth_context.has_any_permission(permissions)?;
    auth_context.is_authenticated()?;
    let user = auth_context.auth.user;
    let base_path = auth_context.auth.authorization.path;

    match req.match_info().get("node_path") {
        None => Ok(AuthorizedPath { auth: Default::default(), user, path: base_path }),
        Some(node_path) => Ok(AuthorizedPath {
            auth: Default::default(),
            user,
            path: join_path(base_path, node_path)?,
        }),
    }
}

fn join_path(mut base_path: Vec<String>, path_to_node: &str) -> Result<Vec<String>, ApiError> {
    let mut paths = path_to_node.split(',').map(str::to_owned);
    match (base_path.last(), paths.next()) {
        (Some(last), Some(first)) if &first == last => {
            base_path.extend(paths);
            Ok(base_path)
        }
        (None, _) => {
            let message = "The authorized node path cannot be empty.";
            warn!("ConfigApi - {}", message);
            Err(ApiError::InvalidAuthorizedPath { message: message.to_owned() })
        }
        _ => Err(ApiError::BadRequestError {
            cause: "Node path does not comply with authorized path".to_string(),
        }),
    }
}

implement_authorization!(ConfigView, Permission::ConfigView);
implement_authorization!(ConfigEdit, Permission::ConfigEdit);
implement_authorization!(RuntimeConfigView, Permission::RuntimeConfigView);
implement_authorization!(RuntimeConfigEdit, Permission::RuntimeConfigEdit);
implement_authorization!(TestEventExecuteActions, Permission::TestEventExecuteActions);

#[cfg(test)]
mod test {
    use crate::auth::middleware::join_path;
    use crate::error::ApiError;

    #[test]
    fn should_join_paths() {
        // Arrange
        let base_path = vec!["root".to_owned()];
        let path_to_node = "root,node_1,node_2";
        let expected = vec!["root".to_owned(), "node_1".to_owned(), "node_2".to_owned()];

        // Act
        let result = join_path(base_path, path_to_node).unwrap();

        // Assert
        assert_eq!(expected, result);
    }

    #[test]
    fn should_return_error_on_empty_base_path() {
        // Arrange
        let base_path = vec![];
        let path_to_node = "root,node_1,node_2";

        // Act
        let result = join_path(base_path, path_to_node);

        // Assert
        match result {
            Err(ApiError::InvalidAuthorizedPath { .. }) => {}
            err => unreachable!("{:?}", err),
        }
    }

    #[test]
    fn should_return_error_on_wrong_node_path_root() {
        // Arrange
        let base_path = vec!["root".to_owned()];
        let path_to_node = "node_1,node_2";

        // Act
        let result = join_path(base_path, path_to_node);

        // Assert
        match result {
            Err(ApiError::BadRequestError { .. }) => {}
            err => unreachable!("{:?}", err),
        }
    }

    #[test]
    fn should_return_error_on_empty_path_to_node() {
        // Arrange
        let base_path = vec!["root".to_owned()];
        let path_to_node = "";

        // Act
        let result = join_path(base_path, path_to_node);

        // Assert
        match result {
            Err(ApiError::BadRequestError { .. }) => {}
            err => unreachable!("{:?}", err),
        }
    }
}
