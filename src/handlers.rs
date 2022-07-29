use crate::error::Error;
use axum::{
    extract::{ContentLengthLimit, Host, Multipart, Path},
    http::{header::CONTENT_TYPE, HeaderMap},
    response::{Html, IntoResponse},
    Extension,
};
use futures::StreamExt;
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use std::path::PathBuf;
use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncWriteExt},
};
use tracing::trace;

const INDEX: &str = include_str!("../web/index.html");
const MAX_CONTENT_LENGTH: u64 = 2048 * 1024 * 1024;
const FILENAME_LENGTH: usize = 6;

pub async fn index() -> Html<&'static str> {
    Html(INDEX)
}

pub async fn view(
    Path(full_file_name): Path<String>,
    storage_folder: Extension<PathBuf>,
) -> Result<impl IntoResponse, Error> {
    let file_name = full_file_name
        .split_once('.')
        .map_or(full_file_name.as_str(), |(name, _)| name);

    let path = storage_folder.join(file_name);

    if !path.exists() {
        return Err(Error::FileNotFound);
    }

    let mut file = File::open(path).await?;
    let metadata = file.metadata().await?;

    let mut buf = Vec::with_capacity(metadata.len().try_into().unwrap());

    file.read_to_end(&mut buf).await?;

    let mut headers = HeaderMap::new();

    if let Some(info) = infer::get(&buf) {
        headers.insert(CONTENT_TYPE, info.mime_type().parse().unwrap());
    }

    Ok((headers, buf))
}

pub async fn upload(
    Extension(storage_folder): Extension<PathBuf>,
    mut multipart: ContentLengthLimit<Multipart, MAX_CONTENT_LENGTH>,
    host: Host,
) -> Result<String, Error> {
    let (file_name, path) = loop {
        let file_name = generate_filename();
        let path = storage_folder.join(&file_name);

        if !path.exists() {
            break (file_name, path);
        }
        trace!("File {file_name} already exists, regenerating");
    };

    let mut f = File::create(path).await?;

    if let Some(mut field) = multipart.0.next_field().await? {
        while let Some(maybe_chunk) = field.next().await {
            let chunk = maybe_chunk?;
            f.write_all(&chunk).await?;
        }

        let resulting_url = format!("{host}/{file_name}", host = host.0);

        Ok(resulting_url)
    } else {
        Err(Error::BadRequest("Missing form field".to_owned()))
    }
}

fn generate_filename() -> String {
    thread_rng()
        .sample_iter(&Alphanumeric)
        .take(FILENAME_LENGTH)
        .map(char::from)
        .collect()
}
