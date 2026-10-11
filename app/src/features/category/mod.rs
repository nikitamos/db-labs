use axum::{
    Json, Router,
    extract::FromRef,
    routing::{delete, post},
};
use serde::{Deserialize, Serialize};

use crate::{
    entities::category::Category,
    features::category::service::{CategoryService},
};

pub mod service;

pub fn routes<R, State>() -> Router<State>
where
    R: service::CategoryService + FromRef<State>,
    State: Clone + Send + Sync + 'static,
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

pub async fn create_category0<Repo: CategoryService + Send + Sync + Clone>(
    axum::extract::State(repo): axum::extract::State<Repo>,
    Json(body): Json<CreateCategoryDto>,
) -> Result<Json<Category>, crate::HttpError> {
    Ok(Json::<Category>::from(repo.create_category(body).await?))
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateCategoryDto {
    pub name: String,
    pub badge_color: i32,
    pub urgency: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateCategoryDto {
    pub name: Option<String>,
    pub badge_color: Option<i32>,
    pub urgency: Option<i32>,
}
