mod config;
mod error;
mod handlers;
mod hash_storage;
mod middleware;

use crate::config::{AuthConfig, Config};
use crate::hash_storage::HashStorage;
use anyhow::anyhow;
use anyhow::Context;
use axum::{routing::get, Extension, Router};
use envconfig::Envconfig;
use std::path::PathBuf;
use tracing::info;
use tracing_subscriber::EnvFilter;

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let config = Config::init_from_env().context("Failed to load config")?;

    let auth_config = if config.disable_auth {
        AuthConfig::Disabled
    } else {
        AuthConfig::Enabled {
            username: config.basic_auth_username,
            password: config.basic_auth_password,
        }
    };

    let storage_folder = PathBuf::from(config.storage_folder);
    if !storage_folder.exists() {
        return Err(anyhow!(
            "Provided storage folder {storage_folder:?} doesn't exist",
        ));
    }

    let hash_filename = storage_folder.join(hash_storage::FILENAME);
    let hash_storage = HashStorage::new(hash_filename)
        .await
        .context("Could not load token storage")?;

    let app = Router::new()
        .route(
            "/",
            get(handlers::index)
                .post(handlers::upload)
                .layer(axum::middleware::from_fn(middleware::auth)),
        )
        .route("/:file_name", get(handlers::view))
        .layer(Extension(auth_config))
        .layer(Extension(storage_folder))
        .layer(Extension(hash_storage));

    info!("Listening on {}", config.listen_address);

    axum::Server::bind(
        &config
            .listen_address
            .parse()
            .context("Invalid listen address")?,
    )
    .serve(app.into_make_service())
    .await?;

    Ok(())
}
