use anyhow::Context;
use blake3::Hash;
use dashmap::DashMap;
use std::{path::PathBuf, sync::Arc};
use tokio::{
    fs::File,
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    sync::Mutex,
};
use tracing::{debug, info};

pub const FILENAME: &str = "hashes.kv";

#[derive(Clone)]
pub struct HashStorage {
    pub map: Arc<DashMap<Hash, String>>, // Hash, filename
    file: Arc<Mutex<File>>,
}

impl HashStorage {
    pub async fn new(file_path: PathBuf) -> anyhow::Result<Self> {
        if file_path.exists() {
            Self::load_from_file(file_path).await
        } else {
            let file = File::create(&file_path).await?;
            Ok(Self {
                map: Arc::new(DashMap::new()),
                file: Arc::new(Mutex::new(file)),
            })
        }
    }

    pub async fn load_from_file(file_path: PathBuf) -> anyhow::Result<Self> {
        let file = File::open(file_path).await?;
        let mut reader = BufReader::new(file.try_clone().await?);

        let map = DashMap::new();
        let mut buf = Vec::with_capacity(64);

        while reader.read_until(b':', &mut buf).await? != 0 {
            buf.remove(buf.len() - 1);
            let hash = Hash::from_hex(&buf).context("could not parse hash")?;
            buf.clear();

            while reader.read_until(b'\n', &mut buf).await? != 0 {
                buf.remove(buf.len() - 1);
                let file_name = String::from_utf8(buf)?;

                map.insert(hash, file_name);
                buf = Vec::with_capacity(64);
            }
        }

        info!("Loaded {} hashes", map.len());

        Ok(Self {
            map: Arc::new(map),
            file: Arc::new(Mutex::new(file)),
        })
    }

    pub async fn write_entry(&self, hash: &Hash, file_name: &str) -> anyhow::Result<()> {
        let mut file = self.file.lock().await;

        file.write_all(hash.to_hex().as_bytes()).await?;
        file.write_all(b":").await?;
        file.write_all(file_name.as_bytes()).await?;
        file.write_all(b"\n").await?;

        debug!("Wrote hash entry for {file_name}");

        Ok(())
    }
}
