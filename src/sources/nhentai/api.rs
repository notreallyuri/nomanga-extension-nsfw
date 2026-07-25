use serde::{Deserialize, Serialize};

pub enum NHentaiSearchSortId {
    Date,
    PopularToday,
    PopularWeek,
    PopularMonth,
    Popular,
    Related,
    LastRead,
    TopReread,
}

pub enum EndpointClass {
    Search,
    GalleryDetail,
    Media,
    Galleries,
    Random,
    Related,
    Tagged,
    Popular,
    Config,
    Captcha,
    Default,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct V2GalleryListItem {
    pub id: u32,
    pub media_id: String,
    pub thumbnail: String,
    pub thumbnail_width: u32,
    pub thumbnail_height: u32,
    pub english_title: Option<String>,
    pub japanese_title: Option<String>,
    pub tag_ids: Vec<u32>,
    pub num_pages: Option<u32>,
    pub num_favorites: Option<u32>,
    pub blacklisted: Option<bool>,
}

impl V2GalleryListItem {
    pub fn best_title(&self) -> String {
        self.english_title
            .clone()
            .or_else(|| self.japanese_title.clone())
            .unwrap_or_else(|| "Untitled".to_owned())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct V2SearchResponse {
    pub result: Option<Vec<V2GalleryListItem>>,
    pub num_pages: u32,
    pub per_page: u32,
    pub total: Option<u32>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct V2GalleryAsset {
    pub path: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct V2GalleryPageAsset {
    pub path: String,
    pub width: u32,
    pub height: u32,
    pub number: u32,
    pub thumbnail: String,
    pub thumbnail_width: u32,
    pub thumbnail_height: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct V2GalleryDetailResponse {
    pub id: u32,
    pub media_id: String,
    pub title: GalleryTitle,
    pub cover: V2GalleryAsset,
    pub thumbnail: V2GalleryAsset,
    pub tags: Vec<GalleryTag>,
    pub num_pages: u32,
    pub num_favorites: u32,
    pub upload_date: u64,
    pub pages: Vec<V2GalleryPageAsset>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GalleryTitle {
    pub english: Option<String>,
    pub japanese: Option<String>,
    pub pretty: Option<String>,
}

impl GalleryTitle {
    pub fn best_title(&self) -> String {
        self.pretty
            .clone()
            .or_else(|| self.english.clone())
            .or_else(|| self.japanese.clone())
            .unwrap_or_else(|| "Untitled".to_owned())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct V2TagEntry {
    pub id: u32,
    #[serde(rename = "type")]
    pub tag_type: String,
    pub name: String,
    pub slug: String,
    pub url: String,
    pub count: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct V2TagListResponse {
    pub result: Option<Vec<V2TagEntry>>,
    pub num_pages: u32,
    pub per_page: u32,
    pub total: Option<u32>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GalleryTag {
    pub id: u32,
    #[serde(rename = "type")]
    pub tag_type: String,
    pub name: String,
    pub url: String,
    pub count: u32,
}
