use nomanga_sdk::{
    guest,
    prelude::{MangaSimple, SelectOption},
};

use crate::sources::nhentai::api;

use super::api::EndpointClass;

pub fn get_endpoint_rate_limit(endpoint: EndpointClass, authenticated: bool) -> u32 {
    match authenticated {
        true => match endpoint {
            EndpointClass::Search => 20,
            EndpointClass::GalleryDetail => 45,
            EndpointClass::Media => 240,
            EndpointClass::Galleries => 30,
            EndpointClass::Random => 30,
            EndpointClass::Related => 30,
            EndpointClass::Tagged => 30,
            EndpointClass::Popular => 30,
            EndpointClass::Config => 30,
            EndpointClass::Captcha => 12,
            EndpointClass::Default => 30,
        },
        false => match endpoint {
            EndpointClass::Search => 10,
            EndpointClass::GalleryDetail => 20,
            EndpointClass::Media => 180,
            EndpointClass::Galleries => 15,
            EndpointClass::Random => 20,
            EndpointClass::Related => 12,
            EndpointClass::Tagged => 15,
            EndpointClass::Popular => 15,
            EndpointClass::Config => 15,
            EndpointClass::Captcha => 6,
            EndpointClass::Default => 15,
        },
    }
}

pub fn opts(labels: &[&str]) -> Vec<SelectOption> {
    labels
        .iter()
        .map(|&s| SelectOption {
            id: s.into(),
            label: s.into(),
        })
        .collect()
}

pub fn apply_language_preference(mut query: String) -> String {
    let lang = guest::setting_or("language", "english");

    if lang != "all" && !lang.is_empty() {
        query.push_str(&format!(" \"{lang}\""));
    }

    let trimmed = query.trim();
    if trimmed.is_empty() {
        "\"\"".to_string()
    } else {
        trimmed.to_string()
    }
}

pub fn build_image_url(path: &str, is_thumb: bool) -> String {
    if path.starts_with("http") {
        return path.to_owned();
    }

    let cdn = if is_thumb {
        "https://t.nhentai.net"
    } else {
        "https://i.nhentai.net"
    };

    let clean_path = path.trim_start_matches('/');

    format!("{cdn}/{clean_path}")
}

pub fn map_galleries(items: Vec<api::V2GalleryListItem>) -> Vec<MangaSimple> {
    items
        .into_iter()
        .map(|g| MangaSimple {
            id: g.id.to_string(),
            title: g.best_title(),
            cover_url: build_image_url(&g.thumbnail, true),
            description: None,
        })
        .collect()
}

pub fn format_tag_type(tag_type: &str) -> String {
    match tag_type {
        "category" => "Type".to_string(),
        "character" => "Character".to_string(),
        other => {
            let mut c = other.chars();
            match c.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
            }
        }
    }
}

pub fn tag_priority(tag_type: &str) -> u8 {
    match tag_type {
        "category" => 1,
        "language" => 2,
        "parody" => 3,
        "character" => 4,
        "tag" => 5,
        _ => 6,
    }
}

pub fn format_tag_label(tag_type: &str, name: &str) -> String {
    if tag_type == "tag" {
        return name.to_string();
    }

    let prefix = match tag_type {
        "category" => "Type".to_string(),
        "character" => "Character".to_string(),
        other => {
            let mut c = other.chars();
            match c.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
            }
        }
    };

    format!("{prefix}: {name}")
}

pub fn apply_global_query_settings(mut query: String) -> String {
    let lang = guest::setting_or("language", "all");
    if lang != "all" && !lang.is_empty() {
        query.push_str(&format!(" language:{}", lang));
    }

    let included = guest::setting_or("global_included", "");
    if !included.is_empty() {
        query.push_str(&format!(" {included}"));
    }

    let excluded = guest::setting_or("global_excluded", "");
    if !excluded.is_empty() {
        query.push_str(&format!(" {excluded}"));
    }

    query.trim().to_string()
}
