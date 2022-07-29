use crate::{error::Error, hash_storage::HashStorage, response::UploadResponse};
use axum::{
    extract::{ContentLengthLimit, Host, Multipart, Path},
    http::{header::CONTENT_TYPE, HeaderMap},
    response::{Html, IntoResponse},
    Extension, Json,
};
use dashmap::mapref::entry::Entry;
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use std::path::PathBuf;
use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncWriteExt},
};
use tracing::{error, trace};

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
    trace!("Reading file from {path:?}");

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
    Extension(hash_storage): Extension<HashStorage>,
    mut multipart: ContentLengthLimit<Multipart, MAX_CONTENT_LENGTH>,
    host: Host,
) -> Result<Json<UploadResponse>, Error> {
    // This can't read `hashes.json` since extensions are stripped
    let (file_name, path) = loop {
        let file_name = generate_filename();
        let path = storage_folder.join(&file_name);

        if !path.exists() {
            break (file_name, path);
        }
        trace!("File {file_name} already exists, regenerating");
    };

    if let Some(field) = multipart.0.next_field().await? {
        let data = field.bytes().await?;

        let hash = blake3::hash(&data);

        match hash_storage.map.entry(hash.clone()) {
            Entry::Occupied(occupied) => {
                let existing_file = occupied.get();
                trace!("Uploaded duplicate, reusing existing file {existing_file}");

                let upload_response = UploadResponse {
                    full_url: format!("{host}/{existing_file}", host = host.0),
                    file_name: existing_file.to_owned(),
                };

                Ok(Json(upload_response))
            }
            Entry::Vacant(vacant) => {
                let mut f = File::create(path).await?;
                f.write_all(&data).await?;

                vacant.insert(file_name.clone());

                {
                    let file_name = file_name.clone();
                    let hash_storage = hash_storage.clone();
                    tokio::spawn(async move {
                        if let Err(err) = hash_storage.write_entry(&hash, &file_name).await {
                            error!("Could not save hash to storage: {err}");
                        }
                    });
                }

                let full_url = format!("{host}/{file_name}", host = host.0);
                let upload_response = UploadResponse {
                    full_url,
                    file_name,
                };

                Ok(Json(upload_response))
            }
        }
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
