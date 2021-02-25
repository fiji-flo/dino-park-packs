use cis_client::error::ProfileError;
use cis_client::error::CisClientError;
use cis_client::getby::GetBy;
use cis_client::AsyncCisClientTrait;
use cis_client::CisFut;
use cis_profile::crypto::SecretStore;
use cis_profile::schema::Profile;
use dino_park_packs::db::operations::users::update_user_cache_unchecked;
use dino_park_packs::db::Pool;
use futures::future::err;
use futures::future::ok;
use serde_json::json;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::RwLock;

#[derive(Clone)]
pub struct CisFakeClient {
    pub store: Arc<RwLock<HashMap<String, Profile>>>,
    pub pool: Pool,
    pub secret_store: Arc<SecretStore>,
}

impl CisFakeClient {
    pub fn new(pool: Pool) -> Self {
        let secret_store = Arc::new(
            SecretStore::default()
                .with_sign_keys_from_inline_iter(vec![(
                    String::from("mozilliansorg"),
                    include_str!("../data/fake_key.json").to_owned(),
                )])
                .unwrap(),
        );
        CisFakeClient {
            store: Arc::new(RwLock::new(HashMap::new())),
            pool,
            secret_store,
        }
    }
}

#[allow(unused_variables)]
impl AsyncCisClientTrait for CisFakeClient {
    fn get_user_by(&self, id: &str, by: &GetBy, filter: Option<&str>) -> CisFut<Profile> {
        if let Some(p) = self.store.read().unwrap().get(id) {
            Box::pin(ok(p.clone()))
        } else {
            Box::pin(err(ProfileError::ProfileDoesNotExist.into()))
        }
    }
    fn get_any_user_by(&self, id: &str, by: &GetBy, filter: Option<&str>) -> CisFut<Profile> {
        unimplemented!()
    }
    fn get_inactive_user_by(&self, id: &str, by: &GetBy, filter: Option<&str>) -> CisFut<Profile> {
        unimplemented!()
    }
    fn update_user(&self, id: &str, profile: Profile) -> CisFut<Value> {
        let mut store = self.store.write().unwrap();
        let p = if let Some(mut p) = store.get_mut(id) {
            p.access_information.mozilliansorg = profile.access_information.mozilliansorg;
            p.clone()
        } else {
            store.insert(id.to_owned(), profile.clone());
            profile
        };
        match update_user_cache_unchecked(&self.pool, &p) {
            Ok(_) => Box::pin(ok(json!({}))),
            Err(e) => Box::pin(err(CisClientError::RuntimeError)),
        }
    }
    fn update_users(&self, profiles: &[Profile]) -> CisFut<Value> {
        unimplemented!()
    }
    fn delete_user(&self, id: &str, profile: Profile) -> CisFut<Value> {
        unimplemented!()
    }
    fn get_secret_store(&self) -> &SecretStore {
        &*self.secret_store
    }
}
