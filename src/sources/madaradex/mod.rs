pub mod parse;

use nomanga_sdk::{
    data::{
        chapter::{Chapter, Page},
        homepage::{Homepage, HomepageSection, SectionLayout},
        manga::Manga,
    },
    extension::{
        error::{SourceError, SourceResult},
        filter::Filter,
        query::{ChapterRef, MangaPage, MangaRef, SearchQuery, SectionRef},
        source::{Source, SourceInfo},
    },
    guest::{self, Request},
    parse::encode_query,
    prelude::*,
};

const BASE_URL: &str = "https://madaradex.org";
const AJAX_URL: &str = "https://madaradex.org/wp-admin/admin-ajax.php";

/// Authorises this session against the image CDN.
///
/// `cdn.madaradex.org` serves a custom 403 unless the request carries both
/// halves of the site's shield: an `mdx_fp` fingerprint that only the page's
/// own JavaScript ever generates, and an `mdx_auth` token the server mints
/// against it. Neither survives on its own — a token presented with a
/// different fingerprint is rejected.
///
/// The token the chapter HTML sets is useless: that page is edge-cached, so
/// its `Set-Cookie` is whatever was minted when the cache filled. The shield
/// works around its own caching by re-minting over AJAX on every page view,
/// which is why this runs per `pages()` call rather than once.
fn authorize(manga_id: &str, chapter_id: &str) -> SourceResult<()> {
    use std::sync::OnceLock;

    static FINGERPRINT: OnceLock<String> = OnceLock::new();

    let fingerprint = FINGERPRINT.get_or_init(|| guest::random_hex(16));
    guest::set_cookie(
        BASE_URL,
        &format!("mdx_fp={fingerprint}; Domain=.madaradex.org; Path=/"),
    );

    Request::post(AJAX_URL)
        .header("Content-Type", "application/x-www-form-urlencoded; charset=UTF-8")
        .header("X-Requested-With", "XMLHttpRequest")
        .referer(&format!("{BASE_URL}/title/{manga_id}/{chapter_id}/"))
        .body("action=mdx_auth_refresh")
        .text()?;

    Ok(())
}

pub struct MadaraDexSource;

impl Source for MadaraDexSource {
    fn info(&self) -> SourceInfo {
        SourceInfo {
            id: "org.madaradex".into(),
            name: "MadaraDex".into(),
            version: "1.0".to_owned(),
            language: "en".into(),
            base_url: BASE_URL.into(),
            icon_url: None,
            hosts: vec!["madaradex.org".into(), "cdn.madaradex.org".into()],
            nsfw: true,
        }
    }

    fn settings(&self) -> Vec<Setting> {
        vec![]
    }

