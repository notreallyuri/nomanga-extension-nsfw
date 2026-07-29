pub mod api;
pub mod parse;

use nomanga_sdk::{
    data::{
        chapter::{Chapter, Page},
        homepage::{Homepage, HomepageSection, SectionLayout},
        manga::{Manga, Status},
    },
    extension::{
        error::{SourceError, SourceResult},
        filter::Filter,
        query::{ChapterRef, MangaPage, MangaRef, SearchQuery, SectionRef},
        rate_limit::{RateLimit, SourceMethod},
        source::{Source, SourceInfo},
    },
    guest::{self, Request},
    parse::encode_query,
    prelude::*,
};
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

const BASE_URL: &str = "https://e-hentai.org";
const API_URL: &str = "https://api.e-hentai.org/api.php";

/// Every category bit set; E-Hentai's `f_cats` is an *exclusion* mask, so
/// showing only one category means masking off the other nine.
const ALL_CATEGORIES: u32 = 1023;

const CATEGORY_BITS: [(&str, u32); 10] = [
    ("misc", 1),
    ("doujinshi", 2),
    ("manga", 4),
    ("artistcg", 8),
    ("gamecg", 16),
    ("imageset", 32),
    ("cosplay", 64),
    ("asianporn", 128),
    ("non-h", 256),
    ("western", 512),
];

/// Maps a listing URL to the cursor that opens each of its pages.
///
/// E-Hentai has no offset pager — `?page=N` is accepted and ignored, and the
/// only way forward is `?next=<gid of the last row>`. Paging forward therefore
/// has to remember where each page started; a jump to a page we have never
/// walked to falls back to stepping forward from the deepest one we know.
static CURSORS: LazyLock<Mutex<HashMap<String, HashMap<u32, String>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn remember_cursor(key: &str, page: u32, cursor: &str) {
    if let Ok(mut cache) = CURSORS.lock() {
        cache
            .entry(key.to_owned())
            .or_default()
            .insert(page, cursor.to_owned());
    }
}

fn known_cursor(key: &str, page: u32) -> Option<String> {
    CURSORS.lock().ok()?.get(key)?.get(&page).cloned()
}

/// Walks forward from the deepest cached page until `page` is reachable,
/// caching each cursor on the way so the next visit is a single request.
fn fetch_listing(base: &str, page: u32) -> SourceResult<(Vec<MangaSimple>, Option<String>)> {
    let page = page.max(1);

    if page == 1 {
        let (items, cursor) = parse::parse_listing(&Request::get(base).text()?)?;
        if let Some(cursor) = &cursor {
            remember_cursor(base, 2, cursor);
        }
        return Ok((items, cursor));
    }

    if let Some(cursor) = known_cursor(base, page) {
        let url = with_cursor(base, &cursor);
        let (items, next) = parse::parse_listing(&Request::get(&url).text()?)?;
        if let Some(next) = &next {
            remember_cursor(base, page + 1, next);
        }
        return Ok((items, next));
    }

    let mut at = (2..=page)
        .rev()
        .find_map(|candidate| known_cursor(base, candidate).map(|c| (candidate, c)));

    if at.is_none() {
        let (_, cursor) = parse::parse_listing(&Request::get(base).text()?)?;
        let Some(cursor) = cursor else {
            return Ok((Vec::new(), None));
        };
        remember_cursor(base, 2, &cursor);
        at = Some((2, cursor));
    }

    let (mut current, mut cursor) = at.ok_or_else(|| SourceError::Parse {
        message: "could not establish a listing cursor".into(),
    })?;

    loop {
        let url = with_cursor(base, &cursor);
        let (items, next) = parse::parse_listing(&Request::get(&url).text()?)?;

        if let Some(next) = &next {
            remember_cursor(base, current + 1, next);
        }

        if current >= page {
            return Ok((items, next));
        }

        let Some(next) = next else {
            return Ok((Vec::new(), None));
        };
        cursor = next;
        current += 1;
    }
}

fn with_cursor(base: &str, cursor: &str) -> String {
    let joiner = if base.contains('?') { '&' } else { '?' };
    format!("{base}{joiner}next={cursor}")
}

/// Seeds the member session, when the user has supplied one.
///
/// E-Hentai has no API key: a session is the pair of cookies the forum login
/// hands a browser, and the forum itself sits behind a Cloudflare challenge, so
/// the app cannot mint them — the user pastes them. They go into the host jar
/// rather than onto each request so the reader's own image fetches carry them
/// too, which is where the raised image quota actually matters.
fn apply_session() -> bool {
    let member_id = guest::setting_or("ipb_member_id", "");
    let pass_hash = guest::setting_or("ipb_pass_hash", "");

    if member_id.trim().is_empty() || pass_hash.trim().is_empty() {
        return false;
    }

    for (name, value) in [("ipb_member_id", &member_id), ("ipb_pass_hash", &pass_hash)] {
        guest::set_cookie(
            BASE_URL,
            &format!("{name}={}; Domain=.e-hentai.org; Path=/", value.trim()),
        );
    }

    true
}

pub struct EHentaiSource;

