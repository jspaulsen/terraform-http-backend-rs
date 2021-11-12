use chrono::NaiveDateTime;
use sqlx::{
    Error as SqlxError,
    SqlitePool,
};


#[derive(sqlx::FromRow, Debug)]
pub struct TerraformRow {
    pub id: String,
    pub state: String,
    pub last_update_ts: NaiveDateTime,
}


#[derive(sqlx::FromRow, Debug)]
pub struct TerraformLockRow {
    pub id: String,
    pub terraform_id: String,
    pub state: String,
    pub last_update_ts: NaiveDateTime,
}


pub struct TerraformQuery {
    pool: SqlitePool,
}


#[derive(Debug)]
pub enum MaybeConflictError {
    Conflict(TerraformLockRow),
    Error(SqlxError),
}


impl From<SqlxError> for MaybeConflictError {
    fn from(e: SqlxError) -> Self {
        Self::Error(e)
    }
}


impl TerraformQuery {
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            pool
        }
    }

    /// Returns an optional terraform row for a given terraform id
    pub async fn get<S: AsRef<str>>(&self, id: S) -> Result<Option<TerraformRow>, SqlxError> {
        sqlx::query_as::<_, TerraformRow>("SELECT * FROM terraform WHERE id = ?1")
            .bind(id.as_ref())
            .fetch_optional(&self.pool)
            .await
    }

    pub async fn create_or_replace<S: AsRef<str>>(&self, id: S, state: S) -> Result<TerraformRow, SqlxError> {
        let query = "INSERT INTO terraform (id, state) VALUES (?1, ?2) \
            ON CONFLICT (id) DO UPDATE SET state=excluded.state \
            RETURNING *";

        sqlx::query_as::<_, TerraformRow>(query)
            .bind(id.as_ref())
            .bind(state.as_ref())
            .fetch_one(&self.pool)
            .await
    }

    pub async fn get_lock_by_terraform_id<S: AsRef<str>>(&self, terraform_id: S) -> Result<Option<TerraformLockRow>, SqlxError> {
        sqlx::query_as::<_, TerraformLockRow>("SELECT * FROM locks WHERE terraform_id = ?1")
            .bind(terraform_id.as_ref())
            .fetch_optional(&self.pool)
            .await
    }

    /// Attempts to obtain the lock on a terraform resource, returning None
    pub async fn lock<S: AsRef<str>>(&self, terraform_id: S, lock_id: S, state: S) -> Result<TerraformLockRow, MaybeConflictError> {
        let query = "INSERT INTO locks (id, terraform_id, state) VALUES (?1, ?2, ?3) ON CONFLICT (terraform_id) DO NOTHING; \
            SELECT * FROM locks WHERE terraform_id = ?2";

        let result: TerraformLockRow = sqlx::query_as::<_, TerraformLockRow>(query)
            .bind(lock_id.as_ref())
            .bind(terraform_id.as_ref())
            .bind(state.as_ref())
            .fetch_one(&self.pool)
            .await?;

        if lock_id.as_ref() != result.id {
            Err(MaybeConflictError::Conflict(result))
        } else {
            Ok(result)
        }
    }

    /// Attempts to unlock a terraform resource, returning None if the resource
    /// lock was not found
    pub async fn unlock<S: AsRef<str>>(&self, terraform_id: S, lock_id: S) -> Result<(), SqlxError> {
        let query = "DELETE FROM locks WHERE id = ?1 AND terraform_id = ?2 RETURNING *";
        sqlx::query(query)
            .bind(lock_id.as_ref())
            .bind(terraform_id.as_ref())
            .execute(&self.pool)
            .await
            .map(|_| ())
    }
}


#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use envconfig::Envconfig;
    use sqlx::SqlitePool;
    use tokio;

    use super::TerraformQuery;
    use crate::{
        database,
        config::Configuration,
    };

    fn default_config() -> Configuration {
        let mut hashmap = HashMap::new();

        hashmap.insert("DATABASE_URI".to_string(), "sqlite::memory:".to_string());
        hashmap.insert("TF_HTTP_USERNAME".to_string(), "asdf".to_string());
        hashmap.insert("TF_HTTP_PASSWORD".to_string(), "asdf".to_string());

        Configuration::init_from_hashmap(&hashmap)
            .unwrap()
    }

    async fn get_migrated_pool(config: &Configuration) -> SqlitePool {
        let pool = database::get_db_pool(&config)
            .await
            .unwrap();

        database::MIGRATE.run(&pool)
            .await
            .unwrap();

        pool
    }

    #[tokio::test]
    async fn test_create_replace() {
        let config = default_config();
        let pool = get_migrated_pool(&config)
            .await;

        let query = TerraformQuery::new(pool);

        let id = "105";
        let initial_state = "initial-state";
        let secondary_state = "secondary-state";

        let initial_results = query.create_or_replace(id, initial_state)
            .await
            .expect("Failed to create initial_state");
        let get_initial = query.get(id)
            .await
            .expect("Failed to get initial_state")
            .expect("No row for initial_state");

        assert_eq!(initial_results.id, id);
        assert_eq!(get_initial.id, id);
        assert_eq!(initial_results.state, initial_state);
        assert_eq!(get_initial.state, initial_state);

        let secondary_results = query.create_or_replace(id, secondary_state)
            .await
            .expect("Failed to create initial_state");
        let get_secondary = query.get(id)
            .await
            .expect("Failed to get secondary_state")
            .expect("No row for secondary_state");

        assert_eq!(secondary_results.state, secondary_state);
        assert_eq!(get_secondary.state, secondary_state);
    }

    #[tokio::test]
    async fn test_lock() {
        let config = default_config();
        let pool = get_migrated_pool(&config)
            .await;

        let query = TerraformQuery::new(pool);

        let id = "105";
        let lock_id = "lock_id";

        let lock_results = query.lock(id, lock_id, "state")
            .await
            .expect("Failed to lock resource");

        let secondary_lock_results = query.lock(id, "different_lock_id", "state")
            .await;

        let get_lock_by_tf = query.get_lock_by_terraform_id(id)
            .await
            .expect("Failed to get lock")
            .expect("No lock row for resource");

        query.unlock(id, lock_id)
            .await
            .expect("Failed to unlock resource");

        assert_eq!(lock_results.id, lock_id);
        assert_eq!(get_lock_by_tf.terraform_id, id);

        // Should be locked
        assert!(secondary_lock_results.is_err());
    }
}