    fn filters(&self) -> Vec<Filter> {
        vec![
            Filter::text("query", "Title").with_placeholder("Search..."),
            Filter::text("author", "Author").with_placeholder("Author"),
            Filter::text("artist", "Artist").with_placeholder("Artist"),
            Filter::text("release", "Year of Release").with_placeholder("Year"),
            Filter::select(
                "op",
                "Genre Condition",
                vec![
                    SelectOption {
                        id: "".into(),
                        label: "OR (having one of selected genres)".into(),
                    },
                    SelectOption {
                        id: "1".into(),
                        label: "AND (having all selected genres)".into(),
                    },
                ],
            )
            .with_default(""),
            Filter::select(
                "adult",
                "Adult Content",
                vec![
                    SelectOption {
                        id: "".into(),
                        label: "All".into(),
                    },
                    SelectOption {
                        id: "0".into(),
                        label: "None adult content".into(),
                    },
                    SelectOption {
                        id: "1".into(),
                        label: "Only adult content".into(),
                    },
                ],
            )
            .with_default(""),
            Filter::multi_select(
                "status",
                "Status",
                vec![
                    SelectOption {
                        id: "on-going".into(),
                        label: "Ongoing".into(),
                    },
                    SelectOption {
                        id: "end".into(),
                        label: "Completed".into(),
                    },
                    SelectOption {
                        id: "canceled".into(),
                        label: "Canceled".into(),
                    },
                    SelectOption {
                        id: "on-hold".into(),
                        label: "On Hold".into(),
                    },
                    SelectOption {
                        id: "upcoming".into(),
                        label: "Upcoming".into(),
                    },
                ],
            ),
            Filter::multi_select(
                "genres",
                "Genres",
                vec![
                    SelectOption {
                        id: "action".into(),
                        label: "Action".into(),
                    },
                    SelectOption {
                        id: "adventure".into(),
                        label: "Adventure".into(),
                    },
                    SelectOption {
                        id: "comedy".into(),
                        label: "Comedy".into(),
                    },
                    SelectOption {
                        id: "drama".into(),
                        label: "Drama".into(),
                    },
                    SelectOption {
                        id: "ecchi".into(),
                        label: "Ecchi".into(),
                    },
                    SelectOption {
                        id: "fantasy".into(),
                        label: "Fantasy".into(),
                    },
                    SelectOption {
                        id: "harem".into(),
                        label: "Harem".into(),
                    },
                    SelectOption {
                        id: "historical".into(),
                        label: "Historical".into(),
                    },
                    SelectOption {
                        id: "horror".into(),
                        label: "Horror".into(),
                    },
                    SelectOption {
                        id: "isekai".into(),
                        label: "Isekai".into(),
                    },
                    SelectOption {
                        id: "martial-arts".into(),
                        label: "Martial Arts".into(),
                    },
                    SelectOption {
                        id: "mature".into(),
                        label: "Mature".into(),
                    },
                    SelectOption {
                        id: "military".into(),
                        label: "Military".into(),
                    },
                    SelectOption {
                        id: "mystery".into(),
                        label: "Mystery".into(),
                    },
                    SelectOption {
                        id: "office".into(),
                        label: "Office".into(),
                    },
                    SelectOption {
                        id: "psychological".into(),
                        label: "Psychological".into(),
                    },
                    SelectOption {
                        id: "romance".into(),
                        label: "Romance".into(),
                    },
                    SelectOption {
                        id: "school-life".into(),
                        label: "School Life".into(),
                    },
                    SelectOption {
                        id: "sci-fi".into(),
                        label: "Sci-Fi".into(),
                    },
                    SelectOption {
                        id: "slice-of-life".into(),
                        label: "Slice of Life".into(),
                    },
                    SelectOption {
                        id: "sports".into(),
                        label: "Sports".into(),
                    },
                    SelectOption {
                        id: "supernatural".into(),
                        label: "Supernatural".into(),
                    },
                    SelectOption {
                        id: "thriller".into(),
                        label: "Thriller".into(),
                    },
                    SelectOption {
                        id: "tragedy".into(),
                        label: "Tragedy".into(),
                    },
                    SelectOption {
                        id: "yuri".into(),
                        label: "Yuri".into(),
                    },
                ],
            ),
            Filter::sort(
                "m_orderby",
                "Order By",
                vec![
                    SelectOption {
                        id: "".into(),
                        label: "Relevance".into(),
                    },
                    SelectOption {
                        id: "latest".into(),
                        label: "Latest".into(),
                    },
                    SelectOption {
                        id: "alphabet".into(),
                        label: "A-Z".into(),
                    },
                    SelectOption {
                        id: "rating".into(),
                        label: "Rating".into(),
                    },
                    SelectOption {
                        id: "trending".into(),
                        label: "Trending".into(),
                    },
                    SelectOption {
                        id: "views".into(),
                        label: "Most Views".into(),
                    },
                    SelectOption {
                        id: "new-manga".into(),
                        label: "New".into(),
                    },
                ],
            )
            .with_default(""),
        ]
    }

    fn homepage(&self) -> SourceResult<Homepage> {
        let mut sections = Vec::new();

        let fetch_section = |id: &str,
                             title: &str,
                             orderby: &str,
                             layout: SectionLayout|
         -> Option<HomepageSection> {
            let url = format!("{BASE_URL}/?s=&post_type=wp-manga&m_orderby={}", orderby);

            if let Ok(html) = Request::get(&url).text() {
                if let Ok(page) = parse::parse_search(&html) {
                    if !page.items.is_empty() {
                        return Some(HomepageSection {
                            id: id.to_string(),
                            title: title.to_string(),
                            layout,
                            items: page.items,
                            paginable: true,
                        });
                    }
                }
            }
            None
        };

        if let Some(sec) =
            fetch_section("trending", "Trending", "trending", SectionLayout::SingleRow)
        {
            sections.push(sec);
        }

        if let Some(sec) = fetch_section(
            "latest",
            "Latest Updates",
            "latest",
            SectionLayout::TripleRow,
        ) {
            sections.push(sec);
        }

        if let Some(sec) = fetch_section("views", "Most Viewed", "views", SectionLayout::SingleRow)
        {
            sections.push(sec);
        }

        if let Some(sec) = fetch_section("rating", "Top Rated", "rating", SectionLayout::SingleRow)
        {
            sections.push(sec);
        }

        if let Some(sec) = fetch_section(
            "new-manga",
            "New Manga",
            "new-manga",
            SectionLayout::SingleRow,
        ) {
            sections.push(sec);
        }

        Ok(Homepage { sections })
    }

