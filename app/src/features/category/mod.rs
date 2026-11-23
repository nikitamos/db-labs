use axum::{
    Router,
    extract::FromRef,
    routing::{delete, post},
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::features::auth::jwt::JwtAuthState;

pub mod service;

pub fn routes<R, State>() -> Router<State>
where
    R: service::CategoryService + FromRef<State>,
    State: Clone + Send + Sync + 'static,
    JwtAuthState: FromRef<State>
{
    type EP<R> = service::CategoryServiceEndpoints<R>;
    Router::new()
        .route(
            "/",
            post(EP::<R>::create_category).get(EP::<R>::get_list),
        )
        .route(
            "/{id}",
            delete(EP::<R>::delete_category)
                .post(EP::<R>::update_category)
                .get(EP::<R>::get_by_id),
        )
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct CreateCategoryDto {
    pub name: String,
    pub badge_color: i32,
    pub urgency: i32,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct UpdateCategoryDto {
    pub name: Option<String>,
    pub badge_color: Option<i32>,
    pub urgency: Option<i32>,
}
