use crate::config::api::{ConfigApi, ConfigApiHandler};
use crate::config::convert::processing_tree_node_details_dto_into_matcher_config;
use crate::error::ApiError;
use crate::model::{ApiDataV2, ExportVersionedMatcherConfig};
use actix_multipart::Multipart;
use actix_web::http::header;
use actix_web::web::{Data, Json, Path};
use actix_web::{web, HttpRequest, HttpResponse, Scope};
use chrono::{Local, SecondsFormat};
use futures_util::TryStreamExt as _;
use gethostname::gethostname;
use log::*;
use serde::de::DeserializeOwned;
use serde::Deserialize;
use std::os::unix::ffi::OsStrExt;
use tornado_engine_api_dto::common::Id;
use tornado_engine_api_dto::config::{
    ProcessingTreeNodeConfigDto, ProcessingTreeNodeDetailsDto, ProcessingTreeNodeEditDto, RuleDto,
    RulePositionDto, TreeInfoDto,
};
use tornado_engine_matcher::config::MatcherConfigEditor;

pub fn build_config_v2_endpoints<
    A: ConfigApiHandler + 'static,
    CM: MatcherConfigEditor + ?Sized + 'static,
>(
    data: ApiDataV2<ConfigApi<A, CM>>,
) -> Scope {
    web::scope("/config")
        .app_data(Data::new(data))
        .service(
            web::scope("/active")
                .service(
                    web::resource("/tree/children/{param_auth}")
                        .route(web::get().to(get_current_tree_node::<A, CM>)),
                )
                .service(
                    web::resource("/tree/children/{param_auth}/{node_path}")
                        .route(web::get().to(get_current_tree_node_with_node_path::<A, CM>)),
                )
                .service(
                    web::resource("/tree/details/{param_auth}/{node_path}")
                        .route(web::get().to(get_current_tree_node_details::<A, CM>)),
                )
                .service(
                    web::resource("/tree/info/{param_auth}")
                        .route(web::get().to(get_current_tree_info::<A, CM>)),
                )
                .service(
                    web::resource("/rule/details/{param_auth}/{ruleset_path}/{rule_name}")
                        .route(web::get().to(get_current_rule_details::<A, CM>)),
                ),
        )
        .service(
            web::scope("/draft")
                .service(
                    web::resource("/tree/children/{param_auth}/{draft_id}")
                        .route(web::get().to(get_draft_tree_node::<A, CM>)),
                )
                .service(
                    web::resource("/tree/children/{param_auth}/{draft_id}/{node_path}")
                        .route(web::get().to(get_draft_tree_node_with_node_path::<A, CM>)),
                )
                .service(
                    web::resource("/tree/details/{param_auth}/{draft_id}/{node_path}")
                        .route(web::get().to(get_draft_tree_node_details::<A, CM>))
                        .route(web::post().to(create_draft_tree_node::<A, CM>))
                        .route(web::put().to(edit_draft_tree_node::<A, CM>))
                        .route(web::delete().to(delete_draft_tree_node::<A, CM>)),
                )
                .service(
                    web::resource("/tree/import/{param_auth}/{draft_id}/{node_path}")
                        .route(web::post().to(import_child_node::<A, CM>))
                        .route(web::put().to(import_node_in_path::<A, CM>)),
                )
                .service(
                    web::resource("/tree/export/{param_auth}/{draft_id}/{node_path}")
                        .route(web::get().to(export_draft_tree_starting_from_node_path::<A, CM>)),
                )
                .service(
                    web::resource("/rule/details/{param_auth}/{draft_id}/{ruleset_path}")
                        .route(web::post().to(create_draft_rule_details::<A, CM>)),
                )
                .service(
                    web::resource(
                        "/rule/details/{param_auth}/{draft_id}/{ruleset_path}/{rule_name}",
                    )
                    .route(web::get().to(get_draft_rule_details::<A, CM>))
                    .route(web::put().to(edit_draft_rule_details::<A, CM>))
                    .route(web::delete().to(delete_draft_rule_details::<A, CM>)),
                )
                .service(
                    web::resource("/rule/move/{param_auth}/{draft_id}/{ruleset_path}/{rule_name}")
                        .route(web::put().to(draft_move_rule::<A, CM>)),
                ),
        )
        .service(
            web::resource("/drafts/{param_auth}")
                .route(web::get().to(get_drafts_by_tenant::<A, CM>))
                .route(web::post().to(create_draft_in_tenant::<A, CM>)),
        )
        .service(
            web::resource("/drafts/{param_auth}/{draft_id}")
                .route(web::delete().to(delete_draft_in_tenant::<A, CM>)),
        )
        .service(
            web::resource("/drafts/{param_auth}/{draft_id}/deploy")
                .route(web::post().to(deploy_draft_for_tenant::<A, CM>)),
        )
        .service(
            web::resource("/drafts/{param_auth}/{draft_id}/takeover")
                .route(web::post().to(draft_take_over_for_tenant::<A, CM>)),
        )
}

#[derive(Deserialize)]
struct AuthAndNodePath {
    param_auth: String,
    node_path: String,
}

#[derive(Deserialize)]
struct RuleDetailsParams {
    param_auth: String,
    ruleset_path: String,
    rule_name: String,
}

#[derive(Deserialize)]
struct DraftRuleDetailsParams {
    param_auth: String,
    draft_id: String,
    ruleset_path: String,
    rule_name: String,
}

#[derive(Deserialize)]
struct DraftRuleDetailsCreateParams {
    param_auth: String,
    draft_id: String,
    ruleset_path: String,
}

#[derive(Deserialize)]
struct DraftPath {
    param_auth: String,
    draft_id: String,
}

#[derive(Deserialize)]
struct DraftPathWithNode {
    param_auth: String,
    draft_id: String,
    node_path: String,
}

async fn get_current_tree_node<
    A: ConfigApiHandler + 'static,
    CM: MatcherConfigEditor + ?Sized + 'static,
>(
    req: HttpRequest,
    data: Data<ApiDataV2<ConfigApi<A, CM>>>,
    param_auth: Path<String>,
) -> actix_web::Result<Json<Vec<ProcessingTreeNodeConfigDto>>> {
    debug!("HttpRequest method [{}] path [{}]", req.method(), req.path());
    let auth_ctx = data.auth.auth_from_request(&req, &param_auth)?;

    let result = data.api.get_current_config_processing_tree_nodes_by_path(auth_ctx, None).await?;
    Ok(Json(result))
}

