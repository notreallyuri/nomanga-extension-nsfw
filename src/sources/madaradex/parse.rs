use nomanga_sdk::parse::{document, selector, text_opt};
use nomanga_sdk::prelude::*;

pub fn parse_tags(html: &str) -> SourceResult<Vec<Tag>> {
    let doc = document(html);

    let tag_sel = selector(r#"div.row.genres > ul > li"#)?;
    let label_sel = selector("a")?;

    let mut tags = Vec::new();

    for tag in doc.select(&tag_sel) {
        if let Some(a_tag) = tag.select(&label_sel).next() {
            let label = a_tag.text().collect::<String>().trim().to_string();

            if let Some(href) = a_tag.value().attr("href") {
                let tag_id = href
                    .trim_end_matches("/")
                    .split("/")
                    .last()
                    .unwrap_or("")
                    .to_string();

                if !tag_id.is_empty() && !label.is_empty() {
                    tags.push(Tag { id: tag_id, label });
                }
            }
        }
    }

    Ok(tags)
}

pub fn parse_search(html: &str) -> SourceResult<MangaPage> {
    let doc = document(html);

    let item_sel = selector(".c-tabs-item__content")?;

    let title_sel = selector(".post-title h3 a, .post-title h4 a")?;
    let img_sel = selector(".tab-thumb img")?;

    let mut items = Vec::new();

    for item in doc.select(&item_sel) {
        let Some(title_el) = item.select(&title_sel).next() else {
            continue;
        };

        let manga_url = title_el.value().attr("href").unwrap_or_default();
        let title = title_el.text().collect::<String>().trim().to_string();

        let id = manga_url
            .trim_end_matches('/')
            .split('/')
            .last()
            .unwrap_or_default()
            .to_string();

        let cover_url = item
            .select(&img_sel)
            .next()
            .and_then(|img| {
                img.value()
                    .attr("data-src")
                    .or_else(|| img.value().attr("data-lazy-src"))
                    .or_else(|| img.value().attr("src"))
            })
            .unwrap_or_default()
            .to_string();

        if !id.is_empty() && !title.is_empty() {
            items.push(MangaSimple {
                id,
                title,
                cover_url,
                description: None,
            });
        }
    }

    let next_sel = selector(".wp-pagenavi .nextpostslink")?;
    let has_next = doc.select(&next_sel).next().is_some();

    Ok(MangaPage { has_next, items })
}

pub fn parse_new_manga(html: &str) -> SourceResult<Vec<MangaSimple>> {
    let doc = document(html);
    let title_sel = selector(".post-title a")?;
    let img_sel = selector("img")?;

    let slider_sel = selector(".popular-slider .slick-slide:not(.slick-cloned) .slider__item")?;

    let mut items = Vec::new();

    for item in doc.select(&slider_sel) {
        let Some(title_el) = item.select(&title_sel).next() else {
            continue;
        };

        let manga_url = title_el.value().attr("href").unwrap_or_default();
        let title = title_el.text().collect::<String>().trim().to_string();

        let id = manga_url
            .trim_end_matches('/')
            .split('/')
            .last()
            .unwrap_or_default()
            .to_string();

        let cover_url = item
            .select(&img_sel)
            .next()
            .and_then(|img| {
                img.value()
                    .attr("data-src")
                    .or_else(|| img.value().attr("data-lazy-src"))
                    .or_else(|| img.value().attr("src"))
            })
            .unwrap_or_default()
            .to_string();

        if !id.is_empty() && !title.is_empty() {
            items.push(MangaSimple {
                id,
                title,
                cover_url,
                description: None,
            });
        }
    }

    Ok(items)
}

pub fn parse_latest_updates(html: &str) -> SourceResult<Vec<MangaSimple>> {
    let doc = document(html);
    let title_sel = selector(".post-title a")?;
    let img_sel = selector("img")?;

    let latest_sel = selector(".page-content-listing .page-item-detail")?;

    let mut items = Vec::new();

    for item in doc.select(&latest_sel) {
        let Some(title_el) = item.select(&title_sel).next() else {
            continue;
        };

        let manga_url = title_el.value().attr("href").unwrap_or_default();
        let title = title_el.text().collect::<String>().trim().to_string();

        let id = manga_url
            .trim_end_matches('/')
            .split('/')
            .last()
            .unwrap_or_default()
            .to_string();

        let cover_url = item
            .select(&img_sel)
            .next()
            .and_then(|img| {
                img.value()
                    .attr("data-src")
                    .or_else(|| img.value().attr("data-lazy-src"))
                    .or_else(|| img.value().attr("src"))
            })
            .unwrap_or_default()
            .to_string();

        if !id.is_empty() && !title.is_empty() {
            items.push(MangaSimple {
                id,
                title,
                cover_url,
                description: None,
            });
        }
    }

    Ok(items)
}

pub fn parse_manga_details(html: &str, manga_id: &str) -> SourceResult<Manga> {
    let doc = document(html);
    let root = doc.root_element();

    let title = text_opt(root, ".post-title h1").unwrap_or_default();
    let description = text_opt(root, ".summary__content").unwrap_or_default();

    let img_sel = selector(".summary_image img")?;
    let cover_url = doc
        .select(&img_sel)
        .next()
        .and_then(|img| {
            img.value()
                .attr("data-src")
                .or_else(|| img.value().attr("data-lazy-src"))
                .or_else(|| img.value().attr("src"))
        })
        .unwrap_or_default()
        .to_string();

    let author_sel = selector(".author-content a")?;
    let author = doc
        .select(&author_sel)
        .map(|e| e.text().collect::<String>().trim().to_string())
        .collect();

    let artist_sel = selector(".artist-content a")?;
    let artist = doc
        .select(&artist_sel)
        .map(|e| e.text().collect::<String>().trim().to_string())
        .collect();

    let mut tags = Vec::new();
    let tag_sel = selector(".genres-content a, .tags-content a")?;
    for tag_el in doc.select(&tag_sel) {
        let label = tag_el.text().collect::<String>().trim().to_string();
        let id = tag_el
            .value()
            .attr("href")
            .unwrap_or_default()
            .trim_end_matches('/')
            .split('/')
            .last()
            .unwrap_or_default()
            .to_string();

        if !id.is_empty() && !label.is_empty() {
            tags.push(Tag { id, label });
        }
    }

    let status_str = text_opt(root, ".post-status .summary-content")
        .unwrap_or_default()
        .to_lowercase();

    let status = if status_str.contains("ongoing") {
        Status::Ongoing
    } else if status_str.contains("completed") || status_str.contains("end") {
        Status::Completed
    } else if status_str.contains("canceled") {
        Status::Cancelled
    } else if status_str.contains("on-hold") {
        Status::Hiatus
    } else {
        Status::Unknown
    };

    Ok(Manga {
        id: manga_id.to_string(),
        title,
        description,
        tags,
        cover_url,
        author,
        artist,
        status,
        last_updated: String::new(),
        rating: None,
        views: None,
    })
}

pub fn parse_chapter_list(html: &str, manga_id: &str) -> SourceResult<Vec<Chapter>> {
    let doc = document(html);
    let chapter_sel = selector(".wp-manga-chapter")?;
    let a_sel = selector("a")?;
    let date_i_sel = selector(".chapter-release-date i")?;
    let date_a_sel = selector(".chapter-release-date a")?;

    let mut chapters = Vec::new();

    for el in doc.select(&chapter_sel) {
        let Some(a_el) = el.select(&a_sel).next() else {
            continue;
        };

        let url = a_el.value().attr("href").unwrap_or_default().to_string();
        let title = a_el.text().collect::<String>().trim().to_string();

        // Extract the Madara chapter ID/slug
        let id = url
            .trim_end_matches('/')
            .split('/')
            .last()
            .unwrap_or_default()
            .to_string();

        let upload_date = if let Some(a_date) = el.select(&date_a_sel).next() {
            a_date.value().attr("title").unwrap_or_default().to_string()
        } else if let Some(i_date) = el.select(&date_i_sel).next() {
            i_date.text().collect::<String>().trim().to_string()
        } else {
            String::new()
        };

        let number = title
            .to_lowercase()
            .replace("chapter", "")
            .trim()
            .split_whitespace()
            .next()
            .unwrap_or("0")
            .parse::<f32>()
            .unwrap_or(0.0);

        if !id.is_empty() {
            chapters.push(Chapter {
                id,
                title: title.clone(),
                manga_id: manga_id.to_string(),
                number,
                volume: None,
                language: "en".to_string(),
                upload_date,
                page_count: None,
                scanlator: None,
                url,
                is_locked: false,
            });
        }
    }

    Ok(chapters)
}

pub fn parse_chapter_pages(html: &str) -> SourceResult<Vec<Page>> {
    let doc = document(html);
    let img_sel = selector(".wp-manga-chapter-img, .reading-content img")?;

    let mut pages = Vec::new();

    for (i, img) in doc.select(&img_sel).enumerate() {
        let url = img
            .value()
            .attr("data-src")
            .or_else(|| img.value().attr("data-lazy-src"))
            .or_else(|| img.value().attr("src"))
            .unwrap_or_default()
            .trim()
            .to_string();

        if !url.is_empty() {
            pages.push(Page {
                number: i as u32,
                image_url: url,
            });
        }
    }

    Ok(pages)
}
