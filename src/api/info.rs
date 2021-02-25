use crate::api::error::ApiError;
use crate::db::operations;
use crate::db::Pool;
use actix_web::dev::HttpServiceFactory;
use actix_web::web;
use actix_web::HttpResponse;
use dino_park_gate::groups::Groups;
use dino_park_gate::groups::GroupsFromToken;
use dino_park_gate::provider::Provider;
use dino_park_gate::scope::ScopeAndUser;
use serde::Serialize;
use std::collections::HashSet;

#[derive(Serialize)]
pub struct AllGroups {
    cis: Vec<String>,
    dino_park: Vec<String>,
    diff: Vec<String>,
}

#[guard(Authenticated)]
async fn groups(
    pool: web::Data<Pool>,
    scope_and_user: ScopeAndUser,
    groups: Groups,
) -> Result<HttpResponse, ApiError> {
    let mut dino_park = operations::groups::groups_for_current_user(&pool, &scope_and_user)?;
    dino_park.sort();
    let mut cis: Vec<String> = groups
        .groups
        .into_iter()
        .filter_map(|g| g.strip_prefix("mozilliansorg_").map(String::from))
        .collect();
    cis.sort();

    let mut diff: Vec<String> = dino_park
        .iter()
        .cloned()
        .collect::<HashSet<_>>()
        .symmetric_difference(&cis.iter().cloned().collect::<HashSet<_>>())
        .cloned()
        .collect();
    diff.sort();

    Ok(HttpResponse::Ok().json(&AllGroups {
        cis,
        dino_park,
        diff,
    }))
}

pub fn info_app(provider: Provider) -> impl HttpServiceFactory {
    let middleware = GroupsFromToken::new(provider);
    web::scope("/info")
        .wrap(middleware)
        .service(web::resource("/groups").route(web::get().to(groups)))
}
