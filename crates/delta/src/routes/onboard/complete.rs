use authifier::models::Session;
use once_cell::sync::Lazy;
use regex::Regex;
use revolt_config::config;
use revolt_database::{Database, Member, User};
use revolt_models::v0;
use revolt_result::{create_error, Result};

use rocket::{serde::json::Json, State};
use serde::{Deserialize, Serialize};
use validator::Validate;

/// Regex for valid usernames
///
/// Block zero width space
/// Block lookalike characters
pub static RE_USERNAME: Lazy<Regex> = Lazy::new(|| Regex::new(r"^(\p{L}|[\d_.-])+$").unwrap());

/// # New User Data
#[derive(Validate, Serialize, Deserialize, JsonSchema)]
pub struct DataOnboard {
    /// New username which will be used to identify the user on the platform
    #[validate(length(min = 2, max = 32), regex = "RE_USERNAME")]
    username: String,
}

/// # Complete Onboarding
///
/// This sets a new username, completes onboarding and allows a user to start using Revolt.
#[openapi(tag = "Onboarding")]
#[post("/complete", data = "<data>")]
pub async fn complete(
    db: &State<Database>,
    session: Session,
    user: Option<User>,
    data: Json<DataOnboard>,
) -> Result<Json<v0::User>> {
    if user.is_some() {
        return Err(create_error!(AlreadyOnboarded));
    }

    let data = data.into_inner();
    data.validate().map_err(|error| {
        create_error!(FailedValidation {
            error: error.to_string()
        })
    })?;

    let user = User::create(db, data.username, session.user_id, None).await?;

    // Auto-join the user to the default server if configured
    let cfg = config().await;
    if let Some(ref server_id) = cfg.api.registration.default_server {
        if !server_id.is_empty() {
            if let Ok(server) = db.fetch_server(server_id).await {
                // Silently ignore errors (e.g. already a member, banned)
                let _ = Member::create(db, &server, &user, None).await;
            }
        }
    }

    Ok(Json(user.into_self(false).await))
}
