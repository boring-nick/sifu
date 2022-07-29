use dashmap::DashMap;
use std::{path::PathBuf, sync::Arc};
use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncWriteExt},
    sync::Mutex,
};

pub const FILENAME: &str = "hashes.json";

#[derive(Clone)]
pub struct HashStorage {
    pub map: Arc<DashMap<String, String>>, // Hash, filename
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
        let mut file = File::open(file_path).await?;
        let metadata = file.metadata().await?;
        let mut buf = Vec::with_capacity(metadata.len() as usize);

        file.read_to_end(&mut buf).await?;

        let map = serde_json::from_slice(&buf)?;

        Ok(Self {
            map: Arc::new(map),
            file: Arc::new(Mutex::new(file)),
        })
    }

    pub async fn save(&self) -> anyhow::Result<()> {
        let data = serde_json::to_vec_pretty(&*self.map)?;
        self.file.lock().await.write_all(&data).await?;

        Ok(())
    }
}
