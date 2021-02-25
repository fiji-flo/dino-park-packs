use crate::db::internal;
use crate::db::Pool;
use chrono::DateTime;
use chrono::Utc;
use cis_client::AsyncCisClientTrait;
use cis_profile::crypto::SecretStore;
use cis_profile::crypto::Signer;
use cis_profile::schema::AccessInformationProviderSubObject;
use cis_profile::schema::Display;
use cis_profile::schema::KeyValue;
use cis_profile::schema::Profile;
use cis_profile::schema::PublisherAuthority;
use failure::format_err;
use failure::Error;
use futures::TryFutureExt;
use std::collections::BTreeMap;
use std::sync::Arc;
use uuid::Uuid;

fn update_groups_and_sign(
    field: &mut AccessInformationProviderSubObject,
    groups: Vec<String>,
    store: &SecretStore,
    now: &DateTime<Utc>,
) -> Result<(), Error> {
    if field.values.is_none() {
        field.metadata.created = *now;
    }
    field.values = Some(KeyValue({
        let mut btm = BTreeMap::new();
        for group in groups {
            btm.insert(group, Some(String::default()));
        }
        btm
    }));
    if field.metadata.display.is_none() {
        field.metadata.display = Some(Display::Staff);
    }
    field.metadata.last_modified = *now;
    field.signature.publisher.name = PublisherAuthority::Mozilliansorg;
    store.sign_attribute(field).map_err(Into::into)
}

pub async fn _send_groups_to_cis(
    cis_client: Arc<impl AsyncCisClientTrait>,
    groups: Vec<String>,
    profile: Profile,
) -> Result<(), Error> {
    let now = &Utc::now();
    let mut update_profile = Profile::default();
    update_profile.access_information.mozilliansorg = profile.access_information.mozilliansorg;
    update_profile.active = profile.active;
    match update_groups_and_sign(
        &mut update_profile.access_information.mozilliansorg,
        groups,
        cis_client.get_secret_store(),
        &now,
    ) {
        Ok(_) => {
            if let Some(user_id) = profile.user_id.value.clone() {
                cis_client
                    .update_user(&user_id, update_profile)
                    .map_ok(|_| ())
                    .await.map_err(Into::into)
            } else {
                Err(format_err!("invalid user_id"))
            }
        }
        Err(e) => Err(e),
    }
}

pub async fn send_groups_to_cis(
    pool: &Pool,
    cis_client: Arc<impl AsyncCisClientTrait>,
    user_uuid: &Uuid,
) -> Result<(), Error> {
    let connection = pool.get()?;
    let user_profile = internal::user::user_profile_by_uuid(&connection, user_uuid)?;
    let groups = internal::member::group_names_for_user(&connection, user_uuid)?;
    drop(connection);
    _send_groups_to_cis(cis_client, groups, user_profile.profile).await
}