impl Source for EHentaiSource {
    fn info(&self) -> SourceInfo {
        SourceInfo {
            id: "org.ehentai".into(),
            name: "E-Hentai".into(),
            version: "1.0".to_owned(),
            language: "multi".into(),
            base_url: BASE_URL.into(),
            icon_url: Some(include_str!("../../../icons/ehentai.txt").into()),
            hosts: vec![
                "e-hentai.org".into(),
                "api.e-hentai.org".into(),
                // Thumbnails, and the sharded nodes the page images come from.
                "ehgt.org".into(),
                "*.hath.network".into(),
            ],
            nsfw: true,
        }
    }

    /// E-Hentai bans clients that hammer it, and `pages()` is unavoidably
    /// request-heavy, so it gets the tightest budget.
    fn rate_limits(&self) -> Vec<RateLimit> {
        vec![
            RateLimit::per_minute(SourceMethod::Homepage, 10),
            RateLimit::per_minute(SourceMethod::Search, 20),
            RateLimit::per_minute(SourceMethod::Section, 20),
            RateLimit::per_minute(SourceMethod::Manga, 30),
            RateLimit::per_minute(SourceMethod::Chapters, 60),
            RateLimit::per_minute(SourceMethod::Pages, 4),
        ]
    }

    fn settings(&self) -> Vec<Setting> {
        vec![
            Setting::secret("ipb_member_id", "Member ID").with_description(
                "Optional. From the ipb_member_id cookie after logging in at \
                 forums.e-hentai.org. Raises the image limit and unlocks the \
                 multi-page viewer.",
            ),
            Setting::secret("ipb_pass_hash", "Pass Hash")
                .with_description("Optional. From the ipb_pass_hash cookie."),
        ]
    }

    fn filters(&self) -> Vec<Filter> {
        vec![
            Filter::multi_select(
                "categories",
                "Categories",
                SelectOption::list([
                    ("doujinshi", "Doujinshi"),
                    ("manga", "Manga"),
                    ("artistcg", "Artist CG"),
                    ("gamecg", "Game CG"),
                    ("imageset", "Image Set"),
                    ("cosplay", "Cosplay"),
                    ("asianporn", "Asian Porn"),
                    ("non-h", "Non-H"),
                    ("western", "Western"),
                    ("misc", "Misc"),
                ]),
            ),
            Filter::text("uploader", "Uploader").with_placeholder("Uploader name"),
        ]
    }

    fn homepage(&self) -> SourceResult<Homepage> {
        apply_session();

        let mut sections = Vec::new();

        for (id, title, layout) in [
            ("latest", "Latest", SectionLayout::TripleRow),
            ("popular", "Popular Now", SectionLayout::SingleRow),
        ] {
            if let Ok((items, _)) = fetch_listing(&section_url(id)?, 1)
                && !items.is_empty()
            {
                sections.push(HomepageSection {
                    id: id.to_owned(),
                    title: title.to_owned(),
                    layout,
                    items,
                    // /popular is a live snapshot with no pager of its own.
                    paginable: id != "popular",
                });
            }
        }

        Ok(Homepage { sections })
    }

    fn section(&self, section: SectionRef) -> SourceResult<MangaPage> {
        apply_session();

        let base = section_url(&section.section_id)?;
        let (items, next) = fetch_listing(&base, section.page)?;

        Ok(MangaPage {
            has_next: next.is_some(),
            items,
        })
    }

    fn search(&self, query: SearchQuery) -> SourceResult<MangaPage> {
        apply_session();

        let mut base = format!("{BASE_URL}/?f_search={}", encode_query(&query.term));

        let included = query.filters.included("categories");
        if !included.is_empty() {
            let wanted: u32 = CATEGORY_BITS
                .iter()
                .filter(|(id, _)| included.iter().any(|c| c == id))
                .map(|(_, bit)| bit)
                .sum();

            base.push_str(&format!("&f_cats={}", ALL_CATEGORIES - wanted));
        }

        if let Some(uploader) = query.filters.text("uploader")
            && !uploader.is_empty()
        {
            base.push_str(&format!("&f_search={}", encode_query(&format!(
                "uploader:{uploader}"
            ))));
        }

        let (items, next) = fetch_listing(&base, query.page)?;

        Ok(MangaPage {
            has_next: next.is_some(),
            items,
        })
    }

    fn manga(&self, manga: MangaRef) -> SourceResult<Manga> {
        apply_session();

        let (gid, token) = parse::split_id(&manga.manga_id)?;

        let request = api::GDataRequest {
            method: "gdata",
            gidlist: vec![(gid, token)],
            namespace: 1,
        };

        let response: api::GDataResponse = Request::post(API_URL)
            .json_body(&request)?
            .json()?;

        let meta = response
            .gmetadata
            .into_iter()
            .next()
            .ok_or_else(|| SourceError::NotFound {
                message: format!("no gallery {}", manga.manga_id),
            })?;

        if let Some(error) = meta.error {
            return Err(SourceError::NotFound { message: error });
        }

        let author = tags_in(&meta.tags, "group");
        let artist = tags_in(&meta.tags, "artist");

        let tags = meta
            .tags
            .iter()
            .map(|raw| Tag {
                id: raw.clone(),
                label: parse::tag_label(raw).to_owned(),
            })
            .collect();

        Ok(Manga {
            id: manga.manga_id.clone(),
            title: meta.title,
            // Galleries carry no synopsis; the original-language title is the
            // one genuinely useful line of prose available.
            description: meta.title_jpn,
            tags,
            cover_url: meta.thumb,
            author,
            artist,
            status: Status::Completed,
            last_updated: meta.posted,
            rating: meta.rating.parse().ok(),
            views: None,
        })
    }

