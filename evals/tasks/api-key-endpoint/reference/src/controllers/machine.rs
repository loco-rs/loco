use loco_rs::prelude::*;

use crate::{models::users, views::machine::MachineIdentity};

/// Identify the calling machine client.
///
/// `auth::ApiToken<users::Model>` is the framework's seam for API-key auth: it
/// resolves the key through `Authenticable` and rejects an unknown or missing
/// one with 401 before this body runs. Parsing the Authorization header here,
/// or looking the user up by `api_key` in the handler, would reimplement it.
#[debug_handler]
async fn whoami(
    auth: auth::ApiToken<users::Model>,
    State(_ctx): State<AppContext>,
) -> Result<Response> {
    format::json(MachineIdentity::from(&auth.user))
}

pub fn routes() -> Routes {
    Routes::new().prefix("/api/machine").add("/whoami", get(whoami))
}
