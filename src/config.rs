use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;
use expanduser::expanduser;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct UploadPart {
    pub idx: usize,
    pub etag: String,
    pub original_size: u64,
    pub processed_size: u64,
    pub original_sha256: String,
    pub processed_sha256: String,
}

#[derive(Serialize, Deserialize)]
pub struct Backup {
    pub name: String,
    pub prefix: String,
    pub chunk_size: usize,
    pub upload_id: String,
    pub parts: Vec<UploadPart>,
    pub done: bool,
    pub started: String,
    pub completed: String,
    pub sha256: String,
    pub compression_enabled: bool,
    pub encryption_enabled: bool,
}

impl Backup {
    pub fn load(path: &Path) -> Result<Self> {
        let data = fs::read_to_string(path)?;
        let backup = serde_yaml::from_str(&data)?;

        Ok(backup)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        save(&self, path)
    }
}

#[derive(Serialize, Deserialize)]
pub struct Profile {
    pub access_key: String,
    pub secret_key: String,
    pub region: String,
    pub bucket: String,
    pub encryption_key: String,
    pub prefix: String,
}

impl Default for Profile {
    fn default() -> Self {
        Profile {
            access_key: "".to_string(),
            secret_key: "".to_string(),
            region: "".to_string(),
            bucket: "".to_string(),
            encryption_key: "".to_string(),
            prefix: "".to_string(),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct Config {
    profiles: HashMap<String, Profile>,
}

impl Config {
    pub fn load() -> Result<Self> {
        let raw = fs::read_to_string(Self::profiles_file())?;
        let config: Config = serde_yaml::from_str(&raw)?;

        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        save(&self, Config::profiles_file().as_path())
    }

    pub fn profile(&self, name: &str) -> Option<&Profile> {
        self.profiles.get(name)
    }

    pub fn set_profile(&mut self, name: &str, prof: Profile) {
        self.profiles.insert(name.to_string(), prof);
    }

    pub fn backup(&self, name: &str) -> PathBuf {
        Self::sab_dir()
            .join("backups")
            .join(format!("{}.yml", name))
    }

    pub fn sab_dir() -> PathBuf {
        PathBuf::from(expanduser("~/.sab").unwrap())
    }

    pub fn profiles_file() -> PathBuf {
        Self::sab_dir().join("profiles.yml")
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            profiles: Default::default(),
        }
    }
}

fn save<T: ?Sized + Serialize>(obj: &T, path: &Path) -> Result<()> {
    let data = serde_yaml::to_string(obj)?;
    let _ = fs::write(path, data)?;

    Ok(())
}
