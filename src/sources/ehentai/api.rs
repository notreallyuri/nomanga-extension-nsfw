use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub struct GDataRequest<'a> {
    pub method: &'a str,
    pub gidlist: Vec<(u64, String)>,
    pub namespace: u8,
}

#[derive(Deserialize)]
pub struct GDataResponse {
    #[serde(default)]
    pub gmetadata: Vec<GMetadata>,
}

/// Numeric fields arrive as strings, except `gid` and `filesize`.
#[derive(Deserialize)]
pub struct GMetadata {
    pub gid: u64,
    pub token: String,
    pub title: String,
    #[serde(default)]
    pub title_jpn: String,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub thumb: String,
    #[serde(default)]
    pub uploader: String,
    #[serde(default)]
    pub posted: String,
    #[serde(default)]
    pub filecount: String,
    #[serde(default)]
    pub rating: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub error: Option<String>,
}

#[derive(Serialize)]
pub struct ShowPageRequest<'a> {
    pub method: &'a str,
    pub gid: u64,
    pub page: u32,
    pub imgkey: &'a str,
    pub showkey: &'a str,
}

#[derive(Deserialize)]
pub struct ShowPageResponse {
    /// The `<img id="img">` element; the image URL has to be read back out of it.
    #[serde(default)]
    pub i3: String,
    #[serde(default)]
    pub error: Option<String>,
}
