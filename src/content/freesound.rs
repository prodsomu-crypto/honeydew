//! Freesound API response adapter for Honeydew Sounds.
//!
//! This module is deliberately transport-free: the UI/network layer supplies a parsed API response and receives
//! validated, provider-neutral assets. API credentials must be supplied by a private local configuration layer;
//! never place them in source, URLs shown in logs, or catalogue metadata.

use serde::Deserialize;

use super::sounds::{Category, SoundAsset};

/// Freesound's search endpoint. The transport adds the locally configured API token without logging it.
pub const SEARCH_ENDPOINT: &str = "https://freesound.org/apiv2/search/";

/// Fields needed to populate the browser without one API request per result.
pub const SEARCH_FIELDS: &str =
    "id,name,username,duration,samplerate,license,license_url,url,previews,tags";

/// Build the public part of a Freesound search URL. Authentication is intentionally left to the caller.
pub fn search_url(query: &str, page: u32, page_size: u8) -> String {
    let page = page.max(1);
    let page_size = page_size.clamp(1, 50);
    format!(
        "{SEARCH_ENDPOINT}?query={}&page={page}&page_size={page_size}&fields={}",
        encode_query(query),
        encode_query(SEARCH_FIELDS),
    )
}

fn encode_query(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            encoded.push(byte as char);
        } else {
            use std::fmt::Write;
            let _ = write!(encoded, "%{byte:02X}");
        }
    }
    encoded
}

/// Minimal envelope for Freesound's paginated search response.
#[derive(Debug, Deserialize)]
pub struct SearchResponse {
    #[serde(default)]
    pub count: u64,
    #[serde(default)]
    pub results: Vec<SoundResult>,
}

/// The subset of a Freesound result required by Honeydew.
#[derive(Debug, Deserialize)]
pub struct SoundResult {
    pub id: u64,
    pub name: String,
    pub username: String,
    #[serde(default)]
    pub duration: Option<f64>,
    #[serde(default)]
    pub samplerate: Option<u32>,
    pub license: String,
    #[serde(default)]
    pub license_url: Option<String>,
    #[serde(default)]
    pub previews: Previews,
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Preview URLs are supplied by Freesound's search response.
#[derive(Debug, Default, Deserialize)]
pub struct Previews {
    #[serde(rename = "preview-hq-ogg")]
    pub high_quality_ogg: Option<String>,
    #[serde(rename = "preview-hq-mp3")]
    pub high_quality_mp3: Option<String>,
    #[serde(rename = "preview-lq-mp3")]
    pub low_quality_mp3: Option<String>,
}

impl SoundResult {
    /// Convert one result, excluding unknown/ND licences and results without a usable preview.
    pub fn into_asset(self) -> Option<SoundAsset> {
        let preview = self.previews.high_quality_ogg
            .or(self.previews.high_quality_mp3)
            .or(self.previews.low_quality_mp3)?;
        let asset = SoundAsset {
            id: format!("freesound:{}", self.id),
            title: self.name,
            creator: self.username,
            category: category_from_tags(&self.tags),
            duration_seconds: self.duration.and_then(|n| {
                (n.is_finite() && n >= 0.0 && n <= 86_400.0).then_some(n as f32)
            }),
            sample_rate: self.samplerate,
            page_url: format!("https://freesound.org/s/{}/", self.id),
            download_url: preview,
            license_id: self.license,
            license_url: self.license_url,
            attribution: None,
            tags: self.tags,
        };
        asset.is_downloadable().then_some(asset)
    }
}

fn category_from_tags(tags: &[String]) -> Category {
    let has = |words: &[&str]| tags.iter().any(|tag| words.iter().any(|word| tag.eq_ignore_ascii_case(word)));
    if has(&["drum", "kick", "snare", "hat", "cymbal", "percussion"]) {
        Category::Drums
    } else if has(&["loop", "looped", "beat", "rhythm"]) {
        Category::Loops
    } else if has(&["vocal", "voice", "speech", "singing"]) {
        Category::Vocals
    } else if has(&["instrument", "piano", "guitar", "bass", "synth", "strings"]) {
        Category::Instruments
    } else if has(&["fx", "sfx", "foley", "impact", "whoosh", "riser"]) {
        Category::Fx
    } else {
        Category::Textures
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_url_encodes_query_and_bounds_pagination() {
        let url = search_url("lo-fi kick & snare", 0, 100);
        assert!(url.contains("query=lo-fi%20kick%20%26%20snare"));
        assert!(url.contains("page=1&page_size=50"));
        assert!(url.contains("fields=id%2Cname%2Cusername"));
    }

    #[test]
    fn maps_freesound_result_and_keeps_attribution_requirements() {
        let json = r#"{
            "id": 42, "name": "Warm kick", "username": "maker",
            "duration": 0.8, "samplerate": 48000,
            "license": "https://creativecommons.org/licenses/by/4.0/",
            "license_url": "https://creativecommons.org/licenses/by/4.0/",
            "previews": {"preview-hq-ogg": "https://cdn.freesound.org/preview/42.ogg"},
            "tags": ["kick", "drum"]
        }"#;
        let result: SoundResult = serde_json::from_str(json).unwrap();
        let asset = result.into_asset().unwrap();
        assert_eq!(asset.id, "freesound:42");
        assert_eq!(asset.category, Category::Drums);
        assert_eq!(asset.sample_rate, Some(48_000));
        assert!(asset.attribution_text().unwrap().contains("maker"));
    }

    #[test]
    fn filters_unknown_licences_and_missing_previews() {
        let base = r#"{"id":1,"name":"x","username":"y","license":"CC-BY-ND","previews":{"preview-hq-ogg":"https://cdn.example/x.ogg"}}"#;
        assert!(serde_json::from_str::<SoundResult>(base).unwrap().into_asset().is_none());
        let no_preview = r#"{"id":1,"name":"x","username":"y","license":"CC0","previews":{}}"#;
        assert!(serde_json::from_str::<SoundResult>(no_preview).unwrap().into_asset().is_none());
    }
}
