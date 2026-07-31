pub mod api;
pub mod util;

use crate::sources::nhentai::util::opts;
use nomanga_sdk::{
    data::{
        chapter::{Chapter, Page},
        homepage::Homepage,
        manga::{Manga, Status},
    },
    extension::{
        error::{SourceError, SourceResult},
        filter::{Filter, FilterValues},
        query::{ChapterRef, MangaPage, MangaRef, SearchQuery},
        rate_limit::{RateLimit, SourceMethod},
        source::{Source, SourceInfo},
    },
    guest::{self, Request},
    parse::encode_query,
    prelude::*,
};

const API_URL: &str = "https://nhentai.net/api/v2";

pub struct NHentaiSource;

impl Source for NHentaiSource {
    fn info(&self) -> SourceInfo {
        SourceInfo {
            id: "net.nhentai.v2".into(),
            name: "nHentai (V2)".into(),
            version: "2.0".to_owned(),
            language: "multi".into(),
            base_url: "https://nhentai.net".into(),
            icon_url: Some(include_str!("../../../icons/nhentai.txt").into()),
            hosts: vec!["nhentai.net".into(), "api.nhentai.net".into()],
            nsfw: true,
            challenge: None,
        }
    }

    fn section(&self, section: SectionRef) -> SourceResult<MangaPage> {
        let mut base_query = encode_query(&util::apply_global_query_settings(String::new()));

        if base_query.trim().is_empty() {
            base_query = "*".to_string();
        }

        let sort_param = match section.section_id.as_str() {
            "latest" => "date",
            "popular_today" => "popular-today",
            "popular_week" => "popular-week",
            "popular_month" => "popular-month",
            "popular_all" => "popular",
            _ => {
                return Err(SourceError::Parse {
                    message: "Unknown section ID".into(),
                });
            }
        };

        let url = format!(
            "{API_URL}/search?query={}&sort={}&page={}",
            base_query, sort_param, section.page
        );

        let res: api::V2SearchResponse = auth_request(&url).json()?;

        let items = util::map_galleries(res.result.unwrap_or_default());
        let has_next = (section.page as u32) < res.num_pages;

        Ok(MangaPage { has_next, items })
    }

    // nhentai meters per endpoint, per IP, per minute, and an API key roughly
    // doubles each ceiling. The host meters per source method instead, so every
    // endpoint budget is split across the methods that spend it and divided by
    // the calls each method makes:
    //
    //   GET /search          10/min anon, 20/min keyed
    //     homepage  x5 calls -> 1 (5 req) keyed 2 (10 req)
    //     search    x1 call  -> 3 (3 req) keyed 6 ( 6 req)
    //     section   x1 call  -> 2 (2 req) keyed 4 ( 4 req)
    //                          = 10 req             = 20 req
    //
    //   GET /galleries/{id}  20/min anon, 45/min keyed
    //     manga     x1 call  -> 10        keyed 22
    //     pages     x1 call  -> 10        keyed 22
    //
    // `chapters` is absent because it makes no request -- the gallery is a
    // single oneshot synthesised from the id it was handed.
    //
    // Read at snapshot time the key always looks absent, since that plugin is
    // built with no config at all, so an unconfigured host sees the anonymous
    // numbers. That is the floor it wants: a configured instance re-reads this
    // and only ever widens.
    fn rate_limits(&self) -> Vec<RateLimit> {
        let keyed = !guest::setting_or("api_key", "").is_empty();

        let (homepage, search, section, gallery) =
            if keyed { (2, 6, 4, 22) } else { (1, 3, 2, 10) };

        vec![
            RateLimit::per_minute(SourceMethod::Homepage, homepage),
            RateLimit::per_minute(SourceMethod::Search, search),
            RateLimit::per_minute(SourceMethod::Section, section),
            RateLimit::per_minute(SourceMethod::Manga, gallery),
            RateLimit::per_minute(SourceMethod::Pages, gallery),
        ]
    }

