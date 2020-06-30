use crate::auth::convert::to_auth_with_permissions_dto;
use crate::model::ApiData;
use actix_web::web::{Data, Json};
use actix_web::{web, HttpRequest, Scope};
use log::*;
use tornado_engine_api_dto::auth::AuthWithPermissionsDto;

pub fn build_auth_endpoints(data: ApiData<()>) -> Scope {
    web::scope("/v1/auth")
        .data(data)
        .service(web::resource("/who_am_i").route(web::get().to(who_am_i)))
}

async fn who_am_i(
    req: HttpRequest,
    data: Data<ApiData<()>>,
) -> actix_web::Result<Json<AuthWithPermissionsDto>> {
    debug!("HttpRequest method [{}] path [{}]", req.method(), req.path());
    let auth_ctx = data.auth.auth_from_request(&req)?;
    auth_ctx.is_authenticated()?;

    let all_permissions = auth_ctx.get_permissions();
    Ok(Json(to_auth_with_permissions_dto(
        auth_ctx.auth.user.clone(),
        auth_ctx.auth.preferences.clone(),
        &all_permissions,
    )))
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::auth::{AuthService, Permission};
    use crate::error::ApiError;
    use actix_web::{
        http::{header, StatusCode},
        test, App,
    };
    use std::collections::BTreeMap;
    use std::sync::Arc;
    use tornado_engine_api_dto::auth::{Auth, PermissionDto};

    fn auth_service() -> AuthService {
        let mut permission_roles_map = BTreeMap::new();
        permission_roles_map.insert(Permission::ConfigEdit, vec!["edit".to_owned()]);
        permission_roles_map
            .insert(Permission::ConfigView, vec!["edit".to_owned(), "view".to_owned()]);

        AuthService::new(Arc::new(permission_roles_map))
    }

    #[actix_rt::test]
    async fn who_am_i_should_return_status_code_unauthenticated_if_no_token() -> Result<(), ApiError>
    {
        // Arrange
        let mut srv = test::init_service(
            App::new().service(build_auth_endpoints(ApiData { auth: auth_service(), api: () })),
        )
        .await;

        // Act
        let request = test::TestRequest::get().uri("/v1/auth/who_am_i").to_request();

        let response = test::call_service(&mut srv, request).await;

        // Assert
        assert_eq!(StatusCode::UNAUTHORIZED, response.status());
        Ok(())
    }

    #[actix_rt::test]
    async fn who_am_i_should_return_the_auth_with_permissions_dto() -> Result<(), ApiError> {
        // Arrange
        let mut srv = test::init_service(
            App::new().service(build_auth_endpoints(ApiData { auth: auth_service(), api: () })),
        )
        .await;

        // Act
        let request = test::TestRequest::get()
            .header(
                header::AUTHORIZATION,
                AuthService::auth_to_token_header(&Auth::new("user", vec!["view"]))?,
            )
            .uri("/v1/auth/who_am_i")
            .to_request();

        // Assert
        let dto: AuthWithPermissionsDto = test::read_response_json(&mut srv, request).await;

        assert_eq!(
            AuthWithPermissionsDto {
                user: "user".to_owned(),
                preferences: None,
                permissions: vec![PermissionDto::ConfigView]
            },
            dto
        );

        Ok(())
    }
}
