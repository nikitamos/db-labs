use aide::axum::routing::{get_with as get,  post_with as post};
use autoendpoint::endpoints;
use axum::extract::{Json, Path};
use sqlx::PgPool;

use crate::{
    entities::{session::Session, user::User}, extract::WithRejection, features::{
        auth::{ChangePasswordDto, SessionDto, SigninDto, SignupDto, jwt::JwtAuthState}, user::service::{SqlxUserService, UserService},
    }, with,
};

type Unit = ();

#[endpoints(error=crate::HttpError, router=aide::axum::ApiRouter, method=get, tag="auth")]
#[async_trait::async_trait]
pub trait AuthService: Clone + Send + Sync + 'static {
    #[endpoint(ok=Json::<User>, path="/signup", method=post)]
    async fn signup(&self, #[extract(WithRejection<Json>)] dto: SignupDto) -> crate::Result<User>;

    #[endpoint(ok=Json::<SessionDto>, path="/login", method=post)]
    async fn login(&self, #[extract(WithRejection<Json>)] dto: SigninDto) -> crate::Result<SessionDto>;

    #[endpoint(ok=Unit,method=post)]
    async fn change_password(
        &self,
        #[extract(Path)] id: i32,
        #[extract(WithRejection<Json>)] dto: ChangePasswordDto,
    ) -> crate::Result<()>;

    async fn logout(&self, session_id: i32) -> crate::Result<()>;
}

#[derive(Debug, Clone)]
pub struct SqlxAuthService {
    pool: PgPool,
    user_service: SqlxUserService,
    jwt_state: JwtAuthState,
    cost: u32,
}

impl SqlxAuthService {
    pub fn new(pool: PgPool, user_service: SqlxUserService, jwt_state: JwtAuthState) -> Self {
        Self {
            pool,
            user_service,
            #[cfg(debug_assertions)]
            cost: 4,
            #[cfg(not(debug_assertions))]
            cost: 12,
            jwt_state
        }
    }
}

#[async_trait::async_trait]
impl AuthService for SqlxAuthService {
    async fn signup(&self, dto: SignupDto) -> crate::Result<User> {
        let password_hash = bcrypt::hash(&dto.password, self.cost)?;
        let mut tx = self.pool.begin().await?;
        let user = sqlx::query_as!(
            User,
            r#"
            SELECT
                id,
                username         AS "username!",
                password_hash,
                email            AS "email!",
                first_name       AS "first_name!",
                second_name      AS "second_name!",
                bio
            FROM write_user($1, $2, $3, $4, $5, $6, $7)
            "#,
            crate::ACTOR_ID,
            dto.username,
            password_hash,
            dto.email,
            dto.first_name,
            dto.second_name,
            dto.bio
        )
        .fetch_one(&mut *tx)
        .await?;
        sqlx::query!(
            "SELECT write_user_role($1, $2, $3)",
            crate::ACTOR_ID,
            user.id,
            "admin"
        )
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;

        Ok(with!(user: password_hash = None))
    }

    async fn login(&self, dto: SigninDto) -> crate::Result<SessionDto> {
        let user = self
            .user_service
            .get_by_username_or_email(dto.identity)
            .await?;
        user.check_password(dto.password)?;
        let token = self.jwt_state.generate_token(user.id.unwrap(), 6000)?;
        let session = sqlx::query_as!(
            Session,
            r#"
            WITH inserted AS (
                INSERT INTO sessions (user_id, auth_token)
                VALUES ($1, $2)
                RETURNING sessions.user_id, sessions.auth_token, sessions.id
            )
            SELECT
                inserted.user_id AS "user",
                inserted.auth_token,
                inserted.id
            FROM inserted
            "#,
            user.id,
            token
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(SessionDto {
            access_token: session.auth_token,
        })
    }

    async fn logout(&self, session_id: i32) -> crate::Result<()> {
        let affected = sqlx::query_scalar!(
            r#"SELECT modify_session($1, $2) AS "affected!""#,
            crate::ACTOR_ID,
            session_id
        )
        .fetch_one(&self.pool)
        .await?;
        if affected == 0 {
            tracing::warn!("attempt to remove non-existent session");
        }
        Ok(())
    }

    async fn change_password(&self, id: i32, dto: ChangePasswordDto) -> crate::Result<()> {
        let user = self.user_service.get_by_id(id).await?;
        user.check_password(dto.old_password)?;
        let hash = bcrypt::hash(dto.new_password, 12)?;
        let affected = sqlx::query_scalar!(
            r#"SELECT modify_user_password($1, $2, $3) AS "affected!""#,
            crate::ACTOR_ID,
            hash,
            user.id
        )
        .fetch_one(&self.pool)
        .await?;
        if affected == 0 {
            tracing::warn!("attempt to update password for non-existent user");
        }
        Ok(())
    }
}
