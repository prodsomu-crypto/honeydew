//! Provider-independent metadata and validation for Honeydew Sounds.
//!
//! Providers translate API responses into SoundAsset. The browser applies one consistent licence policy before
//! displaying results. Downloaded files belong in a user-owned cache, never the managed bundled-content directory.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    Drums,
    Loops,
    Vocals,
    Instruments,
    Fx,
    Textures,
}

/// Licences accepted for a personal-use sound catalogue. Non-commercial use is permitted by the app's
/// personal-use policy; no-derivatives licences remain excluded because they can conflict with normal DAW editing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SoundLicense {
    Cc0,
    CcBy,
    CcByNc,
}

impl SoundLicense {
    pub fn parse(value: &str) -> Option<Self> {
        let value = value.trim().trim_end_matches('/').to_ascii_lowercase();
        match value.as_str() {
            "cc0"
            | "cc0-1.0"
            | "https://creativecommons.org/publicdomain/zero/1.0"
            | "http://creativecommons.org/publicdomain/zero/1.0" => Some(Self::Cc0),
            "cc-by"
            | "cc-by-2.0"
            | "cc-by-3.0"
            | "cc-by-4.0"
            | "https://creativecommons.org/licenses/by/2.0"
            | "https://creativecommons.org/licenses/by/3.0"
            | "https://creativecommons.org/licenses/by/4.0"
            | "http://creativecommons.org/licenses/by/2.0"
            | "http://creativecommons.org/licenses/by/3.0"
            | "http://creativecommons.org/licenses/by/4.0" => Some(Self::CcBy),
            "cc-by-nc"
            | "cc-by-nc-2.0"
            | "cc-by-nc-3.0"
            | "cc-by-nc-4.0"
            | "https://creativecommons.org/licenses/by-nc/2.0"
            | "https://creativecommons.org/licenses/by-nc/3.0"
            | "https://creativecommons.org/licenses/by-nc/4.0"
            | "http://creativecommons.org/licenses/by-nc/2.0"
            | "http://creativecommons.org/licenses/by-nc/3.0"
            | "http://creativecommons.org/licenses/by-nc/4.0" => Some(Self::CcByNc),
            _ => None,
        }
    }

    pub fn requires_attribution(self) -> bool {
        matches!(self, Self::CcBy | Self::CcByNc)
    }

    /// Whether the asset must only be used for non-commercial purposes.
    pub fn is_non_commercial(self) -> bool {
        matches!(self, Self::CcByNc)
    }

    /// A short, human-readable label for result rows and download confirmations.
    pub fn label(self) -> &'static str {
        match self {
            Self::Cc0 => "CC0",
            Self::CcBy => "CC BY",
            Self::CcByNc => "CC BY-NC · personal use",
        }
    }
}

/// Provider-independent metadata for one catalogue result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SoundAsset {
    /// Stable provider-qualified id, e.g. freesound:123456.
    pub id: String,
    pub title: String,
    pub creator: String,
    pub category: Category,
    #[serde(default)]
    pub duration_seconds: Option<f32>,
    #[serde(default)]
    pub sample_rate: Option<u32>,
    /// Canonical provider page, not a transient download URL.
    pub page_url: String,
    /// Direct audio URL obtained from the provider API.
    pub download_url: String,
    pub license_id: String,
    #[serde(default)]
    pub license_url: Option<String>,
    #[serde(default)]
    pub attribution: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

impl SoundAsset {
    pub fn accepted_license(&self) -> Option<SoundLicense> {
        SoundLicense::parse(&self.license_id)
    }

