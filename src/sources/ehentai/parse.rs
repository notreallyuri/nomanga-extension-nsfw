use nomanga_sdk::parse::{document, selector};
use nomanga_sdk::prelude::*;

/// A listing row, in the compact table layout logged-out users get.
pub fn parse_listing(html: &str) -> SourceResult<(Vec<MangaSimple>, Option<String>)> {
    let doc = document(html);

    let row_sel = selector("table.itg td.gl3c.glname a, table.itg td.gl2c a")?;
    let title_sel = selector(".glink")?;

    let mut items = Vec::new();
    let mut seen = Vec::new();

    for link in doc.select(&row_sel) {
        let Some(href) = link.value().attr("href") else {
            continue;
        };
        let Some(id) = gallery_id(href) else {
            continue;
        };
        if seen.contains(&id) {
            continue;
        }

        let Some(title) = link
            .select(&title_sel)
            .next()
            .map(|t| t.text().collect::<String>().trim().to_owned())
        else {
            continue;
        };

        seen.push(id.clone());
        items.push(MangaSimple {
            id,
            title,
            cover_url: String::new(),
            description: None,
        });
    }

    // The thumbnail lives in a sibling cell, so it is collected separately and
    // matched back by row order.
    let thumbs = parse_thumbs(&doc)?;
    for (item, thumb) in items.iter_mut().zip(thumbs) {
        item.cover_url = thumb;
    }

    Ok((items, next_cursor(html)))
}

fn parse_thumbs(doc: &scraper::Html) -> SourceResult<Vec<String>> {
    let sel = selector("table.itg td.gl2c .glthumb img")?;

    Ok(doc
        .select(&sel)
        .map(|img| {
            img.value()
                .attr("data-src")
                .or_else(|| img.value().attr("src"))
                .unwrap_or_default()
                .to_owned()
        })
        .collect())
}

/// `/g/<gid>/<token>/` — both halves are needed to address a gallery, so the
/// id carries them together.
pub fn gallery_id(href: &str) -> Option<String> {
    let rest = href.split("/g/").nth(1)?;
    let mut parts = rest.split('/');
    let gid = parts.next()?;
    let token = parts.next()?;

    if gid.is_empty() || token.is_empty() || !gid.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }

    Some(format!("{gid}/{token}"))
}

pub fn split_id(id: &str) -> SourceResult<(u64, String)> {
    let (gid, token) = id.split_once('/').ok_or_else(|| SourceError::Parse {
        message: format!("malformed gallery id: {id}"),
    })?;

    let gid = gid.parse::<u64>().map_err(|_| SourceError::Parse {
        message: format!("malformed gallery id: {id}"),
    })?;

    Ok((gid, token.to_owned()))
}

/// E-Hentai pages by cursor, not offset: the "next" link carries the gid to
/// resume after. Its absence is what marks the last page.
fn next_cursor(html: &str) -> Option<String> {
    let marker = "next=";
    let mut best = None;

    for (idx, _) in html.match_indices(marker) {
        let rest = &html[idx + marker.len()..];
        let value: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        if !value.is_empty() {
            best = Some(value);
        }
    }

    best
}

/// The `/s/<imgkey>/<gid>-<page>` links on a gallery page, in page order.
pub fn parse_image_keys(html: &str, gid: u64) -> Vec<(u32, String)> {
    let needle = "/s/";
    let mut out: Vec<(u32, String)> = Vec::new();

    for (idx, _) in html.match_indices(needle) {
        let rest = &html[idx + needle.len()..];
        let Some(end) = rest.find(['"', '\'', ' ', '>']) else {
            continue;
        };
        let path = &rest[..end];

        let Some((imgkey, tail)) = path.split_once('/') else {
            continue;
        };
        let Some((row_gid, page)) = tail.split_once('-') else {
            continue;
        };

        if row_gid.parse::<u64>() != Ok(gid) {
            continue;
        }
        let Ok(page) = page.parse::<u32>() else {
            continue;
        };

        if !out.iter().any(|(p, _)| *p == page) {
            out.push((page, imgkey.to_owned()));
        }
    }

    out.sort_by_key(|(page, _)| *page);
    out
}

