use std::time::Duration;

use axum::extract::Path;


#[autoendpoint::endpoints(error=crate::HttpError)]
pub trait UserRepository {
    #[endpoint(ok=String)]
    async fn xxxx(self, #[extract(Path)] id: i32) -> crate::Result<String>;
}

#[derive(Clone)]
pub struct TestUserRepo;
impl UserRepository for TestUserRepo {
    async fn xxxx(self, id: i32) -> crate::Result<String> {
        tokio::time::sleep(Duration::from_millis(852)).await;
        Ok((9*id).to_string())
    }
}

// // fn x() {
// //     let x = UserRepositoryEndpoints::;
// // }
// #[derive(Default)]
// pub struct UserRepositoryEndpoints<Repo: UserRepository>(
//     core::marker::PhantomData<Repo>,
// );
// impl<Repo: UserRepository> UserRepositoryEndpoints<Repo>
// {
//     pub async fn xxxx(
//         axum::extract::State(state): axum::extract::State<Repo>,
//         Path(id): Path<i32>,
//     ) -> Result<String, crate::HttpError> {
//         let repo = state;
//         Ok(i32::from(repo.xxxx(id).await?).to_string())
//     }
// }
