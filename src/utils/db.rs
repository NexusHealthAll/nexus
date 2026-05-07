use sqlx::PgPool;

/// Shared database state passed through Axum's state extractor.
#[derive(Clone)]
pub struct Database {
    pub pool: PgPool,
}

impl Database {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}
