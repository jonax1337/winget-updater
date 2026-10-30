use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use regex::Regex;
use serde::Deserialize;
use url::Url;

use super::http::{self, MAX_METADATA_BYTES, parse_web_url};
use super::{is_plain_path_segment, scoop_root};
use crate::models::{ManagerId, PackageRef};
use crate::process;

const WINGET_SHOW_TIMEOUT: Duration = Duration::from_secs(20);
const NPM_REGISTRY: &str = "https://registry.npmjs.org/";
const PYPI_REGISTRY: &str = "https://pypi.org/pypi/";
// winget prints these labels in the user's UI language, in order of preference.
const WINGET_HOMEPAGE_LABELS: [&str; 4] =
    ["homepage", "startseite", "publisher url", "herausgeber-url"];
const PYPI_PROJECT_URL_KEYS: [&str; 3] = ["homepage", "home", "source"];

/// Where a package's icon may be found online, as published by its package manager.
#[derive(Debug, Default, PartialEq)]
pub(super) struct SiteHints {
    pub icon_url: Option<Url>,
    pub homepage: Option<Url>,
}

impl SiteHints {
    fn with_homepage(homepage: Option<Url>) -> Self {
        Self {
            icon_url: None,
            homepage,
        }
    }
}

pub(super) fn site_hints(package: &PackageRef) -> SiteHints {
    let id = package.id.as_str();
    match package.manager {
        ManagerId::Winget => SiteHints::with_homepage(winget_homepage_of(id)),
        ManagerId::Chocolatey => chocolatey_hints(id),
        ManagerId::Scoop => SiteHints::with_homepage(scoop_homepage_of(id)),
        ManagerId::Npm => SiteHints::with_homepage(npm_homepage_of(id)),
        ManagerId::Pip => SiteHints::with_homepage(pypi_homepage_of(id)),
    }
}

fn winget_homepage_of(id: &str) -> Option<Url> {
    process::validate_package_id(id).ok()?;
    let args = ["show", "--id", id, "--exact", "--accept-source-agreements"];
    let output = process::run("winget", &args, WINGET_SHOW_TIMEOUT).ok()?;
    output
        .success
        .then(|| winget_homepage(&output.stdout))
        .flatten()
}

fn winget_homepage(show_output: &str) -> Option<Url> {
    let labelled_urls: Vec<_> = show_output.lines().filter_map(labelled_url).collect();
    WINGET_HOMEPAGE_LABELS.iter().find_map(|wanted| {
        labelled_urls
            .iter()
            .find(|(label, _)| label == wanted)
            .map(|(_, url)| url.clone())
    })
}

fn labelled_url(line: &str) -> Option<(String, Url)> {
    let (label, value) = line.split_once(':')?;
    Some((label.trim().to_lowercase(), parse_web_url(value)?))
}

fn chocolatey_hints(id: &str) -> SiteHints {
    chocolatey_nuspec_path(id)
        .and_then(|path| fs::read_to_string(path).ok())
        .map(|nuspec| nuspec_hints(&nuspec))
        .unwrap_or_default()
}

fn chocolatey_nuspec_path(id: &str) -> Option<PathBuf> {
    is_plain_path_segment(id).then_some(())?;
    let root = std::env::var_os("ChocolateyInstall")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("ProgramData").map(|data| PathBuf::from(data).join("chocolatey"))
        })?;
    Some(root.join("lib").join(id).join(format!("{id}.nuspec")))
}

fn nuspec_hints(nuspec: &str) -> SiteHints {
    SiteHints {
        icon_url: xml_text(nuspec, "iconUrl").and_then(|text| parse_web_url(&text)),
        homepage: xml_text(nuspec, "projectUrl").and_then(|text| parse_web_url(&text)),
    }
}

fn xml_text(xml: &str, tag: &str) -> Option<String> {
    let pattern = Regex::new(&format!(r"(?s)<{tag}>(.*?)</{tag}>")).ok()?;
    let text = pattern.captures(xml)?[1].trim().replace("&amp;", "&");
    Some(text)
}

#[derive(Deserialize)]
struct ScoopManifest {
    homepage: Option<String>,
}

