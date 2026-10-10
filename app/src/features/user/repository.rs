use std::time::Duration;

use axum::extract::{FromRef, Path};

use crate::AppState;

// #[autoendpoint::endpoints(error=crate::HttpError)]
pub trait UserRepository {
    // #[endpoint(ok=i32)]
    async fn xxxx(self, /*#[extract(Path)]*/ id: i32) -> crate::Result<i32>;
}

#[derive(Clone)]
pub struct TestUserRepo;
impl UserRepository for TestUserRepo {
    async fn xxxx(self, id: i32) -> crate::Result<i32> {
        tokio::time::sleep(Duration::from_millis(852)).await;
        return Ok(9*id);
    }
}

// fn x() {
//     let x = UserRepositoryEndpoints::;
// }
#[derive(Default)]
pub struct UserRepositoryEndpoints<Repo: UserRepository>(
    core::marker::PhantomData<Repo>,
);
impl<Repo: UserRepository> UserRepositoryEndpoints<Repo>
{
    pub async fn xxxx(
        axum::extract::State(state): axum::extract::State<Repo>,
        Path(id): Path<i32>,
    ) -> Result<String, crate::HttpError> {
        let repo = state;
        Ok(i32::from(repo.xxxx(id).await?).to_string())
    }
}
