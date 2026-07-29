pub mod api;
pub mod util;

use nomanga_sdk::{
    data::{
        chapter::{Chapter, Page},
        homepage::{Homepage, HomepageSection, SectionLayout},
        manga::{Manga, Status, Tag},
    },
    extension::{
        error::{SourceError, SourceResult},
        filter::Filter, // Removed unused FilterValues
        query::{ChapterRef, MangaPage, MangaRef, SearchQuery},
        source::{Source, SourceInfo},
    },
    guest::Request,
    prelude::*,
};

const BASE_URL: &str = "https://hitomi.la";
const LTN_URL: &str = "https://ltn.gold-usergeneratedcontent.net";

fn hitomi_get(url: &str) -> Request {
    Request::get(url).header("Referer", "https://hitomi.la/")
}

pub struct HitomiSource;

impl Source for HitomiSource {
    fn info(&self) -> SourceInfo {
        SourceInfo {
            id: "la.hitomi".into(),
            name: "Hitomi.la".into(),
            version: "1.0".to_owned(),
            language: "multi".into(),
            base_url: BASE_URL.into(),
            icon_url: Some(include_str!("../../../icons/hitomi.txt").into()),
            hosts: vec![
                "hitomi.la".into(),
                "ltn.gold-usergeneratedcontent.net".into(),
                "*.hitomi.la".into(),
            ],
            nsfw: true,
        }
    }

    fn section(&self, section: SectionRef) -> SourceResult<MangaPage> {
        let language = guest::setting_or("language", "all");

        let items_per_page = 20;
        let page = section.page as usize;
        let start_idx = (page - 1) * items_per_page;
        let end_idx = start_idx + items_per_page;

        let nozomi_url = match section.section_id.as_str() {
            "latest" => util::get_nozomi_url(None, None, &language, None),
            "popular_today" => util::get_nozomi_url(None, None, &language, Some("day")),
            "popular_week" => util::get_nozomi_url(None, None, &language, Some("week")),
            _ => {
                return Err(SourceError::Parse {
                    message: "Unknown section ID".into(),
                });
            }
        };

        let ids = util::fetch_nozomi_ids(&nozomi_url, start_idx, end_idx).unwrap_or_default();
        let has_next = ids.len() == items_per_page;

        let gg_script = hitomi_get(&format!("{LTN_URL}/gg.js"))
            .text()
            .unwrap_or_default();
        let resolver = util::HitomiResolver::parse_gg(&gg_script).ok();

        let items = ids
            .into_iter()
            .filter_map(|id| {
                let manga = get_manga_by_id(&id.to_string(), resolver.as_ref()).ok()?;
                Some(MangaSimple {
                    id: manga.id,
                    title: manga.title,
                    cover_url: manga.cover_url,
                    description: None,
                })
            })
            .collect();

        Ok(MangaPage { has_next, items })
    }

    fn settings(&self) -> Vec<Setting> {
        vec![
            Setting::select(
                "language",
                "Preferred Language",
                vec![
                    SelectOption::new("all", "All"),
                    SelectOption::new("english", "English"),
                    SelectOption::new("japanese", "Japanese"),
                    SelectOption::new("chinese", "Chinese"),
                    SelectOption::new("korean", "Korean"),
                    SelectOption::new("spanish", "Spanish"),
                ],
            )
            .with_description("Filter the homepage and search results by language."),
        ]
    }

    fn filters(&self) -> Vec<Filter> {
        let mut filters = vec![Filter::text("query", "Search (e.g. female:elf male:orc)")];

        let dynamic_tags = util::fetch_top_tags();

        if !dynamic_tags.is_empty() {
            filters.push(Filter::multi_select("tags", "Popular Tags", dynamic_tags));
        }

        filters
    }

    fn homepage(&self) -> SourceResult<Homepage> {
        let language = guest::setting_or("language", "all");
        let items_per_row = 27;

        let latest_url = util::get_nozomi_url(None, None, &language, None);
        let popular_today_url = util::get_nozomi_url(None, None, &language, Some("day"));
        let popular_week_url = util::get_nozomi_url(None, None, &language, Some("week"));

        let latest_ids = util::fetch_nozomi_ids(&latest_url, 0, items_per_row).unwrap_or_default();
        let pop_today_ids =
            util::fetch_nozomi_ids(&popular_today_url, 0, items_per_row).unwrap_or_default();
        let pop_week_ids =
            util::fetch_nozomi_ids(&popular_week_url, 0, items_per_row).unwrap_or_default();

        let gg_script = hitomi_get(&format!("{LTN_URL}/gg.js"))
            .text()
            .unwrap_or_default();
        let resolver = util::HitomiResolver::parse_gg(&gg_script).ok();

        let map_ids_to_manga = |ids: Vec<u32>| -> Vec<MangaSimple> {
            ids.into_iter()
                .filter_map(|id| {
                    let manga = get_manga_by_id(&id.to_string(), resolver.as_ref()).ok()?;

                    Some(MangaSimple {
                        id: manga.id,
                        title: manga.title,
                        cover_url: manga.cover_url,
                        description: None,
                    })
                })
                .collect()
        };

        Ok(Homepage {
            sections: vec![
                HomepageSection {
                    id: "latest".into(),
                    title: "Latest Updates".into(),
                    layout: SectionLayout::TripleRow,
                    items: map_ids_to_manga(latest_ids),
                    paginable: true,
                },
                HomepageSection {
                    id: "popular_today".into(),
                    title: "Popular Today".into(),
                    layout: SectionLayout::SingleRow,
                    items: map_ids_to_manga(pop_today_ids),
                    paginable: true,
                },
                HomepageSection {
                    id: "popular_week".into(),
                    title: "Popular This Week".into(),
                    layout: SectionLayout::SingleRow,
                    items: map_ids_to_manga(pop_week_ids),
                    paginable: true,
                },
            ],
        })
    }