fn scoop_homepage_of(id: &str) -> Option<Url> {
    is_plain_path_segment(id).then_some(())?;
    let manifest = scoop_root()?
        .join("apps")
        .join(id)
        .join("current")
        .join("manifest.json");
    scoop_homepage(&fs::read(manifest).ok()?)
}

fn scoop_homepage(manifest: &[u8]) -> Option<Url> {
    let manifest: ScoopManifest = serde_json::from_slice(manifest).ok()?;
    parse_web_url(&manifest.homepage?)
}

#[derive(Deserialize)]
struct NpmManifest {
    homepage: Option<String>,
    repository: Option<NpmRepository>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum NpmRepository {
    Location(String),
    Object { url: Option<String> },
}

fn npm_homepage_of(name: &str) -> Option<Url> {
    process::validate_package_id(name).ok()?;
    // `/latest` is the single version manifest; the full document grows with every release.
    let url = registry_url(NPM_REGISTRY, &[name, "latest"])?;
    npm_homepage(&http::download(&url, MAX_METADATA_BYTES)?.body)
}

fn npm_homepage(manifest: &[u8]) -> Option<Url> {
    let manifest: NpmManifest = serde_json::from_slice(manifest).ok()?;
    let repository = manifest.repository.and_then(|repository| match repository {
        NpmRepository::Location(location) => Some(location),
        NpmRepository::Object { url } => url,
    });
    manifest
        .homepage
        .iter()
        .chain(repository.iter())
        .find_map(|location| parse_web_url(&web_location(location)))
}

// npm repository fields use git transport schemes: git+https://… or git://…
fn web_location(location: &str) -> String {
    let location = location.trim().trim_start_matches("git+");
    match location.strip_prefix("git://") {
        Some(rest) => format!("https://{rest}"),
        None => location.to_owned(),
    }
}

#[derive(Deserialize)]
struct PypiResponse {
    info: PypiInfo,
}

#[derive(Deserialize)]
struct PypiInfo {
    home_page: Option<String>,
    project_urls: Option<HashMap<String, Option<String>>>,
}

fn pypi_homepage_of(name: &str) -> Option<Url> {
    process::validate_package_id(name).ok()?;
    let url = registry_url(PYPI_REGISTRY, &[name, "json"])?;
    pypi_homepage(&http::download(&url, MAX_METADATA_BYTES)?.body)
}

fn pypi_homepage(document: &[u8]) -> Option<Url> {
    let info = serde_json::from_slice::<PypiResponse>(document).ok()?.info;
    let project_urls = info.project_urls.unwrap_or_default();
    let project_url = |wanted: &str| {
        project_urls
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(wanted))
            .and_then(|(_, url)| url.as_deref())
    };
    PYPI_PROJECT_URL_KEYS
        .iter()
        .filter_map(|wanted| project_url(wanted))
        .chain(info.home_page.as_deref())
        .find_map(parse_web_url)
}

