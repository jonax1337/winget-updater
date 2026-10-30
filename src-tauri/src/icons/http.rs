use std::net::{Ipv4Addr, Ipv6Addr};
use std::sync::LazyLock;
use std::time::Duration;

use ureq::http::{Response, header::LOCATION};
use ureq::{Agent, Body};
use url::{Host, Url};

const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_REDIRECTS: u32 = 5;
pub(super) const MAX_ICON_BYTES: u64 = 1024 * 1024;
pub(super) const MAX_METADATA_BYTES: u64 = 8 * 1024 * 1024;

static AGENT: LazyLock<Agent> = LazyLock::new(|| {
    Agent::config_builder()
        .timeout_global(Some(REQUEST_TIMEOUT))
        // Redirects are followed by `download` so every hop can be checked.
        .max_redirects(0)
        .user_agent("winget-updater")
        .build()
        .into()
});

pub(super) struct Download {
    /// Location after redirects; relative links on the page resolve against it.
    pub final_url: Url,
    pub body: Vec<u8>,
}

pub(super) fn download(url: &Url, max_bytes: u64) -> Option<Download> {
    let mut current = url.clone();
    for _ in 0..=MAX_REDIRECTS {
        if !is_public_web_url(&current) {
            return None;
        }
        let mut response = AGENT.get(current.as_str()).call().ok()?;
        if !response.status().is_redirection() {
            let body = response
                .body_mut()
                .with_config()
                .limit(max_bytes)
                .read_to_vec()
                .ok()?;
            return Some(Download {
                final_url: current,
                body,
            });
        }
        current = redirect_target(&current, &response)?;
    }
    None
}

fn redirect_target(current: &Url, response: &Response<Body>) -> Option<Url> {
    let location = response.headers().get(LOCATION)?.to_str().ok()?;
    current.join(location).ok()
}

/// URLs come from third-party metadata, so they must not reach the local machine or network.
fn is_public_web_url(url: &Url) -> bool {
    has_web_scheme(url)
        && match url.host() {
            Some(Host::Domain(domain)) => !is_localhost(domain),
            Some(Host::Ipv4(address)) => !is_internal_ipv4(address),
            Some(Host::Ipv6(address)) => !is_internal_ipv6(address),
            None => false,
        }
}

fn is_localhost(domain: &str) -> bool {
    let domain = domain.trim_end_matches('.');
    domain == "localhost" || domain.ends_with(".localhost")
}

fn is_internal_ipv4(address: Ipv4Addr) -> bool {
    let is_carrier_grade_nat = address.octets()[0] == 100 && address.octets()[1] & 0xc0 == 0x40;
    address.is_loopback()
        || address.is_private()
        || address.is_link_local()
        || address.is_unspecified()
        || is_carrier_grade_nat
}

fn is_internal_ipv6(address: Ipv6Addr) -> bool {
    if let Some(mapped) = address.to_ipv4_mapped() {
        return is_internal_ipv4(mapped);
    }
    address.is_loopback()
        || address.is_unspecified()
        || address.is_unique_local()
        || address.is_unicast_link_local()
}

fn has_web_scheme(url: &Url) -> bool {
    matches!(url.scheme(), "http" | "https")
}

pub(super) fn web_url(url: Url) -> Option<Url> {
    has_web_scheme(&url).then_some(url)
}

pub(super) fn parse_web_url(text: &str) -> Option<Url> {
    Url::parse(text.trim()).ok().and_then(web_url)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_http_and_https_are_web_urls() {
        assert!(parse_web_url(" https://bun.sh/ ").is_some());
        assert!(parse_web_url("http://example.org").is_some());
        assert!(parse_web_url("file:///C:/secret.ico").is_none());
        assert!(parse_web_url("data:image/png;base64,AAAA").is_none());
        assert!(parse_web_url("not a url").is_none());
    }

    fn is_public(text: &str) -> bool {
        is_public_web_url(&Url::parse(text).unwrap())
    }

    #[test]
    fn public_hosts_are_allowed() {
        assert!(is_public("https://bun.sh/favicon.ico"));
        assert!(is_public("http://93.184.216.34/"));
        assert!(is_public("https://[2606:4700:4700::1111]/"));
        assert!(is_public("http://100.63.0.1/"));
        assert!(is_public("http://100.128.0.1/"));
    }

    #[test]
    fn localhost_names_are_rejected() {
        assert!(!is_public("http://localhost/"));
        assert!(!is_public("http://LOCALHOST:8080/"));
        assert!(!is_public("http://localhost./"));
        assert!(!is_public("http://app.localhost/"));
    }

    #[test]
    fn internal_ipv4_addresses_are_rejected() {
        for host in [
            "127.0.0.1",
            "127.1",
            "2130706433",
            "10.0.0.1",
            "172.16.5.4",
            "192.168.1.1",
            "169.254.169.254",
            "0.0.0.0",
            "100.64.0.1",
            "100.127.255.254",
        ] {
            assert!(!is_public(&format!("http://{host}/")), "{host}");
        }
    }

    #[test]
    fn internal_ipv6_addresses_are_rejected() {
        for host in [
            "::1",
            "::",
            "fe80::1",
            "fc00::1",
            "fd12:3456::1",
            "::ffff:127.0.0.1",
            "::ffff:192.168.0.1",
            "::ffff:100.64.0.1",
        ] {
            assert!(!is_public(&format!("http://[{host}]/")), "{host}");
        }
        assert!(is_public("http://[::ffff:93.184.216.34]/"));
    }

    #[test]
    fn non_web_schemes_are_rejected() {
        assert!(!is_public("ftp://example.org/icon.ico"));
    }
}
