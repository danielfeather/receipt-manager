use std::{
    collections::HashMap,
    ops::Deref,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Clone, Default)]
pub struct Manifest(HashMap<PathBuf, Chunk>);

impl Deref for Manifest {
    type Target = HashMap<PathBuf, Chunk>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct Chunk {
    pub name: String,
    pub file: String,
    pub src: String,
    pub is_entry: bool,
    pub css: Vec<String>,
}

pub fn load_manifest() -> Option<Manifest> {
    let Ok(manifest_bytes) = std::fs::read_to_string("public/.vite/manifest.json") else {
        return None;
    };

    let Ok(manifest) = serde_json::from_str(&manifest_bytes) else {
        return None;
    };

    Some(manifest)
}

pub fn resolve_css(entry_point: &Path, manifest: Option<&Manifest>) -> Vec<String> {
    let Some(manifest) = manifest else {
        return vec![];
    };

    let Some(chunk) = manifest.get(entry_point) else {
        return vec![];
    };

    return chunk.css.clone();
}

pub fn resolve_scripts(entry_point: &Path, manifest: Option<&Manifest>) -> Vec<String> {
    let Some(manifest) = manifest else {
        return vec![
            String::from("http://localhost:5173/@vite/client"),
            format!("http://localhost:5173/{}", entry_point.to_str().unwrap()),
        ];
    };

    let Some(chunk) = manifest.get(entry_point) else {
        return vec![];
    };

    return vec![format!("/{}", chunk.file)];
}
