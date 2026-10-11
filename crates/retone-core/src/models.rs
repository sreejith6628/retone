//! Curated model lists, embedded from resources/models.json.
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct ModelInfo { pub id: String, pub label: String }

fn all() -> HashMap<String, Vec<ModelInfo>> {
    serde_json::from_str(include_str!("../resources/models.json")).expect("models.json is valid")
}

pub fn curated(provider_key: &str) -> Vec<ModelInfo> { all().remove(provider_key).unwrap_or_default() }

pub fn default_model(provider_key: &str) -> Option<String> {
    curated(provider_key).into_iter().next().map(|m| m.id).filter(|id| !id.is_empty())
}
