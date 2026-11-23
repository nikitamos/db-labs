use aide::{OperationInput, OperationOutput};
use axum::{
    body::Body,
    extract::{FromRequest, FromRequestParts, rejection::JsonRejection},
    http::StatusCode,
};

use crate::HttpError;

pub struct WithRejection<R>(pub R);

impl<R, S> FromRequestParts<S> for WithRejection<R>
where
    S: Send + Sync,
    R: FromRequestParts<S>,
    HttpError: From<R::Rejection>,
{
    type Rejection = HttpError;

    fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &S,
    ) -> impl Future<Output = Result<Self, Self::Rejection>> + Send {
        async {
            let res = R::from_request_parts(parts, state).await?;
            Ok(Self(res))
        }
    }
}

impl<R, S> FromRequest<S> for WithRejection<R>
where
    S: Send + Sync,
    R: FromRequest<S>,
    HttpError: From<R::Rejection>,
{
    type Rejection = HttpError;

    fn from_request(
        body: axum::http::request::Request<Body>,
        state: &S,
    ) -> impl Future<Output = Result<Self, Self::Rejection>> + Send {
        async {
            let res = R::from_request(body, state).await?;
            Ok(Self(res))
        }
    }
}

impl From<JsonRejection> for HttpError {
    fn from(value: JsonRejection) -> Self {
        HttpError::new(StatusCode::BAD_REQUEST, value.to_string())
    }
}
impl<R: OperationInput> OperationInput for WithRejection<R> {
    fn operation_input(ctx: &mut aide::generate::GenContext, operation: &mut aide::openapi::Operation) {
        R::operation_input(ctx, operation)
    }
    fn inferred_early_responses(
        ctx: &mut aide::generate::GenContext,
        operation: &mut aide::openapi::Operation,
    ) -> Vec<(Option<u16>, aide::openapi::Response)>
    {
        HttpError::inferred_responses(ctx, operation)
    }
}
