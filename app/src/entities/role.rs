use sqlx::{Executor, Postgres};

#[derive(Debug, PartialEq, Eq)]
pub struct Permission {
    pub name: String,
}

impl Permission {}

pub struct Role {
    pub name: String,
    pub permissions: Vec<Permission>,
}

impl Role {
    pub async fn fetch_for_user<'a>(
        user_id: i32,
        executor: impl Executor<'a, Database = Postgres>,
    ) -> crate::Result<Vec<Role>> {
        let mut roles: Vec<Role> = vec![];
        let records = sqlx::query!(
            r#"
            SELECT
                role_name       AS "role_name!",
                permission_name AS "permission_name!"
            FROM read_user_roles($1, $2)
            ORDER BY role_name
        "#,
            crate::ACTOR_ID,
            user_id
        )
        .fetch_all(executor)
        .await?;

        // TODO: less copying
        let mut i = 0;
        while i < records.len() {
            let cur_role = records[i].role_name.as_ref();
            let mut j = i + 1;
            while let Some(next_role) = roles.get(j)
                && next_role.name == cur_role
            {
                j += 1;
            }

            roles.push(Role {
                name: records[i].role_name.clone(),
                permissions: records[i..j]
                    .iter()
                    .map(|x| Permission {
                        name: x.permission_name.clone(),
                    })
                    .collect(),
            });

            i = j;
        }
        Ok(roles)
    }
}
