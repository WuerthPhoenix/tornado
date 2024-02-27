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

// This macro rule allows us to create the necessary permission structs to
// keep the code-base DRY.
macro_rules! implement_authorization {
    ($auth:ident, $permission:expr) => {
        // Create a unit struct as a marker type for the permissions.
        #[derive(Debug)]
        pub struct $auth;

        // implement FromRequest to make the marker type usable in the endpoints.
        impl FromRequest for AuthorizedPath<$auth> {
            type Error = ApiError;
            type Future = Ready<Result<Self, Self::Error>>;

            fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
                ready(from_request(req, &[&$permission]))
            }
        }
    };
}

// The authorized path holds the full path from the root that the user can access and the user name.
// The generic parameter is a marker type to distinguish between permissions.
#[derive(Debug)]
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

// This take a request, parses the path and headers and returns the AuthorizedPath with
// the right permissions, if the user has any of the necessary permissions.
fn from_request<T>(
    req: &HttpRequest,
    permissions: &[&Permission],
) -> Result<AuthorizedPath<T>, ApiError> {
    let Some(auth_service) = req.app_data::<Data<AuthServiceV2>>() else {
        // This is always mounted in the daemon command. If it is missing we cannot make any
        // authentication and this is a bug. However this will be caught by tests before it
        // ever can go into production.
        return Err(ApiError::InternalServerError { cause: "AuthServiceV2 was not mounted. This is a bug!".to_string() });
    };

    // If an AuthorizedPath is requested as a endpoint parameter, it needs to accept a param_auth.
    let Some(param_auth) = req.match_info().get("param_auth") else {
        return Err(ApiError::UnauthenticatedError);
    };

    // Check all the user permissions right here to avoid querying them later again.
    // The Auth marker types guarantee that this check was performed with the correct permissions.
    let auth_context = auth_service.auth_from_request(req, param_auth)?;
    auth_context.has_any_permission(permissions)?;
    auth_context.is_authenticated()?;

    let user = auth_context.auth.user;
    let base_path = auth_context.auth.authorization.path;

    // The node path is present if the user wants to access a specific node in the path.
    // Otherwise the path for the user will be just the base path to the node.
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

// Create all marker types for the Permissions. If permissions are added in the future, add them here!
implement_authorization!(ConfigView, Permission::ConfigView);
implement_authorization!(ConfigEdit, Permission::ConfigEdit);
implement_authorization!(RuntimeConfigView, Permission::RuntimeConfigView);
implement_authorization!(RuntimeConfigEdit, Permission::RuntimeConfigEdit);
implement_authorization!(TestEventExecuteActions, Permission::TestEventExecuteActions);

#[cfg(test)]
mod test {
    use crate::auth::auth_v2::test::test_auth_service_v2;
    use crate::auth::middleware::{from_request, join_path, AuthorizedPath, ConfigView};
    use crate::auth::Permission;
    use crate::error::ApiError;
    use actix_web::http::header;
    use actix_web::http::header::HeaderName;
    use actix_web::test::TestRequest;
    use actix_web::web::Data;
    use base64::{engine::general_purpose::STANDARD as base64, Engine as _};
    use serde_json::json;

    fn auth_header() -> (HeaderName, String) {
        let permissions = json!({
            "user": "root",
            "auths": {
                "root-auth": {
                    "path": [ "root" ],
                    "roles": [ "view", "edit" ]
                },
                "master-auth": {
                    "path": [ "root", "master" ],
                    "roles": [ "view", "edit" ]
                },
                "empty-auth": {
                    "path": [],
                    "roles": [ "view" ],
                }
            },
            "preferences": {
                "language": "en_US"
            }
        });

        let token = base64.encode(serde_json::to_string(&permissions).unwrap());
        (header::AUTHORIZATION, format!("Bearer {token}"))
    }

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

    #[test]
    fn should_provide_base_path_from_request() {
        let req = TestRequest::get()
            .insert_header(auth_header())
            .app_data(Data::new(test_auth_service_v2()))
            .param("param_auth", "root-auth")
            .to_http_request();

        let result: AuthorizedPath<ConfigView> =
            from_request(&req, &[&Permission::ConfigView]).unwrap();

        assert_eq!("root", &result.user);
        assert_eq!(vec!["root".to_owned()], result.path);
    }

    #[test]
    fn should_fail_on_missing_permission() {
        let req = TestRequest::get()
            .insert_header(auth_header())
            .app_data(Data::new(test_auth_service_v2()))
            .param("param_auth", "root-auth")
            .to_http_request();

        let result: Result<AuthorizedPath<ConfigView>, _> =
            from_request(&req, &[&Permission::RuntimeConfigEdit]);

        match result {
            Err(ApiError::ForbiddenError { code, .. }) => {
                assert_eq!("MISSING_REQUIRED_PERMISSIONS", &code);
            }
            err => unreachable!("{:?}", err),
        }
    }

    #[test]
    fn should_fail_on_missing_param_auth() {
        let req = TestRequest::get()
            .insert_header(auth_header())
            .app_data(Data::new(test_auth_service_v2()))
            .to_http_request();

        let result: Result<AuthorizedPath<ConfigView>, _> =
            from_request(&req, &[&Permission::RuntimeConfigEdit]);

        match result {
            Err(ApiError::UnauthenticatedError { .. }) => {}
            err => unreachable!("{:?}", err),
        }
    }

    #[test]
    fn should_provide_full_path_from_request() {
        let req = TestRequest::get()
            .insert_header(auth_header())
            .app_data(Data::new(test_auth_service_v2()))
            .param("param_auth", "root-auth")
            .param("node_path", "root,node_1,node_2")
            .to_http_request();

        let result: AuthorizedPath<ConfigView> =
            from_request(&req, &[&Permission::ConfigView]).unwrap();

        assert_eq!("root", &result.user);
        assert_eq!(vec!["root".to_owned(), "node_1".to_owned(), "node_2".to_owned()], result.path);
    }

    #[test]
    fn should_fail_on_wrong_path() {
        let req = TestRequest::get()
            .insert_header(auth_header())
            .app_data(Data::new(test_auth_service_v2()))
            .param("param_auth", "root-auth")
            .param("node_path", "master,node_1")
            .to_http_request();

        let result: Result<AuthorizedPath<ConfigView>, _> =
            from_request(&req, &[&Permission::ConfigView]);

        match result {
            Err(ApiError::BadRequestError { .. }) => {}
            err => unreachable!("{:?}", err),
        }
    }

    #[test]
    fn should_provide_full_path_from_tenant_request() {
        let req = TestRequest::get()
            .insert_header(auth_header())
            .app_data(Data::new(test_auth_service_v2()))
            .param("param_auth", "master-auth")
            .param("node_path", "master,node_1,node_2")
            .to_http_request();

        let result: AuthorizedPath<ConfigView> =
            from_request(&req, &[&Permission::ConfigView]).unwrap();

        assert_eq!("root", &result.user);
        assert_eq!(
            vec!["root".to_owned(), "master".to_owned(), "node_1".to_owned(), "node_2".to_owned()],
            result.path
        );
    }

    #[test]
    fn should_fail_on_empty_auth() {
        let req = TestRequest::get()
            .insert_header(auth_header())
            .app_data(Data::new(test_auth_service_v2()))
            .param("param_auth", "empty-auth")
            .param("node_path", "master,node_1")
            .to_http_request();

        let result: Result<AuthorizedPath<ConfigView>, _> =
            from_request(&req, &[&Permission::ConfigView]);

        match result {
            Err(ApiError::InvalidAuthorizedPath { .. }) => {}
            err => unreachable!("{:?}", err),
        }
    }
}
