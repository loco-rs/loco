pub mod cookie;

use crate::request_context::driver::cookie::SignedPrivateCookieJar;
use tower_sessions::Session;

pub const PRIVATE_COOKIE_NAME: &str = "__loco_app_session";
#[derive(Debug, Clone)]
pub enum Driver {
    TowerSession(Session),
    SignedPrivateCookieJar(Box<SignedPrivateCookieJar>),
}

impl Driver {}
