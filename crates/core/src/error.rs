#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("database: {0}")]
    Sql(#[from] rusqlite::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    /// The request did not get an answer GitHub meant: no network, a refused token, a 5xx.
    #[error("github: {0}")]
    Http(String),
    /// GitHub answered with GraphQL errors.
    #[error("github: {}", .0.join("; "))]
    GraphQl(Vec<String>),
    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, Error>;
