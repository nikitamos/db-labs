use std::time::{SystemTime, UNIX_EPOCH};

use aide::OperationInput;
use axum::extract::{FromRef, FromRequestParts, State};
use axum::http::request::Parts;
use axum::http::{self, StatusCode};
use indexmap::IndexMap;
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};

use crate::HttpError;
pub use super::UserId;

#[derive(Debug, Deserialize, Serialize)]
pub struct Claims {
    pub user_id: i32,
    pub exp: usize,
}

#[derive(Debug, Clone)]
pub struct JwtAuthState {
    decoding_key: DecodingKey,
    validation: Validation,
    encoding_key: EncodingKey,
}

impl JwtAuthState {
    pub fn from_secret(secret: &[u8]) -> Self {
        let validation = Validation::new(Algorithm::HS256);
        Self {
            decoding_key: DecodingKey::from_secret(secret).expect("failed to create JWT key"),
            encoding_key: EncodingKey::from_secret(secret).expect("failed to create JWT key"),
            validation,
        }
    }
    pub fn generate_token(
        &self,
        user_id: i32,
        expires_in_secs: u64,
    ) -> Result<String, jsonwebtoken::errors::Error> {
        let exp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize
            + expires_in_secs as usize;

        let claims = Claims { user_id, exp };
        encode(&Header::default(), &claims, &self.encoding_key)
    }
}

fn verify_token(token: &str, jwt_state: &JwtAuthState) -> crate::Result<i32> {
    let token_data = decode::<Claims>(token, &jwt_state.decoding_key, &jwt_state.validation)
        .map_err(|error| {
            tracing::warn!(message="invalid JWT token passed", verification = ?error);
            crate::Error::InvalidAuthToken
        })?;
    Ok(token_data.claims.user_id)
}

impl<S> FromRequestParts<S> for UserId
where
    S: Send + Sync,
    JwtAuthState: FromRef<S>,
{
    type Rejection = HttpError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let State(jwt_state) = State::<JwtAuthState>::from_request_parts(parts, state)
            .await
            .map_err(|_| HttpError::new(StatusCode::UNAUTHORIZED, "missing auth token"))?;

        let token = parts
            .headers
            .get(http::header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .ok_or(HttpError::new(
                StatusCode::UNAUTHORIZED,
                "Missing auth token",
            ))?;

        if !token.starts_with("Bearer ") {
            return Err(HttpError::new(StatusCode::UNAUTHORIZED, "invalid auth protocol").into());
        }
        let token = &token[7..];

        let user_id = verify_token(token, &jwt_state)?;
        Ok(UserId { user_id })
    }
}

impl OperationInput for UserId {
    fn inferred_early_responses(
        _ctx: &mut aide::generate::GenContext,
        operation: &mut aide::openapi::Operation,
    ) -> Vec<(Option<u16>, aide::openapi::Response)> {
        let entry = ("bearerAuth".to_string(), Vec::new());
        operation.security.push(std::iter::once(entry).collect());
        vec![]
    }
}
