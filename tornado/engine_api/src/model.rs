use crate::auth::AuthService;

pub struct ApiData<T> {
    pub auth: AuthService,
    pub api: T,
}