/// Total image count, read from the gallery page's "N pages" row.
pub fn parse_file_count(html: &str) -> Option<u32> {
    let idx = html.find(" pages</td>")?;
    let head = &html[..idx];
    let start = head.rfind('>')? + 1;

    head[start..].trim().parse().ok()
}

pub fn parse_showkey(html: &str) -> Option<String> {
    let rest = html.split("var showkey=\"").nth(1)?;
    let key: String = rest.chars().take_while(|c| *c != '"').collect();

    (!key.is_empty()).then_some(key)
}

/// Pulls the image URL out of an `<img id="img" src="...">`, which is how both
/// the `/s/` page and the `showpage` API return it.
pub fn parse_image_src(html: &str) -> Option<String> {
    let idx = html.find("id=\"img\"")?;
    let rest = &html[idx..];
    let src = rest.split("src=\"").nth(1)?;
    let url: String = src.chars().take_while(|c| *c != '"').collect();

    url.starts_with("http").then_some(url)
}

/// The multi-page viewer's `imagelist`, which yields every image key in one
/// request instead of paging the gallery 20 rows at a time.
pub fn parse_mpv_keys(html: &str) -> Vec<(u32, String)> {
    let Some(rest) = html.split("var imagelist = ").nth(1) else {
        return Vec::new();
    };
    let Some(end) = rest.find("];") else {
        return Vec::new();
    };

    let mut out = Vec::new();
    for (i, chunk) in rest[..end].split("\"k\":\"").enumerate().skip(1) {
        let key: String = chunk.chars().take_while(|c| *c != '"').collect();
        if !key.is_empty() {
            out.push((i as u32, key));
        }
    }

    out
}

pub fn tag_label(raw: &str) -> &str {
    raw.split_once(':').map(|(_, t)| t).unwrap_or(raw)
}

pub fn tag_namespace(raw: &str) -> &str {
    raw.split_once(':').map(|(n, _)| n).unwrap_or("")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_both_halves_of_a_gallery_id() {
        assert_eq!(
            gallery_id("https://e-hentai.org/g/4082076/e13e002e86/").as_deref(),
            Some("4082076/e13e002e86")
        );
        assert_eq!(gallery_id("https://e-hentai.org/uploader/bob"), None);
    }

    #[test]
    fn takes_the_last_next_cursor_on_the_page() {
        let html = r#"<a href="/?next=100">x</a> <a href="/?f_search=y&next=4074206">n</a>"#;
        assert_eq!(next_cursor(html).as_deref(), Some("4074206"));
        assert_eq!(next_cursor("<a>no pager</a>"), None);
    }

    #[test]
    fn keeps_image_keys_in_page_order_and_ignores_other_galleries() {
        let html = r#"
            <a href="https://e-hentai.org/s/0c160a1b54/4082076-2">2</a>
            <a href="https://e-hentai.org/s/1211073658/4082076-1">1</a>
            <a href="https://e-hentai.org/s/deadbeef00/9999999-1">other</a>
        "#;

        assert_eq!(
            parse_image_keys(html, 4082076),
            vec![
                (1, "1211073658".to_owned()),
                (2, "0c160a1b54".to_owned()),
            ]
        );
    }

    #[test]
    fn reads_the_image_url_out_of_an_img_tag() {
        let html = r#"<a><img id="img" src="https://x.hath.network:43456/h/a-b/keystamp=1-2/f.webp" style="w"/></a>"#;
        assert_eq!(
            parse_image_src(html).as_deref(),
            Some("https://x.hath.network:43456/h/a-b/keystamp=1-2/f.webp")
        );
    }

    #[test]
    fn splits_namespaced_tags() {
        assert_eq!(tag_namespace("artist:foo"), "artist");
        assert_eq!(tag_label("artist:foo"), "foo");
        assert_eq!(tag_label("bare"), "bare");
    }
}