async fn get_current_tree_node_with_node_path<
    A: ConfigApiHandler + 'static,
    CM: MatcherConfigEditor + ?Sized + 'static,
>(
    req: HttpRequest,
    endpoint_params: Path<AuthAndNodePath>,
    data: Data<ApiDataV2<ConfigApi<A, CM>>>,
) -> actix_web::Result<Json<Vec<ProcessingTreeNodeConfigDto>>> {
    debug!("HttpRequest method [{}] path [{}]", req.method(), req.path());
    let auth_ctx = data.auth.auth_from_request(&req, &endpoint_params.param_auth)?;
    let result = data
        .api
        .get_current_config_processing_tree_nodes_by_path(
            auth_ctx,
            Some(&endpoint_params.node_path),
        )
        .await?;
    Ok(Json(result))
}

async fn get_current_tree_node_details<
    A: ConfigApiHandler + 'static,
    CM: MatcherConfigEditor + ?Sized + 'static,
>(
    req: HttpRequest,
    endpoint_params: Path<AuthAndNodePath>,
    data: Data<ApiDataV2<ConfigApi<A, CM>>>,
) -> actix_web::Result<Json<ProcessingTreeNodeDetailsDto>> {
    debug!("HttpRequest method [{}] path [{}]", req.method(), req.path());
    let auth_ctx = data.auth.auth_from_request(&req, &endpoint_params.param_auth)?;
    let result = data
        .api
        .get_current_config_node_details_by_path(auth_ctx, &endpoint_params.node_path)
        .await?;
    Ok(Json(result))
}

async fn get_draft_tree_node_details<
    A: ConfigApiHandler + 'static,
    CM: MatcherConfigEditor + ?Sized + 'static,
>(
    req: HttpRequest,
    endpoint_params: Path<DraftPathWithNode>,
    data: Data<ApiDataV2<ConfigApi<A, CM>>>,
) -> actix_web::Result<Json<ProcessingTreeNodeDetailsDto>> {
    debug!("HttpRequest method [{}] path [{}]", req.method(), req.path());
    let auth_ctx = data.auth.auth_from_request(&req, &endpoint_params.param_auth)?;
    let result = data
        .api
        .get_draft_config_node_details_by_path(
            auth_ctx,
            &endpoint_params.draft_id,
            &endpoint_params.node_path,
        )
        .await?;
    Ok(Json(result))
}

async fn create_draft_tree_node<
    A: ConfigApiHandler + 'static,
    CM: MatcherConfigEditor + ?Sized + 'static,
>(
    req: HttpRequest,
    endpoint_params: Path<DraftPathWithNode>,
    data: Data<ApiDataV2<ConfigApi<A, CM>>>,
    body: Json<ProcessingTreeNodeEditDto>,
) -> actix_web::Result<Json<()>> {
    debug!("HttpRequest method [{}] path [{}]", req.method(), req.path());
    let auth_ctx = data.auth.auth_from_request(&req, &endpoint_params.param_auth)?;
    let config = processing_tree_node_details_dto_into_matcher_config(body.into_inner())?;
    data.api
        .create_draft_config_node(
            auth_ctx,
            &endpoint_params.draft_id,
            &endpoint_params.node_path,
            config,
        )
        .await?;
    Ok(Json(()))
}

async fn parse_uploaded_file<T: DeserializeOwned>(mut payload: Multipart) -> actix_web::Result<T> {
    let mut file_data = Vec::<u8>::new();
    while let Some(mut field) = payload.try_next().await? {
        let Some("file") = field.content_disposition().get_name() else {
            debug!(
                "Skipping field with Content Disposition {:?} during file upload.",
                field.content_disposition().get_name()
            );
            continue;
        };
        while let Some(chunk) = field.try_next().await? {
            file_data.extend_from_slice(&chunk);
        }
    }
    trace!("File uploaded of size {}", file_data.len());
    let json_deserializer = &mut serde_json::Deserializer::from_slice(&file_data);
    let result: Result<T, _> = serde_path_to_error::deserialize(json_deserializer);

    match result {
        Ok(data) => Ok(data),
        Err(error) => Err(ApiError::BadRequestError { cause: format!("{}", error) }.into()),
    }
}

async fn import_child_node<
    A: ConfigApiHandler + 'static,
    CM: MatcherConfigEditor + ?Sized + 'static,
>(
    req: HttpRequest,
    endpoint_params: Path<DraftPathWithNode>,
    data: Data<ApiDataV2<ConfigApi<A, CM>>>,
    body: Multipart,
) -> actix_web::Result<Json<()>> {
    debug!("HttpRequest method [{}] path [{}]", req.method(), req.path());
    let auth_ctx = data.auth.auth_from_request(&req, &endpoint_params.param_auth)?;
    let config = match parse_uploaded_file(body).await? {
        ExportVersionedMatcherConfig::V1(config) | ExportVersionedMatcherConfig::V1_1(config) => {
            config
        }
    };
    data.api
        .create_draft_config_node(
            auth_ctx,
            &endpoint_params.draft_id,
            &endpoint_params.node_path,
            config,
        )
        .await?;
    Ok(Json(()))
}

async fn edit_draft_tree_node<
    A: ConfigApiHandler + 'static,
    CM: MatcherConfigEditor + ?Sized + 'static,
>(
    req: HttpRequest,
    endpoint_params: Path<DraftPathWithNode>,
    data: Data<ApiDataV2<ConfigApi<A, CM>>>,
    body: Json<ProcessingTreeNodeEditDto>,
) -> actix_web::Result<Json<()>> {
    debug!("HttpRequest method [{}] path [{}]", req.method(), req.path());
    let auth_ctx = data.auth.auth_from_request(&req, &endpoint_params.param_auth)?;
    let config = processing_tree_node_details_dto_into_matcher_config(body.into_inner())?;
    data.api
        .edit_draft_config_node(
            auth_ctx,
            &endpoint_params.draft_id,
            &endpoint_params.node_path,
            config,
        )
        .await?;
    Ok(Json(()))
}

async fn import_node_in_path<
    A: ConfigApiHandler + 'static,
    CM: MatcherConfigEditor + ?Sized + 'static,
>(
    req: HttpRequest,
    endpoint_params: Path<DraftPathWithNode>,
    data: Data<ApiDataV2<ConfigApi<A, CM>>>,
    body: Multipart,
) -> actix_web::Result<Json<()>> {
    debug!("HttpRequest method [{}] path [{}]", req.method(), req.path());
    let auth_ctx = data.auth.auth_from_request(&req, &endpoint_params.param_auth)?;
    let config = match parse_uploaded_file(body).await? {
        ExportVersionedMatcherConfig::V1(config) | ExportVersionedMatcherConfig::V1_1(config) => {
            config
        }
    };
    data.api
        .import_draft_config_node(
            auth_ctx,
            &endpoint_params.draft_id,
            &endpoint_params.node_path,
            config,
        )
        .await?;
    Ok(Json(()))
}