    fn chapters(&self, manga: MangaRef) -> SourceResult<Vec<Chapter>> {
        let (gid, token) = parse::split_id(&manga.manga_id)?;

        Ok(vec![Chapter {
            id: manga.manga_id.clone(),
            title: "Gallery".to_owned(),
            manga_id: manga.manga_id.clone(),
            number: 1.0,
            volume: None,
            language: "multi".to_owned(),
            upload_date: String::new(),
            page_count: None,
            scanlator: None,
            url: format!("{BASE_URL}/g/{gid}/{token}/"),
            is_locked: false,
        }])
    }

    /// Costly by construction: E-Hentai exposes no bulk image-URL endpoint, so
    /// every page needs its own `showpage` call. Image keys come either from
    /// the multi-page viewer (one request, members only) or by paging the
    /// gallery 20 rows at a time.
    fn pages(&self, chapter: ChapterRef) -> SourceResult<Vec<Page>> {
        let member = apply_session();
        let (gid, token) = parse::split_id(&chapter.manga_id)?;

        let keys = image_keys(gid, &token, member)?;
        if keys.is_empty() {
            return Err(SourceError::Parse {
                message: "gallery listed no images".into(),
            });
        }

        // The first page is fetched as HTML because it is the only thing that
        // carries the gallery's showkey, which every later call needs.
        let first_url = format!("{BASE_URL}/s/{}/{gid}-{}", keys[0].1, keys[0].0);
        let first_html = Request::get(&first_url).text()?;

        let showkey = parse::parse_showkey(&first_html).ok_or_else(|| SourceError::Parse {
            message: "no showkey on the image page".into(),
        })?;

        let mut pages = Vec::with_capacity(keys.len());
        if let Some(url) = parse::parse_image_src(&first_html) {
            pages.push(Page {
                number: 0,
                image_url: url,
            });
        }

        for (index, (page_no, imgkey)) in keys.iter().enumerate().skip(1) {
            let request = api::ShowPageRequest {
                method: "showpage",
                gid,
                page: *page_no,
                imgkey,
                showkey: &showkey,
            };

            let response: api::ShowPageResponse =
                Request::post(API_URL).json_body(&request)?.json()?;

            if let Some(error) = response.error {
                return Err(SourceError::Parse { message: error });
            }

            let Some(url) = parse::parse_image_src(&response.i3) else {
                continue;
            };

            pages.push(Page {
                number: index as u32,
                image_url: url,
            });
        }

        Ok(pages)
    }
}

fn image_keys(gid: u64, token: &str, member: bool) -> SourceResult<Vec<(u32, String)>> {
    if member {
        let url = format!("{BASE_URL}/mpv/{gid}/{token}/");
        if let Ok(html) = Request::get(&url).text() {
            let keys = parse::parse_mpv_keys(&html);
            if !keys.is_empty() {
                return Ok(keys);
            }
        }
        // Not a member after all, or MPV is unavailable — fall through.
    }

    let gallery_url = format!("{BASE_URL}/g/{gid}/{token}/");
    let first = Request::get(&gallery_url).text()?;

    let mut keys = parse::parse_image_keys(&first, gid);
    let total = parse::parse_file_count(&first).unwrap_or(keys.len() as u32);

    // Gallery pages list 20 thumbnails each.
    let per_page = keys.len().max(1) as u32;
    let mut page = 1;

    while (keys.len() as u32) < total {
        let url = format!("{gallery_url}?p={page}");
        let html = Request::get(&url).text()?;

        let more = parse::parse_image_keys(&html, gid);
        if more.is_empty() {
            break;
        }

        for entry in more {
            if !keys.iter().any(|(p, _)| *p == entry.0) {
                keys.push(entry);
            }
        }

        page += 1;
        if page > total.div_ceil(per_page) + 1 {
            break;
        }
    }

    keys.sort_by_key(|(page, _)| *page);
    Ok(keys)
}

fn section_url(id: &str) -> SourceResult<String> {
    match id {
        "latest" => Ok(format!("{BASE_URL}/")),
        "popular" => Ok(format!("{BASE_URL}/popular")),
        _ => Err(SourceError::Parse {
            message: format!("unknown section {id}"),
        }),
    }
}

fn tags_in(tags: &[String], namespace: &str) -> Vec<String> {
    tags.iter()
        .filter(|raw| parse::tag_namespace(raw) == namespace)
        .map(|raw| parse::tag_label(raw).to_owned())
        .collect()
}
