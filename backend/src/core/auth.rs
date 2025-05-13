use axum::{
    extract::{FromRef, FromRequestParts},
    http::{StatusCode, header, request::Parts},
};

use super::state::AppState;

/// Injected into handlers after verification.
#[derive(Clone)]
pub struct ApiKey {
    #[allow(dead_code)]
    pub id: String,
    pub scope: String, // 'ro' | 'rw'
}

impl<S> FromRequestParts<S> for ApiKey
where
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = StatusCode;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        // Parse header
        let token = parts
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|h| h.to_str().ok())
            .and_then(|h| h.strip_prefix("Bearer "))
            .ok_or(StatusCode::UNAUTHORIZED)?
            .trim();

        // Check against DB
        let st: AppState = AppState::from_ref(state);
        match st.repo.verify_api_key(token).await {
            Ok(Some((id, scope))) => Ok(ApiKey { id, scope }),
            Ok(None) => Err(StatusCode::UNAUTHORIZED),
            Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
        }
    }
}