// Pushing segments percent-encodes separators, e.g. "@scope/name" becomes "@scope%2Fname".
fn registry_url(base: &str, segments: &[&str]) -> Option<Url> {
    let mut url = Url::parse(base).ok()?;
    url.path_segments_mut()
        .ok()?
        .pop_if_empty()
        .extend(segments);
    Some(url)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn url(text: &str) -> Option<Url> {
        Some(Url::parse(text).unwrap())
    }

    #[test]
    fn winget_homepage_reads_german_labels() {
        let output = include_str!("../../tests/fixtures/winget_show_german.txt");
        assert_eq!(
            winget_homepage(output),
            url("https://github.com/astral-sh/uv")
        );
    }

    #[test]
    fn winget_homepage_reads_english_labels() {
        let output = include_str!("../../tests/fixtures/winget_show_english.txt");
        assert_eq!(
            winget_homepage(output),
            url("https://github.com/astral-sh/uv")
        );
    }

    #[test]
    fn winget_homepage_falls_back_to_publisher_url_in_both_languages() {
        for output in [
            "Publisher Url: https://bun.sh/\nPublisher Support Url: https://example.org/help",
            "Herausgeber-Support-URL: https://example.org/hilfe\nHerausgeber-URL: https://bun.sh/",
        ] {
            assert_eq!(winget_homepage(output), url("https://bun.sh/"));
        }
    }

    #[test]
    fn winget_homepage_prefers_homepage_over_publisher_regardless_of_line_order() {
        let output =
            "Startseite: https://site.example/\nHerausgeber-URL: https://publisher.example/";
        assert_eq!(winget_homepage(output), url("https://site.example/"));
        let output =
            "Herausgeber-URL: https://publisher.example/\nStartseite: https://site.example/";
        assert_eq!(winget_homepage(output), url("https://site.example/"));
    }

    #[test]
    fn winget_homepage_ignores_support_license_and_release_note_urls() {
        let output = "Herausgeber-Support-URL: https://a.example/\nLizenz-URL: https://b.example/\nDocs: https://c.example/";
        assert_eq!(winget_homepage(output), None);
        assert_eq!(winget_homepage("Startseite: kein Link"), None);
    }

    #[test]
    fn nuspec_provides_icon_and_project_url() {
        let nuspec = r#"<package><metadata>
            <projectUrl>https://example.org/app</projectUrl>
            <iconUrl>
              https://cdn.example.org/icon.png?a=1&amp;b=2
            </iconUrl></metadata></package>"#;
        assert_eq!(
            nuspec_hints(nuspec),
            SiteHints {
                icon_url: url("https://cdn.example.org/icon.png?a=1&b=2"),
                homepage: url("https://example.org/app"),
            }
        );
    }

    #[test]
    fn nuspec_without_urls_has_no_hints() {
        assert_eq!(nuspec_hints("<package/>"), SiteHints::default());
    }

    #[test]
    fn scoop_manifest_homepage() {
        let manifest = br#"{"version":"1.0","homepage":"https://github.com/BurntSushi/ripgrep"}"#;
        assert_eq!(
            scoop_homepage(manifest),
            url("https://github.com/BurntSushi/ripgrep")
        );
        assert_eq!(scoop_homepage(br#"{"version":"1.0"}"#), None);
        assert_eq!(scoop_homepage(b"not json"), None);
    }

    #[test]
    fn npm_manifest_prefers_homepage() {
        let manifest = br##"{"homepage":"https://github.com/npm/cli#readme","repository":{"url":"git+https://github.com/other/repo.git"}}"##;
        assert_eq!(
            npm_homepage(manifest),
            url("https://github.com/npm/cli#readme")
        );
    }

    #[test]
    fn npm_manifest_falls_back_to_repository() {
        let object =
            br#"{"repository":{"type":"git","url":"git+https://github.com/vercel/next.js.git"}}"#;
        assert_eq!(
            npm_homepage(object),
            url("https://github.com/vercel/next.js.git")
        );
        let text = br#"{"repository":"git://github.com/expressjs/express.git"}"#;
        assert_eq!(
            npm_homepage(text),
            url("https://github.com/expressjs/express.git")
        );
        assert_eq!(npm_homepage(br#"{"repository":"owner/repo"}"#), None);
    }

    #[test]
    fn pypi_prefers_homepage_project_url_then_source_then_home_page() {
        let both = br#"{"info":{"home_page":"https://legacy.example/","project_urls":{"Source":"https://github.com/psf/requests","homepage":"https://requests.readthedocs.io"}}}"#;
        assert_eq!(pypi_homepage(both), url("https://requests.readthedocs.io/"));
        let source_only = br#"{"info":{"home_page":"","project_urls":{"Source":"https://github.com/astral-sh/ruff","Changelog":null}}}"#;
        assert_eq!(
            pypi_homepage(source_only),
            url("https://github.com/astral-sh/ruff")
        );
        let legacy = br#"{"info":{"home_page":"https://legacy.example/","project_urls":null}}"#;
        assert_eq!(pypi_homepage(legacy), url("https://legacy.example/"));
    }

    #[test]
    fn registry_url_encodes_scoped_package_names() {
        assert_eq!(
            registry_url(NPM_REGISTRY, &["@angular/cli", "latest"]),
            url("https://registry.npmjs.org/@angular%2Fcli/latest")
        );
        assert_eq!(
            registry_url(PYPI_REGISTRY, &["requests", "json"]),
            url("https://pypi.org/pypi/requests/json")
        );
    }
}
