use crate::api::error::ApiError;
use crate::api::models::DisplayGroup;
use crate::api::models::DisplayGroupDetails;
use crate::api::models::GroupInfo;
use crate::db::operations;
use crate::db::operations::models::GroupUpdate;
use crate::db::operations::models::NewGroup;
use crate::db::operations::models::SortGroupsBy;
use crate::db::types::GroupType;
use crate::db::Pool;
use actix_web::dev::HttpServiceFactory;
use actix_web::web;
use actix_web::HttpResponse;
use cis_client::AsyncCisClientTrait;
use dino_park_gate::scope::ScopeAndUser;
use dino_park_trust::GroupsTrust;
use log::info;
use serde::Deserialize;
use std::sync::Arc;

#[derive(Deserialize)]
struct ListGroupsQuery {
    f: Option<String>,
    #[serde(default)]
    n: i64,
    #[serde(default = "default_groups_list_size")]
    s: i64,
    #[serde(default)]
    by: SortGroupsBy,
}

fn default_groups_list_size() -> i64 {
    20
}

#[guard(Authenticated)]
async fn get_group(
    pool: web::Data<Pool>,
    group_name: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    operations::groups::get_group(&pool, &group_name)
        .map(|group| HttpResponse::Ok().json(&DisplayGroup::from(group)))
        .map_err(Into::into)
}

#[guard(Ndaed)]
async fn list_groups(
    pool: web::Data<Pool>,
    query: web::Query<ListGroupsQuery>,
) -> Result<HttpResponse, ApiError> {
    let query = query.into_inner();
    operations::groups::list_groups(&pool, query.f, query.by, query.s, query.n)
        .map(|groups| HttpResponse::Ok().json(&groups))
        .map_err(ApiError::GenericBadRequest)
}

#[guard(Ndaed, None, Medium)]
async fn update_group(
    pool: web::Data<Pool>,
    scope_and_user: ScopeAndUser,
    group_update: web::Json<GroupUpdate>,
    group_name: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    let group_update = group_update.into_inner().checked()?;
    operations::groups::update_group(
        &pool,
        &scope_and_user,
        group_name.into_inner(),
        group_update,
    )
    .map(|_| HttpResponse::Created().json(""))
    .map_err(ApiError::GenericBadRequest)
}

#[guard(Staff, Creator, Medium)]
async fn add_group<T: AsyncCisClientTrait>(
    cis_client: web::Data<T>,
    pool: web::Data<Pool>,
    scope_and_user: ScopeAndUser,
    new_group: web::Json<NewGroup>,
) -> Result<HttpResponse, ApiError> {
    let new_group = new_group.into_inner().checked()?;
    info!("trying to create new group: {}", new_group.name);
    operations::groups::add_new_group(&pool, &scope_and_user, new_group, Arc::clone(&*cis_client))
        .await?;
    Ok(HttpResponse::Created().json(""))
}

#[guard(Staff, Creator, Medium)]
async fn delete_group<T: AsyncCisClientTrait>(
    cis_client: web::Data<T>,
    pool: web::Data<Pool>,
    group_name: web::Path<String>,
    scope_and_user: ScopeAndUser,
) -> Result<HttpResponse, ApiError> {
    operations::groups::delete_group(&pool, &scope_and_user, &group_name, Arc::clone(&cis_client))
        .await?;
    Ok(HttpResponse::Created().json(""))
}

#[guard(Authenticated)]
async fn group_details(
    pool: web::Data<Pool>,
    group_name: web::Path<String>,
    scope_and_user: ScopeAndUser,
) -> Result<HttpResponse, ApiError> {
    let host = operations::users::user_by_id(&pool, &scope_and_user.user_id)?;
    let membership =
        operations::members::membership_and_scoped_host(&pool, &scope_and_user, &group_name)?;
    let role = membership.as_ref().map(|m| m.role);
    let super_user = scope_and_user.groups_scope == GroupsTrust::Admin;
    let curator = role.as_ref().map(|r| r.is_curator()).unwrap_or_default() || super_user;
    let is_member = membership.is_some();
    let member_count = match operations::members::member_count(&pool, &group_name) {
        Ok(member_count) => member_count,
        Err(e) => return Err(ApiError::GenericBadRequest(e)),
    };
    let group = operations::groups::get_group_with_terms_flag(&pool, &group_name)?;
    let invitation_count = if curator {
        Some(operations::invitations::pending_invitations_count(
            &pool,
            &scope_and_user,
            &group_name,
            &host,
        )?)
    } else {
        None
    };
    let renewal_count = if curator {
        let renewals = operations::members::renewal_count(&pool, &group_name, None)?;
        if group.group.group_expiration.unwrap_or_default() == 0 && renewals == 0 {
            None
        } else {
            Some(renewals)
        }
    } else {
        None
    };
    let request_count = if curator && group.group.typ == GroupType::Reviewed {
        Some(operations::requests::request_count(
            &pool,
            &scope_and_user,
            &group_name,
        )?)
    } else {
        None
    };
    let result = DisplayGroupDetails {
        membership,
        super_user,
        curator,
        member: is_member,
        group: GroupInfo {
            name: group.group.name,
            description: group.group.description,
            typ: group.group.typ,
            expiration: if curator {
                group.group.group_expiration.or(Some(0))
            } else {
                None
            },
            created: group.group.created,
            terms: group.terms,
            trust: group.group.trust,
        },
        member_count,
        invitation_count,
        renewal_count,
        request_count,
    };
    Ok(HttpResponse::Ok().json(&result))
}

pub fn groups_app<T: AsyncCisClientTrait + 'static>() -> impl HttpServiceFactory {
    web::scope("/groups")
        .service(
            web::resource("")
                .route(web::post().to(add_group::<T>))
                .route(web::get().to(list_groups)),
        )
        .service(
            web::resource("/{group_name}")
                .route(web::get().to(get_group))
                .route(web::put().to(update_group))
                .route(web::delete().to(delete_group::<T>)),
        )
        .service(web::resource("/{group_name}/details").route(web::get().to(group_details)))
}
