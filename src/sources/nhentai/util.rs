use super::api::EndpointClass;
use crate::sources::nhentai::api;
use nomanga_sdk::{
    guest,
    prelude::{MangaSimple, SelectOption},
};

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

/// Strips the bracketed metadata a raw nhentai title is wrapped in --
/// `[Artist] Title (Convention) [English] {Decensored}` becomes `Title`.
///
/// The detail endpoint ships this form as `title.pretty`, but the list
/// endpoints behind the homepage and search carry only the raw english and
/// japanese strings, so a card would otherwise read differently from the page
/// it opens. Reproduced here rather than fetched: asking the detail endpoint
/// per card would be a request each, against a 20/min budget.
///
/// A title that is nothing but metadata keeps its raw form -- a blank card is
/// worse than a noisy one.
pub fn pretty_title(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut depth = 0usize;

    for ch in raw.chars() {
        match ch {
            '[' | '(' | '{' => depth += 1,
            ']' | ')' | '}' => depth = depth.saturating_sub(1),
            _ if depth == 0 => out.push(ch),
            _ => {}
        }
    }

    let collapsed = out.split_whitespace().collect::<Vec<_>>().join(" ");

    if collapsed.is_empty() {
        raw.trim().to_owned()
    } else {
        collapsed
    }
}

pub fn map_galleries(items: Vec<api::V2GalleryListItem>) -> Vec<MangaSimple> {
    items
        .into_iter()
        .map(|g| MangaSimple {
            id: g.id.to_string(),
            title: pretty_title(&g.best_title()),
            cover_url: build_image_url(&g.thumbnail, true),
            description: None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::pretty_title;

    #[test]
    fn strips_the_metadata_around_a_title() {
        assert_eq!(
            pretty_title(
                "[Yamada Gogogo] Erolibrary (COMIC Anthurium 2019-01) [English] {Hennojin}"
            ),
            "Erolibrary"
        );
        assert_eq!(
            pretty_title("(C97) [Circle] Title Here [English]"),
            "Title Here"
        );
    }

    #[test]
    fn leaves_a_bare_title_alone() {
        assert_eq!(pretty_title("Just A Title"), "Just A Title");
    }

    #[test]
    fn handles_nesting_and_strays() {
        assert_eq!(pretty_title("[Artist [Circle]] Title"), "Title");
        assert_eq!(pretty_title("Title] Extra"), "Title Extra");
    }

    #[test]
    fn keeps_a_title_that_is_all_metadata() {
        assert_eq!(pretty_title("[Artist Only]"), "[Artist Only]");
    }
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
