use crate::helpers::api::post;
use crate::helpers::misc::Soa;
use crate::helpers::users::user_uuid;
use actix_http::Request;
use actix_web::dev::*;
use cis_profile::schema::Profile;
use serde_json::json;

pub async fn add_to_group<S, B, E>(mut app: &mut S, scope: &Soa, user: &Profile, group: &str)
where
    S: Service<Request, Response = ServiceResponse<B>, Error = E>,
    E: std::fmt::Debug,
{
    let res = post(
        &mut app,
        &format!("/groups/api/v1/sudo/member/{}", group),
        json!({ "user_uuid": user_uuid(&user) }),
        &scope.clone().admin(),
    )
    .await;
    assert!(res.status().is_success());
}