async fn delete_draft_tree_node<
    A: ConfigApiHandler + 'static,
    CM: MatcherConfigEditor + ?Sized + 'static,
>(
    req: HttpRequest,
    endpoint_params: Path<DraftPathWithNode>,
    data: Data<ApiDataV2<ConfigApi<A, CM>>>,
) -> actix_web::Result<Json<()>> {
    debug!("HttpRequest method [{}] path [{}]", req.method(), req.path());
    let auth_ctx = data.auth.auth_from_request(&req, &endpoint_params.param_auth)?;
    data.api
        .delete_draft_config_node(auth_ctx, &endpoint_params.draft_id, &endpoint_params.node_path)
        .await?;
    Ok(Json(()))
}

async fn get_current_tree_info<
    A: ConfigApiHandler + 'static,
    CM: MatcherConfigEditor + ?Sized + 'static,
>(
    req: HttpRequest,
    endpoint_params: Path<String>,
    data: Data<ApiDataV2<ConfigApi<A, CM>>>,
) -> actix_web::Result<Json<TreeInfoDto>> {
    debug!("HttpRequest method [{}] path [{}]", req.method(), req.path());
    let auth_ctx = data.auth.auth_from_request(&req, &endpoint_params)?;
    let result = data.api.get_authorized_tree_info(&auth_ctx).await?;
    Ok(Json(result))
}

async fn get_current_rule_details<
    A: ConfigApiHandler + 'static,
    CM: MatcherConfigEditor + ?Sized + 'static,
>(
    req: HttpRequest,
    endpoint_params: Path<RuleDetailsParams>,
    data: Data<ApiDataV2<ConfigApi<A, CM>>>,
) -> actix_web::Result<Json<RuleDto>> {
    debug!("HttpRequest method [{}] path [{}]", req.method(), req.path());
    let auth_ctx = data.auth.auth_from_request(&req, &endpoint_params.param_auth)?;
    let result = data
        .api
        .get_current_rule_details_by_path(
            &auth_ctx,
            &endpoint_params.ruleset_path,
            &endpoint_params.rule_name,
        )
        .await?;
    Ok(Json(result))
}

async fn get_draft_rule_details<
    A: ConfigApiHandler + 'static,
    CM: MatcherConfigEditor + ?Sized + 'static,
>(
    req: HttpRequest,
    endpoint_params: Path<DraftRuleDetailsParams>,
    data: Data<ApiDataV2<ConfigApi<A, CM>>>,
) -> actix_web::Result<Json<RuleDto>> {
    debug!("HttpRequest method [{}] path [{}]", req.method(), req.path());
    let auth_ctx = data.auth.auth_from_request(&req, &endpoint_params.param_auth)?;
    let result = data
        .api
        .get_draft_rule_details_by_path(
            &auth_ctx,
            &endpoint_params.draft_id,
            &endpoint_params.ruleset_path,
            &endpoint_params.rule_name,
        )
        .await?;
    Ok(Json(result))
}

async fn create_draft_rule_details<
    A: ConfigApiHandler + 'static,
    CM: MatcherConfigEditor + ?Sized + 'static,
>(
    req: HttpRequest,
    endpoint_params: Path<DraftRuleDetailsCreateParams>,
    data: Data<ApiDataV2<ConfigApi<A, CM>>>,
    rule_dto: Json<RuleDto>,
) -> actix_web::Result<Json<()>> {
    debug!("HttpRequest method [{}] path [{}]", req.method(), req.path());
    let auth_ctx = data.auth.auth_from_request(&req, &endpoint_params.param_auth)?;
    data.api
        .create_draft_rule_details_by_path(
            auth_ctx,
            &endpoint_params.draft_id,
            &endpoint_params.ruleset_path,
            rule_dto.0,
        )
        .await?;
    Ok(Json(()))
}

async fn edit_draft_rule_details<
    A: ConfigApiHandler + 'static,
    CM: MatcherConfigEditor + ?Sized + 'static,
>(
    req: HttpRequest,
    endpoint_params: Path<DraftRuleDetailsParams>,
    data: Data<ApiDataV2<ConfigApi<A, CM>>>,
    rule_dto: Json<RuleDto>,
) -> actix_web::Result<Json<()>> {
    debug!("HttpRequest method [{}] path [{}]", req.method(), req.path());
    let auth_ctx = data.auth.auth_from_request(&req, &endpoint_params.param_auth)?;
    data.api
        .edit_draft_rule_details_by_path(
            auth_ctx,
            &endpoint_params.draft_id,
            &endpoint_params.ruleset_path,
            &endpoint_params.rule_name,
            rule_dto.0,
        )
        .await?;
    Ok(Json(()))
}

async fn draft_move_rule<
    A: ConfigApiHandler + 'static,
    CM: MatcherConfigEditor + ?Sized + 'static,
>(
    req: HttpRequest,
    endpoint_params: Path<DraftRuleDetailsParams>,
    data: Data<ApiDataV2<ConfigApi<A, CM>>>,
    rule_dto: Json<RulePositionDto>,
) -> actix_web::Result<Json<()>> {
    debug!("HttpRequest method [{}] path [{}]", req.method(), req.path());
    let auth_ctx = data.auth.auth_from_request(&req, &endpoint_params.param_auth)?;
    data.api
        .move_draft_rule_by_path(
            auth_ctx,
            &endpoint_params.draft_id,
            &endpoint_params.ruleset_path,
            &endpoint_params.rule_name,
            rule_dto.0.position,
        )
        .await?;
    Ok(Json(()))
}

async fn delete_draft_rule_details<
    A: ConfigApiHandler + 'static,
    CM: MatcherConfigEditor + ?Sized + 'static,
>(
    req: HttpRequest,
    endpoint_params: Path<DraftRuleDetailsParams>,
    data: Data<ApiDataV2<ConfigApi<A, CM>>>,
) -> actix_web::Result<Json<()>> {
    debug!("HttpRequest method [{}] path [{}]", req.method(), req.path());
    let auth_ctx = data.auth.auth_from_request(&req, &endpoint_params.param_auth)?;
    data.api
        .delete_draft_rule_details_by_path(
            auth_ctx,
            &endpoint_params.draft_id,
            &endpoint_params.ruleset_path,
            &endpoint_params.rule_name,
        )
        .await?;
    Ok(Json(()))
}

