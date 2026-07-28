use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct HitomiGalleryJson {
    pub title: Option<String>,
    pub japanese_title: Option<String>,
    pub date: Option<String>,
    pub artists: Option<Vec<HitomiAuthor>>,
    pub groups: Option<Vec<HitomiAuthor>>,
    pub tags: Option<Vec<HitomiTagJson>>,
    pub files: Option<Vec<HitomiFileJson>>,
}

#[derive(Deserialize, Debug)]
pub struct HitomiAuthor {
    pub artist: Option<String>,
    pub group: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct HitomiTagJson {
    pub tag: Option<String>,
    pub male: Option<serde_json::Value>,
    pub female: Option<serde_json::Value>,
}

#[derive(Deserialize, Debug)]
pub struct HitomiFileJson {
    pub hash: Option<String>,
    pub hasavif: Option<u8>,
    pub haswebp: Option<u8>,
}