    /// Validate untrusted provider metadata before presenting or downloading it.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.id.is_empty() || self.id.len() > 160 || !self.id.is_ascii() {
            return Err("invalid asset id");
        }
        if self.title.trim().is_empty() || self.title.len() > 512 {
            return Err("invalid title");
        }
        if self.creator.trim().is_empty() || self.creator.len() > 256 {
            return Err("invalid creator");
        }
        if !https_url(&self.page_url) || !https_url(&self.download_url) {
            return Err("asset URLs must use HTTPS");
        }
        if self
            .license_url
            .as_deref()
            .is_some_and(|url| !https_url(url))
        {
            return Err("invalid licence URL");
        }
        if self
            .duration_seconds
            .is_some_and(|n| !n.is_finite() || n < 0.0 || n > 86_400.0)
        {
            return Err("invalid duration");
        }
        if self
            .sample_rate
            .is_some_and(|n| !(1_000..=768_000).contains(&n))
        {
            return Err("invalid sample rate");
        }
        if self.tags.len() > 128 || self.tags.iter().any(|tag| tag.len() > 80) {
            return Err("invalid tags");
        }
        Ok(())
    }

    pub fn is_downloadable(&self) -> bool {
        self.validate().is_ok() && self.accepted_license().is_some()
    }

    /// Attribution shown to the user and persisted beside a downloaded file.
    pub fn attribution_text(&self) -> Option<String> {
        let license = self.accepted_license()?;
        if !license.requires_attribution() {
            return None;
        }
        Some(
            self.attribution
                .clone()
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(|| {
                    format!("{} — {} ({})", self.title, self.creator, self.license_id)
                }),
        )
    }
}

fn https_url(value: &str) -> bool {
    let Some(rest) = value.strip_prefix("https://") else {
        return false;
    };
    !rest.is_empty()
        && !rest.starts_with('/')
        && !rest.chars().any(|c| matches!(c, '\r' | '\n' | '\0' | ' '))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn asset(license_id: &str) -> SoundAsset {
        SoundAsset {
            id: "freesound:123".into(),
            title: "Closed hi-hat".into(),
            creator: "Example creator".into(),
            category: Category::Drums,
            duration_seconds: Some(0.42),
            sample_rate: Some(44_100),
            page_url: "https://freesound.org/s/123/".into(),
            download_url: "https://cdn.example.org/audio/123.ogg".into(),
            license_id: license_id.into(),
            license_url: None,
            attribution: None,
            tags: vec!["hat".into(), "drum".into()],
        }
    }

    #[test]
    fn accepts_only_reusable_licences() {
        assert_eq!(asset("CC0").accepted_license(), Some(SoundLicense::Cc0));
        assert_eq!(
            asset("CC-BY-4.0").accepted_license(),
            Some(SoundLicense::CcBy)
        );
        assert_eq!(
            asset("https://creativecommons.org/publicdomain/zero/1.0/").accepted_license(),
            Some(SoundLicense::Cc0)
        );
        assert_eq!(
            asset("https://creativecommons.org/licenses/by/4.0/").accepted_license(),
            Some(SoundLicense::CcBy)
        );
        assert_eq!(
            asset("https://creativecommons.org/licenses/by-nc/4.0/").accepted_license(),
            Some(SoundLicense::CcByNc)
        );
        assert_eq!(
            asset("CC-BY-NC").accepted_license(),
            Some(SoundLicense::CcByNc)
        );
        assert!(
            asset("CC-BY-NC")
                .attribution_text()
                .unwrap()
                .contains("Example creator")
        );
        assert!(SoundLicense::CcByNc.is_non_commercial());
        assert_eq!(SoundLicense::CcByNc.label(), "CC BY-NC · personal use");
        assert!(!SoundLicense::CcBy.is_non_commercial());
        assert!(!asset("CC-BY-ND").is_downloadable());
    }

    #[test]
    fn rejects_insecure_urls_and_invalid_metadata() {
        let mut item = asset("CC0");
        item.download_url = "http://example.org/file.wav".into();
        assert_eq!(item.validate(), Err("asset URLs must use HTTPS"));
        let mut item = asset("CC0");
        item.duration_seconds = Some(f32::NAN);
        assert_eq!(item.validate(), Err("invalid duration"));
    }

    #[test]
    fn cc_by_attribution_is_retained_but_cc0_needs_none() {
        assert!(
            asset("CC-BY-4.0")
                .attribution_text()
                .unwrap()
                .contains("Example creator")
        );
        assert!(asset("CC0").attribution_text().is_none());
    }
}