    fn search(&self, query: SearchQuery) -> SourceResult<MangaPage> {
        let page = query.page as usize;
        let items_per_page = 20;
        let start_idx = (page - 1) * items_per_page;
        let end_idx = start_idx + items_per_page;
        let language = guest::setting_or("language", "all");

        let user_query = if !query.term.is_empty() {
            query.term.clone()
        } else if let Some(q) = query.filters.text("query") {
            q.to_string()
        } else {
            String::new()
        };

        let nozomi_url = if user_query.is_empty() {
            util::get_nozomi_url(None, None, &language, None)
        } else {
            let first_term = user_query.split_whitespace().next().unwrap_or("");
            let mut parts = first_term.splitn(2, ':');
            if let (Some(t), Some(n)) = (parts.next(), parts.next()) {
                util::get_nozomi_url(Some(t), Some(n), &language, None)
            } else {
                util::get_nozomi_url(Some("tag"), Some(first_term), &language, None)
            }
        };

        let ids = util::fetch_nozomi_ids(&nozomi_url, start_idx, end_idx).unwrap_or_default();
        let has_next = ids.len() == items_per_page;

        let gg_script = hitomi_get(&format!("{LTN_URL}/gg.js"))
            .text()
            .unwrap_or_default();
        let resolver = util::HitomiResolver::parse_gg(&gg_script).ok();

        let items = ids
            .into_iter()
            .filter_map(|id| {
                let manga = get_manga_by_id(&id.to_string(), resolver.as_ref()).ok()?;
                Some(MangaSimple {
                    id: manga.id,
                    title: manga.title,
                    cover_url: manga.cover_url,
                    description: None,
                })
            })
            .collect();

        Ok(MangaPage { has_next, items })
    }

    fn manga(&self, manga: MangaRef) -> SourceResult<Manga> {
        get_manga_by_id(&manga.manga_id, None)
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
            url: format!("{BASE_URL}/galleries/{}.html", manga.manga_id),
            is_locked: false,
        }])
    }

    fn pages(&self, chapter: ChapterRef) -> SourceResult<Vec<Page>> {
        let id = chapter.manga_id;

        let gallery_url = format!("{LTN_URL}/galleries/{}.js", id);
        let raw_js = hitomi_get(&gallery_url).text()?;
        let json_str = util::extract_gallery_json(&raw_js)?;
        let data: api::HitomiGalleryJson =
            serde_json::from_str(json_str).map_err(|e| SourceError::Parse {
                message: e.to_string(),
            })?;

        let gg_script = hitomi_get(&format!("{LTN_URL}/gg.js")).text()?;
        let resolver = util::HitomiResolver::parse_gg(&gg_script)?;

        let mut pages = Vec::new();

        if let Some(files) = data.files {
            for (i, file) in files.iter().enumerate() {
                if let Some(hash) = &file.hash {
                    let ext = if file.haswebp.unwrap_or(0) != 0 {
                        "webp"
                    } else {
                        "avif"
                    };

                    let image_url = resolver.get_image_uri(hash, ext, false, false);

                    pages.push(Page {
                        number: i as u32,
                        image_url,
                    });
                }
            }
        }

        Ok(pages)
    }
}

fn get_manga_by_id(id: &str, resolver: Option<&util::HitomiResolver>) -> SourceResult<Manga> {
    let gallery_url = format!("{LTN_URL}/galleries/{}.js", id);
    let raw_js = hitomi_get(&gallery_url).text()?;

    let json_str = util::extract_gallery_json(&raw_js)?;
    let data: api::HitomiGalleryJson =
        serde_json::from_str(json_str).map_err(|e| SourceError::Parse {
            message: e.to_string(),
        })?;

    let title = data.title.unwrap_or_else(|| format!("Gallery {}", id));

    let mut authors = Vec::new();
    if let Some(arts) = data.artists {
        for a in arts {
            if let Some(n) = a.artist {
                authors.push(n);
            }
        }
    }
    if let Some(grps) = data.groups {
        for g in grps {
            if let Some(n) = g.group {
                authors.push(n);
            }
        }
    }

    let mut tags = Vec::new();
    if let Some(t_list) = data.tags {
        for t in t_list {
            if let Some(name) = t.tag {
                let prefix = if t.male.is_some() {
                    "male:"
                } else if t.female.is_some() {
                    "female:"
                } else {
                    ""
                };
                tags.push(Tag {
                    id: format!("{}{}", prefix, name),
                    label: name.clone(),
                });
            }
        }
    }

    let mut cover_url = format!("{BASE_URL}/favicon.ico");

    let local_resolver_cache;
    let active_resolver = match resolver {
        Some(r) => Some(r),
        None => {
            if let Ok(gg_script) = hitomi_get(&format!("{LTN_URL}/gg.js")).text() {
                local_resolver_cache = util::HitomiResolver::parse_gg(&gg_script).ok();
                local_resolver_cache.as_ref()
            } else {
                None
            }
        }
    };

    if let Some(res) = active_resolver {
        if let Some(files) = &data.files {
            if let Some(first_file) = files.first() {
                if let Some(hash) = &first_file.hash {
                    let ext = if first_file.haswebp.unwrap_or(0) != 0 {
                        "webp"
                    } else {
                        "avif"
                    };
                    cover_url = res.get_image_uri(hash, ext, true, false);
                }
            }
        }
    }

    Ok(Manga {
        id: id.to_string(),
        title,
        description: String::new(),
        tags,
        cover_url,
        author: authors.clone(),
        artist: authors,
        status: Status::Completed,
        last_updated: data.date.unwrap_or_default(),
        rating: None,
        views: None,
    })
}