async fn get_draft_tree_node<
    A: ConfigApiHandler + 'static,
    CM: MatcherConfigEditor + ?Sized + 'static,
>(
    req: HttpRequest,
    data: Data<ApiDataV2<ConfigApi<A, CM>>>,
    path: Path<DraftPath>,
) -> actix_web::Result<Json<Vec<ProcessingTreeNodeConfigDto>>> {
    debug!("HttpRequest method [{}] path [{}]", req.method(), req.path());
    let auth_ctx = data.auth.auth_from_request(&req, &path.param_auth)?;

    let result = data
        .api
        .get_draft_config_processing_tree_nodes_by_path(auth_ctx, &path.draft_id, None)
        .await?;
    Ok(Json(result))
}

async fn get_draft_tree_node_with_node_path<
    A: ConfigApiHandler + 'static,
    CM: MatcherConfigEditor + ?Sized + 'static,
>(
    req: HttpRequest,
    endpoint_params: Path<DraftPathWithNode>,
    data: Data<ApiDataV2<ConfigApi<A, CM>>>,
) -> actix_web::Result<Json<Vec<ProcessingTreeNodeConfigDto>>> {
    debug!("HttpRequest method [{}] path [{}]", req.method(), req.path());
    let auth_ctx = data.auth.auth_from_request(&req, &endpoint_params.param_auth)?;
    let result = data
        .api
        .get_draft_config_processing_tree_nodes_by_path(
            auth_ctx,
            &endpoint_params.draft_id,
            Some(&endpoint_params.node_path),
        )
        .await?;
    Ok(Json(result))
}

async fn export_draft_tree_starting_from_node_path<
    A: ConfigApiHandler + 'static,
    CM: MatcherConfigEditor + ?Sized + 'static,
>(
    req: HttpRequest,
    endpoint_params: Path<DraftPathWithNode>,
    data: Data<ApiDataV2<ConfigApi<A, CM>>>,
) -> actix_web::Result<HttpResponse> {
    debug!("HttpRequest method [{}] path [{}]", req.method(), req.path());
    let auth_ctx = data.auth.auth_from_request(&req, &endpoint_params.param_auth)?;
    let result = data
        .api
        .export_draft_tree_starting_from_node_path(
            auth_ctx,
            &endpoint_params.draft_id,
            &endpoint_params.node_path,
        )
        .await?;
    let filename = format!(
        "{}-{}-{}.json",
        String::from_utf8_lossy(gethostname().as_bytes()),
        result.get_name(),
        Local::now().to_rfc3339_opts(SecondsFormat::Secs, false)
    );
    let response = HttpResponse::Ok()
        .insert_header((
            header::CONTENT_DISPOSITION,
            format!("attachment; filename: \"{}\"", filename),
        ))
        .content_type("application/json")
        .json(ExportVersionedMatcherConfig::V1_1(result));
    Ok(response)
}

async fn get_drafts_by_tenant<
    A: ConfigApiHandler + 'static,
    CM: MatcherConfigEditor + ?Sized + 'static,
>(
    req: HttpRequest,
    param_auth: Path<String>,
    data: Data<ApiDataV2<ConfigApi<A, CM>>>,
) -> actix_web::Result<Json<Vec<String>>> {
    debug!("HttpRequest method [{}] path [{}]", req.method(), req.path());
    let auth_ctx = data.auth.auth_from_request(&req, &param_auth)?;
    let result = data.api.get_drafts_by_tenant(&auth_ctx).await?;
    Ok(Json(result))
}

async fn create_draft_in_tenant<
    A: ConfigApiHandler + 'static,
    CM: MatcherConfigEditor + ?Sized + 'static,
>(
    req: HttpRequest,
    param_auth: Path<String>,
    data: Data<ApiDataV2<ConfigApi<A, CM>>>,
) -> actix_web::Result<Json<Id<String>>> {
    debug!("HttpRequest method [{}] path [{}]", req.method(), req.path());
    let auth_ctx = data.auth.auth_from_request(&req, &param_auth)?;
    let result = data.api.create_draft_in_tenant(&auth_ctx).await?;
    Ok(Json(result))
}

async fn delete_draft_in_tenant<
    A: ConfigApiHandler + 'static,
    CM: MatcherConfigEditor + ?Sized + 'static,
>(
    req: HttpRequest,
    path: Path<DraftPath>,
    data: Data<ApiDataV2<ConfigApi<A, CM>>>,
) -> actix_web::Result<Json<()>> {
    debug!("HttpRequest method [{}] path [{}]", req.method(), req.path());
    let auth_ctx = data.auth.auth_from_request(&req, &path.param_auth)?;
    data.api.delete_draft_in_tenant(&auth_ctx, &path.draft_id).await?;
    Ok(Json(()))
}

async fn deploy_draft_for_tenant<
    A: ConfigApiHandler + 'static,
    CM: MatcherConfigEditor + ?Sized + 'static,
>(
    req: HttpRequest,
    path: Path<DraftPath>,
    data: Data<ApiDataV2<ConfigApi<A, CM>>>,
) -> actix_web::Result<Json<()>> {
    debug!("HttpRequest method [{}] path [{}]", req.method(), req.path());
    let auth_ctx = data.auth.auth_from_request(&req, &path.param_auth)?;
    data.api.deploy_draft_for_tenant(&auth_ctx, &path.draft_id).await?;
    Ok(Json(()))
}

async fn draft_take_over_for_tenant<
    A: ConfigApiHandler + 'static,
    CM: MatcherConfigEditor + ?Sized + 'static,
