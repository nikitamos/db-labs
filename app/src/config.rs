use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub app_db_uri: String,
    pub port: String
}