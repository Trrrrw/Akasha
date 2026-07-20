use std::{env, net::SocketAddr};

use akasha_db::DbOptions;
use anyhow::{Context, Result};

#[derive(Clone)]
pub struct Config {
    pub bind_addr: SocketAddr,
    pub database: DbOptions,
    pub auth: AuthConfig,
    pub github: GitHubConfig,
    pub worker: WorkerConfig,
}

#[derive(Clone)]
pub struct AuthConfig {
    pub jwt_secret: String,
    pub token_hash_secret: String,
}

#[derive(Clone)]
pub struct GitHubConfig {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_url: String,
    pub admin_github_id: Option<u64>,
}

#[derive(Clone)]
pub struct WorkerConfig {
    pub token: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            bind_addr: env_or("BIND_ADDR", "0.0.0.0:7040")
                .parse()
                .context("BIND_ADDR must be a socket address")?,

            database: DbOptions {
                pg_host: env_or("POSTGRES_HOST", "127.0.0.1"),
                pg_port: env_or("POSTGRES_PORT", "5432"),
                pg_user: required("POSTGRES_USER")?,
                pg_password: required("POSTGRES_PASSWORD")?,
                pg_database: env_or("POSTGRES_DB", "Akasha"),
            },

            auth: AuthConfig {
                jwt_secret: required("JWT_SECRET")?,
                token_hash_secret: required("TOKEN_HASH_SECRET")?,
            },

            github: GitHubConfig {
                client_id: required("GITHUB_CLIENT_ID")?,
                client_secret: required("GITHUB_CLIENT_SECRET")?,
                redirect_url: required("GITHUB_OAUTH_REDIRECT_URL")?,
                admin_github_id: env::var("ADMIN_GITHUB_ID")
                    .ok()
                    .map(|value| value.parse())
                    .transpose()
                    .context("ADMIN_GITHUB_ID must be an unsigned integer")?,
            },

            worker: WorkerConfig {
                token: required("WORKER_TOKEN")?,
            },
        })
    }
}

fn required(key: &str) -> Result<String> {
    env::var(key).with_context(|| format!("missing required environment variable {key}"))
}

fn env_or(key: &str, default: &str) -> String {
    env::var(key).unwrap_or_else(|_| default.to_owned())
}
