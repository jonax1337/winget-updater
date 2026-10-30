use std::sync::{Condvar, Mutex, MutexGuard, PoisonError};

use url::Url;

use super::http::{self, MAX_ICON_BYTES};
use super::metadata::{SiteHints, site_hints};
use super::{raster, site_icon};
use crate::models::PackageRef;

const MAX_ICON_ATTEMPTS: usize = 3;
/// A long package list requests every visible icon at once; each lookup may spawn
/// `winget show` and several downloads, so only a few run concurrently.
const MAX_CONCURRENT_LOOKUPS: usize = 4;

static LOOKUP_SLOTS: LookupSlots = LookupSlots::new(MAX_CONCURRENT_LOOKUPS);

/// Looks up the package's homepage or icon URL and downloads the icon as a PNG.
/// Any failure along the way means "no icon".
pub(super) fn download_icon(package: &PackageRef) -> Option<Vec<u8>> {
    let _slot = LOOKUP_SLOTS.acquire();
    let SiteHints { icon_url, homepage } = site_hints(package);
    icon_url
        .and_then(|url| png_from_urls(&[url]))
        .or_else(|| png_from_homepage(&homepage?))
}

fn png_from_homepage(homepage: &Url) -> Option<Vec<u8>> {
    match site_icon::github_avatar(homepage) {
        Some(avatar) => png_from_urls(&[avatar]),
        None => png_from_urls(&homepage_icon_candidates(homepage)),
    }
}

// A failing page still leaves the conventional /favicon.ico to try.
fn homepage_icon_candidates(homepage: &Url) -> Vec<Url> {
    match http::download(homepage, MAX_ICON_BYTES) {
        Some(page) => {
            site_icon::icon_candidates(&String::from_utf8_lossy(&page.body), &page.final_url)
        }
        None => site_icon::icon_candidates("", homepage),
    }
}

fn png_from_urls(urls: &[Url]) -> Option<Vec<u8>> {
    urls.iter().take(MAX_ICON_ATTEMPTS).find_map(|url| {
        let download = http::download(url, MAX_ICON_BYTES)?;
        raster::icon_png(&download.body)
    })
}

struct LookupSlots {
    free: Mutex<usize>,
    released: Condvar,
}

impl LookupSlots {
    const fn new(count: usize) -> Self {
        Self {
            free: Mutex::new(count),
            released: Condvar::new(),
        }
    }

    fn acquire(&self) -> LookupSlot<'_> {
        let mut free = self.lock();
        while *free == 0 {
            free = self
                .released
                .wait(free)
                .unwrap_or_else(PoisonError::into_inner);
        }
        *free -= 1;
        LookupSlot { slots: self }
    }

    fn lock(&self) -> MutexGuard<'_, usize> {
        self.free.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// Returns its slot when the lookup ends, even if it panics.
struct LookupSlot<'a> {
    slots: &'a LookupSlots,
}

impl Drop for LookupSlot<'_> {
    fn drop(&mut self) {
        *self.slots.lock() += 1;
        self.slots.released.notify_one();
    }
}