>(
    req: HttpRequest,
    path: Path<DraftPath>,
    data: Data<ApiDataV2<ConfigApi<A, CM>>>,
) -> actix_web::Result<Json<()>> {
    debug!("HttpRequest method [{}] path [{}]", req.method(), req.path());
    let auth_ctx = data.auth.auth_from_request(&req, &path.param_auth)?;
    data.api.draft_take_over_for_tenant(&auth_ctx, &path.draft_id).await?;
    Ok(Json(()))
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::auth::auth_v2::AuthServiceV2;
    use crate::error::ApiError;
    use crate::{auth::auth_v2::test::test_auth_service_v2, test_root::start_context};
    use actix_web::http::header::HeaderName;
    use actix_web::{
        http::{header, StatusCode},
        test, App,
    };
    use async_trait::async_trait;
    use std::collections::HashMap;
    use std::sync::Arc;
    use tornado_engine_api_dto::auth_v2::{AuthHeaderV2, Authorization};
    use tornado_engine_api_dto::config::ConstraintDto;
    use tornado_engine_matcher::config::nodes::Filter;
    use tornado_engine_matcher::config::rule::{Constraint, Rule};
    use tornado_engine_matcher::config::{
        Defaultable, MatcherConfig, MatcherConfigDraft, MatcherConfigDraftData, MatcherConfigReader,
    };
    use tornado_engine_matcher::error::MatcherError;

    struct ConfigManager {}

    #[async_trait::async_trait(? Send)]
    impl MatcherConfigReader for ConfigManager {
        async fn get_config(&self) -> Result<MatcherConfig, MatcherError> {
            Ok(MatcherConfig::Filter {
                name: "root".to_owned(),
                filter: Filter {
                    description: "".to_string(),
                    filter: Defaultable::Default {},
                    active: false,
                },
                nodes: vec![
                    MatcherConfig::Filter {
                        name: "child_1".to_owned(),
                        filter: Filter {
                            description: "".to_string(),
                            filter: Defaultable::Default {},
                            active: false,
                        },
                        nodes: vec![],
                    },
                    MatcherConfig::Ruleset {
                        name: "child_2".to_owned(),
                        rules: vec![Rule {
                            name: "rule_1".to_string(),
                            description: "Rule 1 description".to_string(),
                            do_continue: false,
                            active: true,
                            constraint: Constraint {
                                where_operator: None,
                                with: Default::default(),
                            },
                            actions: vec![],
                        }],
                    },
                ],
            })
        }
    }

    #[async_trait::async_trait(? Send)]
    impl MatcherConfigEditor for ConfigManager {
        async fn get_drafts(&self) -> Result<Vec<String>, MatcherError> {
            Ok(vec![])
        }

        async fn get_draft(&self, draft_id: &str) -> Result<MatcherConfigDraft, MatcherError> {
            Ok(MatcherConfigDraft {
                data: MatcherConfigDraftData {
                    user: "user".to_owned(),
                    draft_id: draft_id.to_owned(),
                    created_ts_ms: 0,
                    updated_ts_ms: 0,
                },
                config: MatcherConfig::Filter {
                    name: "root".to_owned(),
                    filter: Filter {
                        description: "".to_string(),
                        filter: Defaultable::Default {},
                        active: false,
                    },
                    nodes: vec![
                        MatcherConfig::Filter {
                            name: "child_1".to_owned(),
                            filter: Filter {
                                description: "".to_string(),
                                filter: Defaultable::Default {},
                                active: false,
                            },
                            nodes: vec![MatcherConfig::Filter {
                                name: "child_1_1".to_owned(),
                                filter: Filter {
                                    description: "".to_string(),
                                    filter: Defaultable::Default {},
                                    active: false,
                                },
                                nodes: vec![],
                            }],
                        },
                        MatcherConfig::Ruleset {
                            name: "child_2".to_owned(),
                            rules: vec![Rule {
                                name: "rule_1".to_string(),
                                description: "Rule 1 description".to_string(),
                                do_continue: false,
                                active: true,
                                constraint: Constraint {
                                    where_operator: None,
                                    with: Default::default(),
                                },
                                actions: vec![],
                            }],
                        },
                    ],
                },
            })
        }

        async fn create_draft(&self, _user: String) -> Result<String, MatcherError> {
            Ok("".to_string())
        }

        async fn update_draft(
            &self,
            _draft_id: &str,
            _user: String,
            _config: &MatcherConfig,
        ) -> Result<(), MatcherError> {
            Ok(())
        }

        async fn deploy_draft(&self, _draft_id: &str) -> Result<MatcherConfig, MatcherError> {
            Ok(MatcherConfig::Ruleset { name: "ruleset_new".to_owned(), rules: vec![] })
        }

        async fn delete_draft(&self, _draft_id: &str) -> Result<(), MatcherError> {
            Ok(())
        }

        async fn draft_take_over(
            &self,
            _draft_id: &str,
            _user: String,
        ) -> Result<(), MatcherError> {
            Ok(())
        }

        async fn deploy_config(
            &self,
            _config: &MatcherConfig,
        ) -> Result<MatcherConfig, MatcherError> {
            unimplemented!()
        }
    }

    struct TestApiHandler {}

    #[async_trait(? Send)]
    impl ConfigApiHandler for TestApiHandler {
        async fn reload_configuration(&self) -> Result<MatcherConfig, ApiError> {
            Ok(MatcherConfig::Ruleset { name: "ruleset_new".to_owned(), rules: vec![] })
        }
    }

    fn auth_map(name: &str, auth: Authorization) -> HashMap<String, Authorization> {
        let mut auths = HashMap::new();
        auths.insert(name.to_owned(), auth);
        auths
    }

    fn test_auth_root_edit() -> (HeaderName, String) {
        (
            header::AUTHORIZATION,
            AuthServiceV2::auth_to_token_header(&AuthHeaderV2 {
                user: "user".to_string(),
                auths: auth_map(
                    "auth1",
                    Authorization { path: vec!["root".to_owned()], roles: vec!["edit".to_owned()] },
                ),
                preferences: None,
            })
            .unwrap(),
        )
    }

    #[actix_rt::test]
    async fn v2_endpoint_should_have_a_get_drafts_for_tenant_get_endpoint() -> Result<(), ApiError>
    {
        // Arrange
        let srv = test::init_service(App::new().service(build_config_v2_endpoints(ApiDataV2 {
            auth: test_auth_service_v2(),
            api: ConfigApi::new(TestApiHandler {}, Arc::new(ConfigManager {})),
        })))
        .await;

        // Act
        let request = test::TestRequest::get()
            .insert_header(test_auth_root_edit())
            .uri("/config/drafts/auth1")
            .to_request();

        let response = test::call_service(&srv, request).await;

        // Assert
        assert_eq!(StatusCode::OK, response.status());
        Ok(())
    }

    #[actix_rt::test]
    async fn v2_endpoint_should_have_a_get_draft_single_node_get_endpoint() -> Result<(), ApiError>
    {
        // Arrange
        let srv = test::init_service(App::new().service(build_config_v2_endpoints(ApiDataV2 {
            auth: test_auth_service_v2(),
            api: ConfigApi::new(TestApiHandler {}, Arc::new(ConfigManager {})),
        })))
        .await;

        // Act
        let request = test::TestRequest::get()
            .insert_header(test_auth_root_edit())
            .uri("/config/draft/tree/children/auth1/draft123")
            .to_request();

        let response = test::call_service(&srv, request).await;

        // Assert
        assert_eq!(StatusCode::OK, response.status());
        Ok(())
    }

    #[actix_rt::test]
    async fn v2_endpoint_should_have_a_get_draft_single_node_with_path_get_endpoint(
    ) -> Result<(), ApiError> {
        start_context();

        // Arrange
        let srv = test::init_service(App::new().service(build_config_v2_endpoints(ApiDataV2 {
            auth: test_auth_service_v2(),
            api: ConfigApi::new(TestApiHandler {}, Arc::new(ConfigManager {})),
        })))
        .await;

        // Act
        let request = test::TestRequest::get()
            .insert_header((
                header::AUTHORIZATION,
                AuthServiceV2::auth_to_token_header(&AuthHeaderV2 {
                    user: "user".to_string(),
                    auths: auth_map(
                        "auth1",
                        Authorization {
                            path: vec!["root".to_owned(), "child_1".to_owned()],
                            roles: vec!["edit".to_owned()],
                        },
                    ),
                    preferences: None,
                })?,
            ))
            .uri("/config/draft/tree/children/auth1/draft123/child_1")
            .to_request();

        let response = test::call_service(&srv, request).await;

        // Assert
        assert_eq!(StatusCode::OK, response.status());

        let dto: Vec<ProcessingTreeNodeConfigDto> = test::read_body_json(response).await;

        assert_eq!(
            vec![ProcessingTreeNodeConfigDto::Filter {
                name: "child_1_1".to_owned(),
                rules_count: 0,
                children_count: 0,
                has_iterator_ancestor: false,
                description: "".to_string(),
                active: false,
            },],
            dto
        );
        Ok(())
    }

    #[actix_rt::test]
    async fn v2_endpoint_should_have_a_create_draft_in_tenant_post_endpoint() -> Result<(), ApiError>
    {
        // Arrange
        let srv = test::init_service(App::new().service(build_config_v2_endpoints(ApiDataV2 {
            auth: test_auth_service_v2(),
            api: ConfigApi::new(TestApiHandler {}, Arc::new(ConfigManager {})),
        })))
        .await;

        // Act
        let request = test::TestRequest::post()
            .insert_header(test_auth_root_edit())
            .uri("/config/drafts/auth1")
            .to_request();

        let response = test::call_service(&srv, request).await;

        // Assert
        assert_eq!(StatusCode::OK, response.status());
        Ok(())
    }

    #[actix_rt::test]
    async fn v2_endpoint_should_have_a_delete_draft_for_tenant_delete_endpoint(
    ) -> Result<(), ApiError> {
        // Arrange
        let srv = test::init_service(App::new().service(build_config_v2_endpoints(ApiDataV2 {
            auth: test_auth_service_v2(),
            api: ConfigApi::new(TestApiHandler {}, Arc::new(ConfigManager {})),
        })))
        .await;

        // Act
        let request = test::TestRequest::delete()
            .insert_header(test_auth_root_edit())
            .uri("/config/drafts/auth1/draft123")
            .to_request();

        let response = test::call_service(&srv, request).await;

        // Assert
        assert_eq!(StatusCode::OK, response.status());
        Ok(())
    }

    #[actix_rt::test]
    async fn v2_endpoint_should_have_a_deploy_draft_for_tenant_post_endpoint(
    ) -> Result<(), ApiError> {
        // Arrange
        let srv = test::init_service(App::new().service(build_config_v2_endpoints(ApiDataV2 {
            auth: test_auth_service_v2(),
            api: ConfigApi::new(TestApiHandler {}, Arc::new(ConfigManager {})),
        })))
        .await;

        // Act
        let request = test::TestRequest::post()
            .insert_header(test_auth_root_edit())
            .uri("/config/drafts/auth1/draft123/deploy")
            .to_request();

        let response = test::call_service(&srv, request).await;

        // Assert
        assert_eq!(StatusCode::OK, response.status());
        Ok(())
    }

    #[actix_rt::test]
    async fn v2_endpoint_should_have_a_draft_take_over_for_tenant_post_endpoint(
    ) -> Result<(), ApiError> {
        // Arrange
        let srv = test::init_service(App::new().service(build_config_v2_endpoints(ApiDataV2 {
            auth: test_auth_service_v2(),
            api: ConfigApi::new(TestApiHandler {}, Arc::new(ConfigManager {})),
        })))
        .await;

        // Act
        let request = test::TestRequest::post()
            .insert_header(test_auth_root_edit())
            .uri("/config/drafts/auth1/draft123/takeover")
            .to_request();

        let response = test::call_service(&srv, request).await;

        // Assert
        assert_eq!(StatusCode::OK, response.status());
        Ok(())
    }

    #[actix_rt::test]
    async fn v2_endpoint_get_children_should_return_status_code_ok() -> Result<(), ApiError> {
        // Arrange
        let srv = test::init_service(App::new().service(build_config_v2_endpoints(ApiDataV2 {
            auth: test_auth_service_v2(),
            api: ConfigApi::new(TestApiHandler {}, Arc::new(ConfigManager {})),
        })))
        .await;

        // Act
        let request = test::TestRequest::get()
            .insert_header((
                header::AUTHORIZATION,
                AuthServiceV2::auth_to_token_header(&AuthHeaderV2 {
                    user: "admin".to_string(),
                    auths: auth_map(
                        "auth1",
                        Authorization {
                            path: vec!["root".to_owned()],
                            roles: vec!["view".to_owned()],
                        },
                    ),
                    preferences: None,
                })?,
            ))
            .uri("/config/active/tree/children/auth1")
            .to_request();

        let response = test::call_service(&srv, request).await;

        // Assert
        assert_eq!(StatusCode::OK, response.status());
        Ok(())
    }

    #[actix_rt::test]
    async fn v2_endpoint_get_children_by_node_path_should_return_status_code_ok(
    ) -> Result<(), ApiError> {
        // Arrange
        let srv = test::init_service(App::new().service(build_config_v2_endpoints(ApiDataV2 {
            auth: test_auth_service_v2(),
            api: ConfigApi::new(TestApiHandler {}, Arc::new(ConfigManager {})),
        })))
        .await;

        // Act
        let request = test::TestRequest::get()
            .insert_header((
                header::AUTHORIZATION,
                AuthServiceV2::auth_to_token_header(&AuthHeaderV2 {
                    user: "admin".to_string(),
                    auths: auth_map(
                        "auth1",
                        Authorization {
                            path: vec!["root".to_owned()],
                            roles: vec!["view".to_owned(), "edit".to_owned()],
                        },
                    ),
                    preferences: None,
                })?,
            ))
            .uri("/config/active/tree/children/auth1/root")
            .to_request();

        let response = test::call_service(&srv, request).await;

        // Assert
        assert_eq!(StatusCode::OK, response.status());
        Ok(())
    }

    #[actix_rt::test]
    async fn v2_endpoint_get_details_by_node_path_should_return_status_code_ok(
    ) -> Result<(), ApiError> {
        // Arrange
        let srv = test::init_service(App::new().service(build_config_v2_endpoints(ApiDataV2 {
            auth: test_auth_service_v2(),
            api: ConfigApi::new(TestApiHandler {}, Arc::new(ConfigManager {})),
        })))
        .await;

        // Act
        let request = test::TestRequest::get()
            .insert_header((
                header::AUTHORIZATION,
                AuthServiceV2::auth_to_token_header(&AuthHeaderV2 {
                    user: "admin".to_string(),
                    auths: auth_map(
                        "auth1",
                        Authorization {
                            path: vec!["root".to_owned(), "child_1".to_owned()],
                            roles: vec!["view".to_owned()],
                        },
                    ),
                    preferences: None,
                })?,
            ))
            .uri("/config/active/tree/details/auth1/child_1")
            .to_request();

        let response = test::call_service(&srv, request).await;

        // Assert
        assert_eq!(StatusCode::OK, response.status());
        Ok(())
    }

    #[actix_rt::test]
    async fn v2_endpoint_get_rule_details_by_ruleset_path_should_return_status_code_ok(
    ) -> Result<(), ApiError> {
        // Arrange
        let srv = test::init_service(App::new().service(build_config_v2_endpoints(ApiDataV2 {
            auth: test_auth_service_v2(),
            api: ConfigApi::new(TestApiHandler {}, Arc::new(ConfigManager {})),
        })))
        .await;

        // Act
        let request = test::TestRequest::get()
            .insert_header((
                header::AUTHORIZATION,
                AuthServiceV2::auth_to_token_header(&AuthHeaderV2 {
                    user: "admin".to_string(),
                    auths: auth_map(
                        "auth1",
                        Authorization {
                            path: vec!["root".to_owned(), "child_2".to_owned()],
                            roles: vec!["view".to_owned()],
                        },
                    ),
                    preferences: None,
                })?,
            ))
            .uri("/config/active/rule/details/auth1/child_2/rule_1")
            .to_request();

        let response = test::call_service(&srv, request).await;

        // Assert
        assert_eq!(StatusCode::OK, response.status());
        Ok(())
    }

    #[actix_rt::test]
    async fn v2_endpoint_get_draft_details_by_node_path_should_return_status_code_ok(
    ) -> Result<(), ApiError> {
        // Arrange
        let srv = test::init_service(App::new().service(build_config_v2_endpoints(ApiDataV2 {
            auth: test_auth_service_v2(),
            api: ConfigApi::new(TestApiHandler {}, Arc::new(ConfigManager {})),
        })))
        .await;

        // Act
        let request = test::TestRequest::get()
            .insert_header((
                header::AUTHORIZATION,
                AuthServiceV2::auth_to_token_header(&AuthHeaderV2 {
                    user: "user".to_string(),
                    auths: auth_map(
                        "auth1",
                        Authorization {
                            path: vec!["root".to_owned(), "child_1".to_owned()],
                            roles: vec!["view".to_owned()],
                        },
                    ),
                    preferences: None,
                })?,
            ))
            .uri("/config/draft/tree/details/auth1/draft123/child_1")
            .to_request();

        let response = test::call_service(&srv, request).await;

        // Assert
        assert_eq!(StatusCode::OK, response.status());
        Ok(())
    }

    #[actix_rt::test]
    async fn v2_endpoint_get_draft_rule_details_by_ruleset_path_should_return_status_code_ok(
    ) -> Result<(), ApiError> {
        // Arrange
        let srv = test::init_service(App::new().service(build_config_v2_endpoints(ApiDataV2 {
            auth: test_auth_service_v2(),
            api: ConfigApi::new(TestApiHandler {}, Arc::new(ConfigManager {})),
        })))
        .await;

        // Act
        let request = test::TestRequest::get()
            .insert_header((
                header::AUTHORIZATION,
                AuthServiceV2::auth_to_token_header(&AuthHeaderV2 {
                    user: "user".to_string(),
                    auths: auth_map(
                        "auth1",
                        Authorization {
                            path: vec!["root".to_owned(), "child_2".to_owned()],
                            roles: vec!["view".to_owned()],
                        },
                    ),
                    preferences: None,
                })?,
            ))
            .uri("/config/active/rule/details/auth1/child_2/rule_1")
            .to_request();

        let response = test::call_service(&srv, request).await;

        // Assert
        assert_eq!(StatusCode::OK, response.status());
        Ok(())
    }

    #[actix_rt::test]
    async fn v2_endpoint_get_tree_info_return_status_code_ok() -> Result<(), ApiError> {
        // Arrange
        let srv = test::init_service(App::new().service(build_config_v2_endpoints(ApiDataV2 {
            auth: test_auth_service_v2(),
            api: ConfigApi::new(TestApiHandler {}, Arc::new(ConfigManager {})),
        })))
        .await;

        // Act
        let request = test::TestRequest::get()
            .insert_header((
                header::AUTHORIZATION,
                AuthServiceV2::auth_to_token_header(&AuthHeaderV2 {
                    user: "admin".to_string(),
                    auths: auth_map(
                        "auth1",
                        Authorization {
                            path: vec!["root".to_owned()],
                            roles: vec!["view".to_owned()],
                        },
                    ),
                    preferences: None,
                })?,
            ))
            .uri("/config/active/tree/info/auth1")
            .to_request();

        let response = test::call_service(&srv, request).await;

        // Assert
        assert_eq!(StatusCode::OK, response.status());
        Ok(())
    }

    #[actix_rt::test]
    async fn v2_endpoint_create_node_in_draft_by_path_should_return_ok() -> Result<(), ApiError> {
        // Arrange
        let srv = test::init_service(App::new().service(build_config_v2_endpoints(ApiDataV2 {
            auth: test_auth_service_v2(),
            api: ConfigApi::new(TestApiHandler {}, Arc::new(ConfigManager {})),
        })))
        .await;

        // Act
        let request = test::TestRequest::post()
            .insert_header(test_auth_root_edit())
            .uri("/config/draft/tree/details/auth1/draft123/root,child_1")
            .set_json(&ProcessingTreeNodeDetailsDto::Filter {
                name: "test_filter".to_string(),
                description: "".to_string(),
                active: false,
                filter: None,
            })
            .to_request();

        let response = test::call_service(&srv, request).await;

        // Assert
        assert_eq!(StatusCode::OK, response.status());
        Ok(())
    }

    #[actix_rt::test]
    async fn v2_endpoint_create_rule_in_draft_by_path_should_return_ok() -> Result<(), ApiError> {
        // Arrange
        let srv = test::init_service(App::new().service(build_config_v2_endpoints(ApiDataV2 {
            auth: test_auth_service_v2(),
            api: ConfigApi::new(TestApiHandler {}, Arc::new(ConfigManager {})),
        })))
        .await;

        // Act
        let request = test::TestRequest::post()
            .insert_header(test_auth_root_edit())
            .uri("/config/draft/rule/details/auth1/draft123/root,child_2")
            .set_json(&RuleDto {
                name: "rule-1".to_string(),
                description: "nothing relevant".to_string(),
                do_continue: false,
                active: true,
                constraint: ConstraintDto { where_operator: None, with: Default::default() },
                actions: vec![],
            })
            .to_request();

        let response = test::call_service(&srv, request).await;

        // Assert
        assert_eq!(StatusCode::OK, response.status());
        Ok(())
    }

    #[actix_rt::test]
    async fn v2_endpoint_edit_rule_in_draft_by_path_should_return_ok() -> Result<(), ApiError> {
        // Arrange
        let srv = test::init_service(App::new().service(build_config_v2_endpoints(ApiDataV2 {
            auth: test_auth_service_v2(),
            api: ConfigApi::new(TestApiHandler {}, Arc::new(ConfigManager {})),
        })))
        .await;

        // Act
        let request = test::TestRequest::put()
            .insert_header(test_auth_root_edit())
            .uri("/config/draft/rule/details/auth1/draft123/root,child_2/rule_1")
            .set_json(&RuleDto {
                name: "rule_2".to_string(),
                description: "nothing relevant".to_string(),
                do_continue: false,
                active: true,
                constraint: ConstraintDto { where_operator: None, with: Default::default() },
                actions: vec![],
            })
            .to_request();

        let response = test::call_service(&srv, request).await;

        // Assert
        assert_eq!(StatusCode::OK, response.status());
        Ok(())
    }

    #[actix_rt::test]
    async fn v2_endpoint_delete_rule_in_draft_by_path_should_return_ok() -> Result<(), ApiError> {
        // Arrange
        let srv = test::init_service(App::new().service(build_config_v2_endpoints(ApiDataV2 {
            auth: test_auth_service_v2(),
            api: ConfigApi::new(TestApiHandler {}, Arc::new(ConfigManager {})),
        })))
        .await;

        // Act
        let request = test::TestRequest::delete()
            .insert_header(test_auth_root_edit())
            .uri("/config/draft/rule/details/auth1/draft123/root,child_2/rule_1")
            .to_request();

        let response = test::call_service(&srv, request).await;

        // Assert
        assert_eq!(StatusCode::OK, response.status());
        Ok(())
    }

    #[actix_rt::test]
    async fn v2_endpoint_move_rule_in_draft_by_path_should_return_ok() -> Result<(), ApiError> {
        // Arrange
        let srv = test::init_service(App::new().service(build_config_v2_endpoints(ApiDataV2 {
            auth: test_auth_service_v2(),
            api: ConfigApi::new(TestApiHandler {}, Arc::new(ConfigManager {})),
        })))
        .await;

        // Act
        let request = test::TestRequest::put()
            .insert_header(test_auth_root_edit())
            .uri("/config/draft/rule/move/auth1/draft123/root,child_2/rule_1")
            .set_json(&RulePositionDto { position: 0 })
            .to_request();

        let response = test::call_service(&srv, request).await;

        // Assert
        assert_eq!(StatusCode::OK, response.status());
        Ok(())
    }

    #[actix_rt::test]
    async fn v2_endpoint_move_rule_in_draft_out_of_bounds_by_path_should_return_err(
    ) -> Result<(), ApiError> {
        // Arrange
        let srv = test::init_service(App::new().service(build_config_v2_endpoints(ApiDataV2 {
            auth: test_auth_service_v2(),
            api: ConfigApi::new(TestApiHandler {}, Arc::new(ConfigManager {})),
        })))
        .await;

        // Act
        let request = test::TestRequest::put()
            .insert_header(test_auth_root_edit())
            .uri("/config/draft/rule/move/auth1/draft123/root,child_2/rule_1")
            .set_json(&RulePositionDto { position: 5 })
            .to_request();

        let response = test::call_service(&srv, request).await;

        // Assert
        assert_eq!(StatusCode::BAD_REQUEST, response.status());
        Ok(())
    }

    #[actix_rt::test]
    async fn v2_endpoint_edit_node_in_draft_by_path_should_return_ok() -> Result<(), ApiError> {
        // Arrange
        let srv = test::init_service(App::new().service(build_config_v2_endpoints(ApiDataV2 {
            auth: test_auth_service_v2(),
            api: ConfigApi::new(TestApiHandler {}, Arc::new(ConfigManager {})),
        })))
        .await;

        // Act
        let request = test::TestRequest::put()
            .insert_header(test_auth_root_edit())
            .uri("/config/draft/tree/details/auth1/draft123/root,child_1")
            .set_json(&ProcessingTreeNodeDetailsDto::Filter {
                name: "test_filter".to_string(),
                description: "".to_string(),
                active: false,
                filter: None,
            })
            .to_request();

        let response = test::call_service(&srv, request).await;

        // Assert
        assert_eq!(StatusCode::OK, response.status());
        Ok(())
    }

    #[actix_rt::test]
    async fn v2_endpoint_delete_node_in_draft_by_path_should_return_ok() -> Result<(), ApiError> {
        // Arrange
        let srv = test::init_service(App::new().service(build_config_v2_endpoints(ApiDataV2 {
            auth: test_auth_service_v2(),
            api: ConfigApi::new(TestApiHandler {}, Arc::new(ConfigManager {})),
        })))
        .await;

        // Act
        let request = test::TestRequest::delete()
            .insert_header(test_auth_root_edit())
            .uri("/config/draft/tree/details/auth1/draft123/root,child_1")
            .to_request();

        let response = test::call_service(&srv, request).await;

        // Assert
        assert_eq!(StatusCode::OK, response.status());
        Ok(())
    }
}
