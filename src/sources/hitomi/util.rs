use nomanga_sdk::{
    extension::error::{SourceError, SourceResult},
    guest::Request,
    parse::encode_query,
    prelude::SelectOption,
};
use regex::Regex;
use std::collections::HashSet;

pub struct HitomiResolver {
    b_val: String,
    o_val: bool,
    c_set: HashSet<u32>,
}

impl HitomiResolver {
    pub fn parse_gg(script: &str) -> SourceResult<Self> {
        let mut b_val = String::from("1763701202");
        let mut o_val = false;
        let mut c_set = HashSet::new();

        if let Some(caps) = Regex::new(r#"(?:var\s+)?b\s*[:=]\s*['"]([^'"]+)['"]"#)
            .unwrap()
            .captures(script)
        {
            b_val = caps
                .get(1)
                .unwrap()
                .as_str()
                .trim_end_matches('/')
                .to_string();
        }

        if let Some(caps) = Regex::new(r#"(?:var\s+)?o\s*=\s*(\d+)"#)
            .unwrap()
            .captures(script)
        {
            o_val = caps.get(1).unwrap().as_str() == "0";
        }

        if let Some(caps) = Regex::new(r#"(?:var\s+)?c\s*=\s*\[([\d,\s]+)\]"#)
            .unwrap()
            .captures(script)
        {
            for num_str in caps.get(1).unwrap().as_str().split(',') {
                if let Ok(n) = num_str.trim().parse::<u32>() {
                    c_set.insert(n);
                }
            }
        }

        if c_set.is_empty() {
            for caps in Regex::new(r#"case\s+(\d+):"#)
                .unwrap()
                .captures_iter(script)
            {
                if let Ok(n) = caps[1].parse::<u32>() {
                    c_set.insert(n);
                }
            }
            if c_set.is_empty() {
                for line in script.lines() {
                    let trimmed = line.trim();
                    if trimmed.starts_with("c") {
                        if let Some(caps) = Regex::new(r#"c\s*=\s*(\d+)"#).unwrap().captures(line) {
                            if let Ok(n) = caps[1].parse::<u32>() {
                                c_set.insert(n);
                            }
                        }
                    } else if trimmed.starts_with("case") {
                        if let Some(caps) = Regex::new(r#"case\s*(\d+):"#).unwrap().captures(line) {
                            if let Ok(n) = caps[1].parse::<u32>() {
                                c_set.insert(n);
                            }
                        }
                    }
                }
            }
        }

        if c_set.is_empty() {
            return Err(SourceError::Parse {
                message: "Failed to parse gg.js routing array".into(),
            });
        }

        Ok(Self {
            b_val,
            o_val,
            c_set,
        })
    }

    pub fn get_image_uri(
        &self,
        hash: &str,
        ext: &str,
        is_thumbnail: bool,
        is_small: bool,
    ) -> String {
        let len = hash.len();
        if len < 3 {
            return String::new();
        }

        let last_char = &hash[len - 1..];
        let prev_two = &hash[len - 3..len - 1];
        let route_hex = format!("{}{}", last_char, prev_two);
        let route = u32::from_str_radix(&route_hex, 16).unwrap_or(0);

        let is_matched = self.c_set.contains(&route) == self.o_val;

        if !is_thumbnail {
            let prefix = ext.chars().next().unwrap_or('w');
            let suffix = if is_matched { "2" } else { "1" };
            format!(
                "https://{}{}.gold-usergeneratedcontent.net/{}/{}/{}.{}",
                prefix, suffix, self.b_val, route, hash, ext
            )
        } else {
            let prefix = if is_matched { "b" } else { "a" };
            let size = if is_small { "small" } else { "big" };
            format!(
                "https://{}tn.gold-usergeneratedcontent.net/{}{}tn/{}/{}/{}.{}",
                prefix, ext, size, last_char, prev_two, hash, ext
            )
        }
    }
}

pub fn get_nozomi_url(
    tag_type: Option<&str>,
    tag_name: Option<&str>,
    language: &str,
    sort: Option<&str>,
) -> String {
    let domain = "https://ltn.gold-usergeneratedcontent.net";

    if let Some(s) = sort {
        let sort_path = if s == "day" { "today" } else { s };
        return format!("{}/popular/{}-{}.nozomi", domain, sort_path, language);
    }

    if let (Some(t), Some(n)) = (tag_type, tag_name) {
        let encoded_name = encode_query(&n.replace('_', " ").trim());
        match t {
            "male" | "female" => format!(
                "{}/n/tag/{}:{}-{}.nozomi",
                domain, t, encoded_name, language
            ),
            "language" => format!("{}/n/index-{}.nozomi", domain, encoded_name),
            _ => format!("{}/n/{}/{}-{}.nozomi", domain, t, encoded_name, language),
        }
    } else {
        format!("{}/n/index-{}.nozomi", domain, language)
    }
}

pub fn fetch_nozomi_ids(url: &str, start_index: usize, end_index: usize) -> SourceResult<Vec<u32>> {
    let byte_start = start_index * 4;
    let byte_end = (end_index * 4) - 1;

    let req = Request::get(url)
        .header("Referer", "https://hitomi.la/")
        .header("Range", format!("bytes={}-{}", byte_start, byte_end));

    let bytes = req.bytes()?;

    Ok(crate::sources::hitomi::util::parse_nozomi(&bytes))
}

pub fn parse_nozomi(bytes: &[u8]) -> Vec<u32> {
    let mut ids = Vec::with_capacity(bytes.len() / 4);

    for chunk in bytes.chunks_exact(4) {
        let id = u32::from_be_bytes(chunk.try_into().unwrap());
        ids.push(id);
    }

    ids
}

pub fn extract_gallery_json(raw_js: &str) -> SourceResult<&str> {
    let start = raw_js.find('{');
    let end = raw_js.rfind('}');

    match (start, end) {
        (Some(s), Some(e)) if e > s => Ok(&raw_js[s..=e]),
        _ => Err(SourceError::Parse {
            message: "Failed to extract JSON from galleryinfo.js".into(),
        }),
    }
}

pub fn fetch_full_nozomi(url: &str) -> SourceResult<Vec<u32>> {
    let req = Request::get(url).header("Referer", "https://hitomi.la/");
    let bytes = req.bytes()?;
    Ok(parse_nozomi(&bytes))
}

pub fn intersect_ids(mut lists: Vec<Vec<u32>>) -> Vec<u32> {
    if lists.is_empty() {
        return Vec::new();
    }

    lists.sort_by_key(|l| l.len());

    let mut current: std::collections::HashSet<u32> = lists.remove(0).into_iter().collect();

    for next_list in lists {
        let next_set: std::collections::HashSet<u32> = next_list.into_iter().collect();
        current.retain(|id| next_set.contains(id));
        if current.is_empty() {
            break;
        }
    }

    let mut final_ids: Vec<u32> = current.into_iter().collect();

    final_ids.sort_by(|a, b| b.cmp(a));
    final_ids
}

pub fn fetch_top_tags() -> Vec<SelectOption> {
    struct ParsedTag {
        id: String,
        display: String,
        count: u32,
    }

    let mut all_tags = Vec::new();

    let chars = ["a", "b", "c", "d", "e", "f", "g", "h", "m", "s", "t", "y"];

    let re = Regex::new(
        r#"href="(?:https?://hitomi\.la)?/tag/([^"]+)-all\.html"[^>]*>([^<]+)</a>\s*\(([\d,]+)\)"#,
    )
    .unwrap();

    for c in chars {
        let url = format!("https://hitomi.la/alltags-{}.html", c);

        if let Ok(html) = crate::sources::hitomi::hitomi_get(&url).text() {
            for caps in re.captures_iter(&html) {
                let raw_id = caps.get(1).map_or("", |m| m.as_str());
                let display = caps.get(2).map_or("", |m| m.as_str()).trim();
                let count_str = caps.get(3).map_or("", |m| m.as_str()).replace(",", "");

                let count = count_str.parse::<u32>().unwrap_or(0);

                let id = raw_id.replace("%3A", ":").replace("%20", " ");

                if count > 100 {
                    all_tags.push(ParsedTag {
                        id,
                        display: display.to_string(),
                        count,
                    });
                }
            }
        }
    }

    all_tags.sort_by(|a, b| b.count.cmp(&a.count));

    all_tags
        .into_iter()
        .take(300)
        .map(|t| {
            let display_label = if t.id.starts_with("female:") {
                format!("{} ♀", t.display)
            } else if t.id.starts_with("male:") {
                format!("{} ♂", t.display)
            } else {
                t.display
            };

            SelectOption::new(t.id, display_label)
        })
        .collect()
}
