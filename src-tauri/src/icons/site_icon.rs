use std::cmp::Reverse;
use std::collections::HashMap;
use std::sync::LazyLock;

use regex::Regex;
use url::Url;

use super::http::web_url;

const GITHUB_AVATAR_PIXELS: u32 = 64;

static LINK_TAG: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?is)<link\b[^>]*>").expect("valid link tag pattern"));
static ATTRIBUTE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?is)([a-z][a-z0-9-]*)\s*=\s*(?:"([^"]*)"|'([^']*)'|([^\s"'>]+))"#)
        .expect("valid attribute pattern")
});

struct IconLink {
    url: Url,
    size: u32,
}

/// Owner avatars represent GitHub-hosted tools better than GitHub's own favicon.
pub(super) fn github_avatar(homepage: &Url) -> Option<Url> {
    if !matches!(homepage.host_str()?, "github.com" | "www.github.com") {
        return None;
    }
    let owner = homepage
        .path_segments()?
        .next()
        .filter(|owner| !owner.is_empty())?;
    Url::parse(&format!(
        "https://github.com/{owner}.png?size={GITHUB_AVATAR_PIXELS}"
    ))
    .ok()
}

/// Declared icons, largest first, followed by the conventional `/favicon.ico`.
pub(super) fn icon_candidates(html: &str, page: &Url) -> Vec<Url> {
    let mut links: Vec<_> = LINK_TAG
        .find_iter(html)
        .filter_map(|tag| icon_link(tag.as_str(), page))
        .collect();
    links.sort_by_key(|link| Reverse(link.size));
    let mut candidates: Vec<_> = links.into_iter().map(|link| link.url).collect();
    candidates.extend(page.join("/favicon.ico").ok().and_then(web_url));
    candidates
}

fn icon_link(tag: &str, page: &Url) -> Option<IconLink> {
    let attributes = attributes(tag);
    let declares_icon = attributes
        .get("rel")?
        .to_lowercase()
        .split_whitespace()
        .any(|token| token == "icon" || token.starts_with("apple-touch-icon"));
    let url = page
        .join(attributes.get("href")?.trim())
        .ok()
        .and_then(web_url)?;
    // The decoder handles raster formats only.
    let is_svg = url.path().to_lowercase().ends_with(".svg")
        || attributes
            .get("type")
            .is_some_and(|mime| mime.contains("svg"));
    (declares_icon && !is_svg).then(|| IconLink {
        size: attributes
            .get("sizes")
            .map_or(0, |sizes| largest_dimension(sizes)),
        url,
    })
}

fn attributes(tag: &str) -> HashMap<String, String> {
    ATTRIBUTE
        .captures_iter(tag)
        .filter_map(|captures| {
            let value = (2..=4).find_map(|group| captures.get(group))?;
            Some((captures[1].to_lowercase(), value.as_str().to_owned()))
        })
        .collect()
}

fn largest_dimension(sizes: &str) -> u32 {
    sizes
        .split_whitespace()
        .filter_map(|size| size.to_lowercase().split_once('x')?.0.parse().ok())
        .max()
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn url(text: &str) -> Url {
        Url::parse(text).unwrap()
    }

    fn candidate_strings(html: &str, page: &str) -> Vec<String> {
        icon_candidates(html, &url(page))
            .iter()
            .map(Url::to_string)
            .collect()
    }

    #[test]
    fn site_icon_prefers_largest_declared_size() {
        let html = r#"<head>
            <link rel="icon" href="/small.png" sizes="16x16">
            <link rel="apple-touch-icon" href="/touch.png" sizes="180x180">
            <link rel="shortcut icon" href="/medium.ico" sizes="32x32 48x48">
        </head>"#;
        assert_eq!(
            candidate_strings(html, "https://example.org/"),
            [
                "https://example.org/touch.png",
                "https://example.org/medium.ico",
                "https://example.org/small.png",
                "https://example.org/favicon.ico",
            ]
        );
    }

    #[test]
    fn site_icon_keeps_document_order_for_equal_sizes() {
        let html = r#"<link rel=icon href=a.png><link href='b.png' rel='icon'>"#;
        assert_eq!(
            candidate_strings(html, "https://example.org/docs/"),
            [
                "https://example.org/docs/a.png",
                "https://example.org/docs/b.png",
                "https://example.org/favicon.ico",
            ]
        );
    }

    #[test]
    fn site_icon_skips_svg_and_unrelated_links() {
        let html = r#"
            <link rel="icon" href="/logo.svg">
            <link rel="icon" type="image/svg+xml" href="/logo">
            <link rel="mask-icon" href="/mask.png">
            <link rel="stylesheet" href="/style.css">
            <link rel="icon" href="javascript:void(0)">"#;
        assert_eq!(
            candidate_strings(html, "https://example.org/"),
            ["https://example.org/favicon.ico"]
        );
    }

    #[test]
    fn site_icon_resolves_relative_protocol_relative_and_absolute_hrefs() {
        let html = r#"
            <link rel="icon" href="../img/a.png" sizes="64x64">
            <link rel="icon" href="//cdn.example.net/b.png" sizes="48x48">
            <link rel="icon" href="https://static.example.com/c.png" sizes="32x32">"#;
        assert_eq!(
            candidate_strings(html, "https://example.org/docs/start/"),
            [
                "https://example.org/docs/img/a.png",
                "https://cdn.example.net/b.png",
                "https://static.example.com/c.png",
                "https://example.org/favicon.ico",
            ]
        );
    }

    #[test]
    fn site_icon_without_markup_falls_back_to_origin_favicon() {
        assert_eq!(
            candidate_strings("", "https://bun.sh/docs?x=1#top"),
            ["https://bun.sh/favicon.ico"]
        );
    }

    #[test]
    fn site_icon_ignores_unsized_marker_values() {
        assert_eq!(largest_dimension("any"), 0);
        assert_eq!(largest_dimension("16x16 256X256 32x32"), 256);
    }

    #[test]
    fn github_avatar_uses_repository_owner() {
        for homepage in [
            "https://github.com/astral-sh/uv",
            "https://github.com/astral-sh/uv#readme",
            "https://www.github.com/astral-sh",
        ] {
            assert_eq!(
                github_avatar(&url(homepage)),
                Some(url("https://github.com/astral-sh.png?size=64"))
            );
        }
    }

    #[test]
    fn github_avatar_ignores_other_hosts_and_owner_less_urls() {
        assert_eq!(github_avatar(&url("https://github.com/")), None);
        assert_eq!(github_avatar(&url("https://gitlab.com/group/app")), None);
        assert_eq!(
            github_avatar(&url("https://github.com.evil.example/a")),
            None
        );
    }
}
