use crate::auth::auth_v2::AuthServiceV2;
use crate::auth::AuthService;

pub struct ApiData<T> {
    pub auth: AuthService,
    pub api: T,
}

pub struct ApiDataV2<T> {
    pub auth: AuthServiceV2,
    pub api: T,
}