    fn settings(&self) -> Vec<Setting> {
        vec![
            Setting::select(
                "language",
                "Preferred Language",
                SelectOption::list([
                    ("all", "All"),
                    ("english", "English"),
                    ("japanese", "Japanese"),
                    ("chinese", "Chinese"),
                ]),
            )
            .with_description("Filter homepage and searches by this language."),
            Setting::secret("api_key", "API Key").with_description(
                "Optional. Roughly doubles nhentai's per-minute request ceilings, \
                 so browsing and searching throttle less. Create one under your \
                 nhentai account settings.",
            ),
            Setting::text("global_included", "Global Included Tags")
                .with_description("Added to every search (e.g. \"big breasts\" sole female)"),
            Setting::text("global_excluded", "Global Excluded Tags")
                .with_description("Removed from every search (e.g. -guro -\"ugly bastard\")"),
        ]
    }

    fn filters(&self) -> Vec<Filter> {
        let mut all_tags = Vec::new();

        for page in 1..=4 {
            let url = format!("{API_URL}/tags/tag?sort=popular&per_page=100&page={page}");
            if let Ok(res) = auth_request(&url).json::<api::V2TagListResponse>() {
                if let Some(tags) = res.result {
                    all_tags.extend(tags);
                }
            } else {
                break;
            }
        }

        let options = if all_tags.is_empty() {
            opts(&[])
        } else {
            all_tags.sort_by(|a, b| a.name.cmp(&b.name));
            all_tags.dedup_by(|a, b| a.name == b.name);
            all_tags
                .into_iter()
                .map(|t| SelectOption {
                    id: t.name.clone(),
                    label: t.name,
                })
                .collect()
        };

        vec![
            Filter::multi_select("tags", "Tags", options).with_exclusion(),
            Filter::text("artist", "Artist").with_placeholder("e.g. shindo l"),
            Filter::text("group", "Group").with_placeholder("e.g. fakku"),
            Filter::text("parody", "Parody").with_placeholder("e.g. touhou project"),
            Filter::text("character", "Character").with_placeholder("e.g. tifa lockhart"),
            Filter::text("pages", "Pages Count").with_placeholder("(e.g. >20, <50, 10-50)"),
            Filter::text("favorites", "Favorites").with_placeholder("(e.g. >500, 1000+, 69<)"),
            Filter::text("uploaded", "Uploaded Date").with_placeholder("(e.g. <7d, >1m, <1y)"),
        ]
    }

    fn homepage(&self) -> SourceResult<Homepage> {
        let mut base_query = encode_query(&util::apply_global_query_settings(String::new()));

        if base_query.trim().is_empty() {
            base_query = "*".to_string();
        }

        let latest: api::V2SearchResponse =
            auth_request(&format!("{API_URL}/search?query={base_query}&sort=date")).json()?;

        let popular_today: api::V2SearchResponse = auth_request(&format!(
            "{API_URL}/search?query={base_query}&sort=popular-today"
        ))
        .json()?;

        let popular_week: api::V2SearchResponse = auth_request(&format!(
            "{API_URL}/search?query={base_query}&sort=popular-week"
        ))
        .json()?;

        let popular_month: api::V2SearchResponse = auth_request(&format!(
            "{API_URL}/search?query={base_query}&sort=popular-month",
        ))
        .json()?;

        let popular_all: api::V2SearchResponse =
            auth_request(&format!("{API_URL}/search?query={base_query}&sort=popular",)).json()?;

        Ok(Homepage {
            sections: vec![
                HomepageSection {
                    id: "latest".into(),
                    title: "Latest Updates".into(),
                    layout: SectionLayout::TripleRow,
                    items: util::map_galleries(latest.result.unwrap_or_default()),
                    paginable: true,
                },
                HomepageSection {
                    id: "popular_today".into(),
                    title: "Popular Today".into(),
                    layout: SectionLayout::SingleRow,
                    items: util::map_galleries(popular_today.result.unwrap_or_default()),
                    paginable: true,
                },
                HomepageSection {
                    id: "popular_week".into(),
                    title: "Popular This Week".into(),
                    layout: SectionLayout::SingleRow,
                    items: util::map_galleries(popular_week.result.unwrap_or_default()),
                    paginable: true,
                },
                HomepageSection {
                    id: "popular_month".into(),
                    title: "Popular This Month".into(),
                    layout: SectionLayout::SingleRow,
                    items: util::map_galleries(popular_month.result.unwrap_or_default()),
                    paginable: true,
                },
                HomepageSection {
                    id: "popular_all".into(),
                    title: "All-Time Popular".into(),
                    layout: SectionLayout::SingleRow,
                    items: util::map_galleries(popular_all.result.unwrap_or_default()),
                    paginable: true,
                },
            ],
        })
    }