    fn section(&self, section: SectionRef) -> SourceResult<MangaPage> {
        let orderby = match section.section_id.as_str() {
            "trending" => "trending",
            "latest" => "latest",
            "views" => "views",
            "rating" => "rating",
            "new-manga" => "new-manga",
            _ => {
                return Err(SourceError::Parse {
                    message: "Unknown section ID".into(),
                });
            }
        };

        let url = format!(
            "{BASE_URL}/page/{}/?s=&post_type=wp-manga&m_orderby={}",
            section.page, orderby
        );

        let html = Request::get(&url).text()?;

        parse::parse_search(&html)
    }

    fn search(&self, query: SearchQuery) -> SourceResult<MangaPage> {
        let mut url = format!("{BASE_URL}/page/{}/?s=", query.page);

        let user_query = if !query.term.is_empty() {
            query.term.clone()
        } else if let Some(q) = query.filters.text("query") {
            q.to_string()
        } else {
            "*".to_string()
        };

        url.push_str(&encode_query(&user_query));
        url.push_str("&post_type=wp-manga");

        if let Some(author) = query.filters.text("author") {
            if !author.is_empty() {
                url.push_str(&format!("&author={}", &encode_query(author)));
            }
        }
        if let Some(artist) = query.filters.text("artist") {
            if !artist.is_empty() {
                url.push_str(&format!("&artist={}", &encode_query(artist)));
            }
        }
        if let Some(release) = query.filters.text("release") {
            if !release.is_empty() {
                url.push_str(&format!("&release={}", &encode_query(release)));
            }
        }

        if let Some(op) = query.filters.select("op") {
            if !op.is_empty() {
                url.push_str(&format!("&op={}", op));
            }
        }
        if let Some(adult) = query.filters.select("adult") {
            if !adult.is_empty() {
                url.push_str(&format!("&adult={}", adult));
            }
        }

        if let Some((orderby, _reversed)) = query.filters.sort("m_orderby") {
            if !orderby.is_empty() {
                url.push_str(&format!("&m_orderby={}", orderby));
            }
        }

        for status in query.filters.included("status") {
            url.push_str(&format!("&status[]={}", status));
        }

        for genre in query.filters.included("genres") {
            url.push_str(&format!("&genre[]={}", genre));
        }

        let html = Request::get(&url).text()?;
        parse::parse_search(&html)
    }

    fn manga(&self, manga: MangaRef) -> SourceResult<Manga> {
        let url = format!("{BASE_URL}/title/{}/", manga.manga_id);
        let html = Request::get(&url).text()?;

        parse::parse_manga_details(&html, &manga.manga_id)
    }

    fn chapters(&self, manga: MangaRef) -> SourceResult<Vec<Chapter>> {
        let url = format!("{BASE_URL}/title/{}/", manga.manga_id);
        let html = Request::get(&url).text()?;

        parse::parse_chapter_list(&html, &manga.manga_id)
    }

    fn pages(&self, chapter: ChapterRef) -> SourceResult<Vec<Page>> {
        let url = format!(
            "{BASE_URL}/title/{}/{}/?style=list",
            chapter.manga_id, chapter.chapter_id
        );

        let html = Request::get(&url).text()?;
        let pages = parse::parse_chapter_pages(&html)?;

        // The app fetches these URLs itself, long after this call returns, so
        // the cookies have to be in the host jar before we hand them over.
        authorize(&chapter.manga_id, &chapter.chapter_id)?;

        Ok(pages)
    }
}
