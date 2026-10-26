use aide::axum::routing::{post, get};
use autoendpoint::endpoints;
use axum::extract::{Json, Path};
use sqlx::PgPool;

use crate::{
    ResultExtensions,
    entities::{session::Session, user::User},
    features::{
        auth::{ChangePasswordDto, SessionDto, SigninDto, SignupDto},
        user::service::{SqlxUserService, UserService},
    },
    with,
};

type Unit = ();

#[endpoints(error=crate::HttpError, router=aide::axum::ApiRouter, method=get)]
#[async_trait::async_trait]
pub trait AuthService: Clone + Send + Sync + 'static {
    #[endpoint(ok=Json::<User>, path="/signup", method=post)]
    async fn signup(&self, #[extract(Json)] dto: SignupDto) -> crate::Result<User>;

    #[endpoint(ok=Json::<SessionDto>, path="/login", method=post)]
    async fn login(&self, #[extract(Json)] dto: SigninDto) -> crate::Result<SessionDto>;

    #[endpoint(ok=Unit,method=post)]
    async fn change_password(
        &self,
        #[extract(Path)] id: i32,
        #[extract(Json)] dto: ChangePasswordDto,
    ) -> crate::Result<()>;

    async fn logout(&self, session_id: i32) -> crate::Result<()>;
}

#[derive(Debug, Clone)]
pub struct SqlxAuthService {
    pool: PgPool,
    user_service: SqlxUserService,
}

impl SqlxAuthService {
    pub fn new(pool: PgPool, user_service: SqlxUserService) -> Self {
        Self { pool, user_service }
    }
}

#[async_trait::async_trait]
impl AuthService for SqlxAuthService {
    async fn signup(&self, dto: SignupDto) -> crate::Result<User> {
        let password_hash = bcrypt::hash(&dto.password, 12)?;
        Ok(sqlx::query_as!(
            User,
            r#"
            INSERT INTO users (
                username,
                password_hash,
                email,
                first_name,
                second_name,
                bio
                )
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING id, username, password_hash, email, first_name, second_name, bio
            "#,
            dto.username,
            password_hash,
            dto.email,
            dto.first_name,
            dto.second_name,
            dto.bio
        )
        .fetch_one(&self.pool)
        .await
        .map(|u| with!(u: password_hash = None))?)
    }

    async fn login(&self, dto: SigninDto) -> crate::Result<SessionDto> {
        let user = self
            .user_service
            .get_by_username_or_email(dto.identity)
            .await?;
        user.check_password(dto.password)?;
        let session = sqlx::query_as!(
            Session,
            r#"
            INSERT INTO sessions (user_id, auth_token)
            VALUES ($1, $2)
            RETURNING user_id as user, auth_token, id
            "#,
            user.id,
            "xxxx-not-implemented"
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(SessionDto {
            access_token: session.auth_token,
        })
    }

    async fn logout(&self, session_id: i32) -> crate::Result<()> {
        sqlx::query!("DELETE FROM sessions WHERE id=$1", session_id)
            .execute(&self.pool)
            .await
            .report_noop_delete("session")?;
        Ok(())
    }

    async fn change_password(&self, id: i32, dto: ChangePasswordDto) -> crate::Result<()> {
        let user = self.user_service.get_by_id(id).await?;
        user.check_password(dto.old_password)?;
        let hash = bcrypt::hash(dto.new_password, 12)?;
        sqlx::query_as!(
            Session,
            r#"
            UPDATE users
            SET password_hash = $1
            WHERE id = $2
            "#,
            hash,
            user.id
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(())
    }
}