    fn search(&self, query: SearchQuery) -> SourceResult<MangaPage> {
        let mut final_query = query.term.clone();

        for tag in query.filters.included("tags") {
            final_query.push_str(&format!(" \"{tag}\""));
        }
        for tag in query.filters.excluded("tags") {
            final_query.push_str(&format!(" -\"{tag}\""));
        }

        for id in ["artist", "group", "parody", "character"] {
            if let Some(value) = query.filters.text(id) {
                if !value.is_empty() {
                    final_query.push_str(&format!(" {id}:\"{value}\""));
                }
            }
        }

        for id in ["pages", "favorites", "uploaded"] {
            if let Some(value) = query.filters.text(id) {
                if !value.is_empty() {
                    final_query.push_str(&format!(" {id}:{value}"));
                }
            }
        }

        final_query = util::apply_language_preference(final_query);
        let search_str = encode_query(&final_query);

        let url = format!("{API_URL}/search?query={}&page={}", search_str, query.page);

        let res: api::V2SearchResponse = auth_request(&url).json()?;
        let items = res.result.unwrap_or_default();

        Ok(MangaPage {
            has_next: query.page < res.num_pages,
            items: util::map_galleries(items),
        })
    }

    fn manga(&self, manga: MangaRef) -> SourceResult<Manga> {
        let url = format!("{API_URL}/galleries/{}/", manga.manga_id);

        let mut res: api::V2GalleryDetailResponse = auth_request(&url).json()?;

        let authors: Vec<String> = res
            .tags
            .iter()
            .filter(|t| t.tag_type == "artist" || t.tag_type == "group")
            .map(|t| t.name.clone())
            .collect();

        res.tags
            .sort_by_key(|t| (util::tag_priority(&t.tag_type), t.name.clone()));

        Ok(Manga {
            id: res.id.to_string(),
            title: res.title.best_title(),
            description: String::new(),
            tags: res
                .tags
                .into_iter()
                .filter(|t| t.tag_type != "artist" && t.tag_type != "group")
                .map(|t| Tag {
                    id: t.id.to_string(),
                    label: util::format_tag_label(&t.tag_type, &t.name),
                })
                .collect(),
            cover_url: util::build_image_url(&res.cover.path, true),
            author: authors.clone(),
            artist: authors,
            status: Status::Completed,
            last_updated: res.upload_date.to_string(),
            rating: None,
            views: Some(res.num_favorites as u64),
        })
    }

    fn chapters(&self, manga: MangaRef) -> SourceResult<Vec<Chapter>> {
        Ok(vec![Chapter {
            id: manga.manga_id.clone(),
            title: "Oneshot".to_string(),
            manga_id: manga.manga_id.clone(),
            number: 1.0,
            volume: None,
            language: "multi".to_string(),
            upload_date: String::new(),
            page_count: None,
            scanlator: None,
            url: format!("https://nhentai.net/g/{}/", manga.manga_id),
            is_locked: false,
        }])
    }

    fn pages(&self, chapter: ChapterRef) -> SourceResult<Vec<Page>> {
        let url = format!("{API_URL}/galleries/{}/", chapter.manga_id);

        let res: api::V2GalleryDetailResponse = auth_request(&url).json()?;

        let mut pages = Vec::new();

        for (i, page) in res.pages.iter().enumerate() {
            pages.push(Page {
                number: i as u32,
                image_url: util::build_image_url(&page.path, false),
            });
        }

        Ok(pages)
    }
}

fn auth_request(url: &str) -> Request {
    let mut req = Request::get(url);

    let api_key = guest::setting_or("api_key", "");
    if !api_key.is_empty() {
        req = req.header("Authorization", format!("Key {api_key}"));
    }

    req
}
