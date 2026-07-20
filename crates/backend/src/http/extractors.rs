use crate::{features::auth::token, http::error::AppError, state::AppState};
use axum::{
    extract::FromRequestParts,
    http::{header, request::Parts},
};

pub(crate) enum DataWriteActor {
    Admin { user_id: String },
    Worker,
}
impl DataWriteActor {
    pub(crate) fn label(&self) -> &str {
        match self {
            Self::Admin { user_id } => user_id,
            Self::Worker => "worker",
        }
    }
}
impl FromRequestParts<AppState> for DataWriteActor {
    type Rejection = AppError;
    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let value = parts
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .ok_or_else(|| AppError::Unauthorized("missing bearer token".into()))?;
        if value == state.config().worker.token {
            return Ok(Self::Worker);
        }
        let id = token::verify(&state.config().auth, value)?
            .sub
            .parse()
            .map_err(|_| AppError::Unauthorized("invalid access token subject".into()))?;
        let user = akasha_db::repositories::auth::find_current_user(state.db(), id)
            .await
            .map_err(|e| AppError::Internal(e.into()))?
            .ok_or_else(|| AppError::Unauthorized("invalid user".into()))?;
        if !user.is_admin {
            return Err(AppError::Forbidden("admin permission required".into()));
        }
        Ok(Self::Admin {
            user_id: user.id.to_string(),
        })
    }
}
