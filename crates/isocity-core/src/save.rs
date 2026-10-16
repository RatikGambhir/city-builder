//! Persistence: multiple saved cities on disk plus import of IsoCity web saves.
//!
//! Layout inside the save directory:
//! - `index.json` – list of [`SavedCityMeta`] (most recent first)
//! - `<id>.city.gz` – gzip-compressed JSON [`GameState`]

use crate::types::GameState;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedCityMeta {
    pub id: String,
    pub city_name: String,
    pub population: i64,
    pub money: i64,
    pub year: i32,
    pub month: i32,
    pub grid_size: i32,
    /// Milliseconds since the Unix epoch.
    pub saved_at: u64,
}

impl SavedCityMeta {
    pub fn from_state(state: &GameState) -> Self {
        Self {
            id: state.id.clone(),
            city_name: state.city_name.clone(),
            population: state.stats.population,
            money: state.stats.money,
            year: state.year,
            month: state.month,
            grid_size: state.grid_size,
            saved_at: now_ms(),
        }
    }
}

pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[derive(Debug)]
pub enum SaveError {
    Io(std::io::Error),
    Json(serde_json::Error),
    NotFound,
}

impl std::fmt::Display for SaveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SaveError::Io(e) => write!(f, "I/O error: {e}"),
            SaveError::Json(e) => write!(f, "invalid city data: {e}"),
            SaveError::NotFound => write!(f, "saved city not found"),
        }
    }
}

impl std::error::Error for SaveError {}

impl From<std::io::Error> for SaveError {
    fn from(e: std::io::Error) -> Self {
        SaveError::Io(e)
    }
}

impl From<serde_json::Error> for SaveError {
    fn from(e: serde_json::Error) -> Self {
        SaveError::Json(e)
    }
}

/// Serializes a city to gzip-compressed JSON.
pub fn encode(state: &GameState) -> Result<Vec<u8>, SaveError> {
    let json = serde_json::to_vec(state)?;
    let mut enc = GzEncoder::new(Vec::new(), Compression::fast());
    enc.write_all(&json)?;
    Ok(enc.finish()?)
}

/// Parses a city from gzip-compressed or plain JSON (including IsoCity web
/// exports) and normalizes it so it is ready to simulate.
pub fn decode(bytes: &[u8]) -> Result<GameState, SaveError> {
    let json = if bytes.starts_with(&[0x1f, 0x8b]) {
        let mut out = Vec::new();
        GzDecoder::new(bytes).read_to_end(&mut out)?;
        out
    } else {
        bytes.to_vec()
    };
    let state: GameState = serde_json::from_slice(&json)?;
    Ok(crate::normalize_loaded(state))
}

pub struct SaveManager {
    dir: PathBuf,
}

impl SaveManager {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    /// Platform data directory, e.g. `~/.local/share/isocity/cities`.
    pub fn default_dir() -> PathBuf {
        let base = if cfg!(target_os = "windows") {
            std::env::var_os("APPDATA").map(PathBuf::from)
        } else if cfg!(target_os = "macos") {
            std::env::var_os("HOME").map(|h| PathBuf::from(h).join("Library/Application Support"))
        } else {
            std::env::var_os("XDG_DATA_HOME")
                .map(PathBuf::from)
                .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))
        };
        base.unwrap_or_else(|| PathBuf::from(".")).join("isocity").join("cities")
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    fn index_path(&self) -> PathBuf {
        self.dir.join("index.json")
    }

    fn city_path(&self, id: &str) -> PathBuf {
        let safe: String = id
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
            .collect();
        self.dir.join(format!("{safe}.city.gz"))
    }

    pub fn list(&self) -> Vec<SavedCityMeta> {
        let Ok(bytes) = std::fs::read(self.index_path()) else {
            return Vec::new();
        };
        let mut list: Vec<SavedCityMeta> = serde_json::from_slice(&bytes).unwrap_or_default();
        list.retain(|m| self.city_path(&m.id).exists());
        list.sort_by(|a, b| b.saved_at.cmp(&a.saved_at));
        list
    }

    fn write_index(&self, list: &[SavedCityMeta]) -> Result<(), SaveError> {
        std::fs::create_dir_all(&self.dir)?;
        let tmp = self.dir.join("index.json.tmp");
        std::fs::write(&tmp, serde_json::to_vec_pretty(list)?)?;
        std::fs::rename(tmp, self.index_path())?;
        Ok(())
    }

    pub fn save(&self, state: &GameState) -> Result<SavedCityMeta, SaveError> {
        std::fs::create_dir_all(&self.dir)?;
        let bytes = encode(state)?;
        let path = self.city_path(&state.id);
        let tmp = path.with_extension("tmp");
        std::fs::write(&tmp, bytes)?;
        std::fs::rename(&tmp, &path)?;
        let meta = SavedCityMeta::from_state(state);
        let mut list = self.list();
        list.retain(|m| m.id != meta.id);
        list.insert(0, meta.clone());
        self.write_index(&list)?;
        Ok(meta)
    }

    pub fn load(&self, id: &str) -> Result<GameState, SaveError> {
        let path = self.city_path(id);
        if !path.exists() {
            return Err(SaveError::NotFound);
        }
        decode(&std::fs::read(path)?)
    }

    pub fn delete(&self, id: &str) -> Result<(), SaveError> {
        let _ = std::fs::remove_file(self.city_path(id));
        let mut list = self.list();
        list.retain(|m| m.id != id);
        self.write_index(&list)
    }

    /// Most recently saved city, used for "Continue".
    pub fn latest(&self) -> Option<SavedCityMeta> {
        self.list().into_iter().next()
    }
}
