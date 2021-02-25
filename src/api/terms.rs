use crate::api::error::ApiError;
use crate::db::operations;
use crate::db::Pool;
use actix_web::dev::HttpServiceFactory;
use actix_web::web;
use actix_web::HttpResponse;
use dino_park_gate::scope::ScopeAndUser;
use serde::Deserialize;

const TERMS_MAX_LEN: usize = 7000;

#[derive(Deserialize)]
pub struct TermsUpdate {
    text: String,
}

impl TermsUpdate {
    pub fn checked(self) -> Result<String, ApiError> {
        if self.text.len() <= TERMS_MAX_LEN {
            Ok(self.text)
        } else {
            Err(ApiError::InputToLong)
        }
    }
}

#[guard(Authenticated)]
async fn view_terms(
    pool: web::Data<Pool>,
    group_name: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    match operations::terms::get_terms(&pool, &group_name) {
        Ok(terms) => Ok(HttpResponse::Ok().json(&terms)),
        Err(e) => Err(ApiError::GenericBadRequest(e)),
    }
}

#[guard(Ndaed, None, Medium)]
async fn delete_terms(
    pool: web::Data<Pool>,
    group_name: web::Path<String>,
    scope_and_user: ScopeAndUser,
) -> Result<HttpResponse, ApiError> {
    match operations::terms::delete_terms(&pool, &scope_and_user, &group_name) {
        Ok(_) => Ok(HttpResponse::Created().json("")),
        Err(e) => Err(ApiError::GenericBadRequest(e)),
    }
}

#[guard(Ndaed, None, Medium)]
async fn update_terms(
    pool: web::Data<Pool>,
    group_name: web::Path<String>,
    terms_update: web::Json<TermsUpdate>,
    scope_and_user: ScopeAndUser,
) -> Result<HttpResponse, ApiError> {
    match operations::terms::update_terms(
        &pool,
        &scope_and_user,
        &group_name,
        terms_update.into_inner().checked()?,
    ) {
        Ok(_) => Ok(HttpResponse::Created().json("")),
        Err(e) => Err(ApiError::GenericBadRequest(e)),
    }
}

pub fn terms_app() -> impl HttpServiceFactory {
    web::scope("/terms").service(
        web::resource("/{group_name}")
            .route(web::get().to(view_terms))
            .route(web::put().to(update_terms))
            .route(web::delete().to(delete_terms))
            .route(web::post().to(update_terms)),
    )
}
