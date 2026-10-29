//! The download page the `pages` job deploys at the site's root: one
//! static `index.html` with inline CSS, no script and no web font, that
//! works in light and dark mode and at phone width.
//!
//! The page is data ([`Site`]) rendered by one template ([`render`]).
//! [`stock`] builds the nightly's page from the same constants that name
//! the release assets ([`layout::DMG`], [`installer::MSI`],
//! [`flatpak::ASSET`] and [`archive`]'s), so a renamed asset renames its
//! link. When the deploy serves the signed Flatpak repo ([`FlatpakRepo`]),
//! the Linux row leads with the nightly's `.flatpakref` on [`PAGES`]. Each [`Link`] holds a full URL, so a channel whose files live
//! elsewhere (a `.flatpakref` served from Pages, a `beta` release) is more
//! data, not a new template. [`write_site`] writes the page through the
//! [`Files`] port, and [`host`] reads `nova-site`'s arguments.

pub mod host;

use std::io;
use std::path::{Path, PathBuf};

use crate::appcast::Channel as AppChannel;
use crate::archive;
use crate::installer;
use crate::layout;
use crate::linux::flatpak;
use crate::ports::Files;
use crate::xml::escape;

/// The repository the releases are published from.
pub const REPO: &str = "https://github.com/edpaget/ev-nova";

/// The Pages site's root: the repository's project site
/// (`infra/github/pages.tf` sets no custom domain). The download page is
/// served at it, and the Flatpak repo under it.
pub const PAGES: &str = "https://edpaget.github.io/ev-nova";

/// The rolling prerelease's tag.
pub const NIGHTLY: &str = "nightly";

/// The rolling beta prerelease's tag: nightlies promoted by hand.
pub const BETA: &str = "beta";

/// The page's file, at the output directory's root.
pub const INDEX: &str = "index.html";

/// The game's name, the page's heading.
pub const NAME: &str = "EV Nova";

/// The page's one sentence.
pub const SUMMARY: &str = "An engine for EV Nova that plays the game from your own \
     Nova Files; no game data is included.";

/// The page's stylesheet: the system's fonts, and colours that follow its
/// light or dark scheme.
const STYLE: &str = "\
:root { --bg: #ffffff; --fg: #1b1b1f; --muted: #5d5d66; --accent: #0b57d0; --line: #dedee3; }
@media (prefers-color-scheme: dark) {
  :root { --bg: #121216; --fg: #ececf1; --muted: #a2a2ad; --accent: #8ab4f8; --line: #2e2e36; }
}
* { box-sizing: border-box; }
body { margin: 0; background: var(--bg); color: var(--fg);
  font: 16px/1.5 system-ui, -apple-system, \"Segoe UI\", Roboto, sans-serif; }
main { max-width: 36rem; margin: 0 auto; padding: 2.5rem 1rem; }
h1 { font-size: 2rem; line-height: 1.2; margin: 0 0 0.5rem; }
p { color: var(--muted); margin: 0 0 2rem; }
h2 { font-size: 0.8rem; font-weight: 600; letter-spacing: 0.08em; text-transform: uppercase;
  color: var(--muted); margin: 0 0 0.5rem; }
ul { list-style: none; margin: 0; padding: 0; }
a { color: var(--accent); }
.downloads li { display: flex; flex-wrap: wrap; align-items: baseline; gap: 0.25rem 1rem;
  padding: 0.75rem 0; border-top: 1px solid var(--line); }
.downloads li:last-child { border-bottom: 1px solid var(--line); }
.platform { font-weight: 600; min-width: 5.5rem; }
.primary { font-weight: 600; }
section { margin-bottom: 2rem; }
.links { display: flex; flex-wrap: wrap; gap: 0.5rem 1.5rem; }
";

/// A link: the text shown and the full URL it goes to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Link {
    /// The text shown.
    pub label: String,
    /// The full URL.
    pub href: String,
}

impl Link {
    /// A link from its label and URL.
    pub fn new(label: impl Into<String>, href: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            href: href.into(),
        }
    }
}

/// One platform's downloads in a channel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Download {
    /// The platform's name, as `macOS`.
    pub platform: String,
    /// The download most players want: the installer.
    pub primary: Link,
    /// Other forms of the same build, as a portable archive.
    pub extras: Vec<Link>,
}

/// A release channel, such as the nightly, and its downloads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Channel {
    /// The channel's name, as `Nightly`.
    pub label: String,
    /// One per platform.
    pub downloads: Vec<Download>,
}

/// Everything the page shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Site {
    /// The game's name, the page's heading and title.
    pub name: String,
    /// The one sentence under the heading.
    pub summary: String,
    /// The channels, in order.
    pub channels: Vec<Channel>,
    /// The links under the downloads: the releases, the repository, the
    /// readme.
    pub links: Vec<Link>,
}

/// The URL of `asset` in the release tagged `tag` of `repo`.
#[must_use]
pub fn release_asset(repo: &str, tag: &str, asset: &str) -> String {
    format!("{repo}/releases/download/{tag}/{asset}")
}

/// Whether this deploy serves the signed Flatpak repo
/// ([`flatpak::repo`]), which `ci.yml` knows from the `flatpak-repo`
/// job's output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlatpakRepo {
    /// It doesn't: the Linux row leads with the plain bundle.
    Absent,
    /// It does: the Linux row leads with the `.flatpakref`, which installs
    /// from the repo and updates, then the bundle, which follows it too.
    Published,
}

/// The Linux row: the `.flatpakref` first when the repo is published.
fn linux(repo: FlatpakRepo, nightly: impl Fn(&str) -> String) -> Download {
    let bundle = Link::new("flatpak", nightly(flatpak::ASSET));
    let tarball = Link::new("tar.gz", nightly(archive::LINUX.file));
    let (primary, extras) = match repo {
        FlatpakRepo::Absent => (bundle, vec![tarball]),
        FlatpakRepo::Published => (
            Link::new(
                "flatpakref",
                flatpak::repo::flatpakref_url(AppChannel::Nightly),
            ),
            vec![bundle, tarball],
        ),
    };
    Download {
        platform: "Linux".to_owned(),
        primary,
        extras,
    }
}

/// The page the `pages` job deploys: the nightly's downloads, linked by
/// the names the packers give them, and the `.flatpakref` when `repo` is
/// published.
#[must_use]
pub fn stock(repo: FlatpakRepo) -> Site {
    let nightly = |asset: &str| release_asset(REPO, NIGHTLY, asset);
    Site {
        name: NAME.to_owned(),
        summary: SUMMARY.to_owned(),
        channels: vec![Channel {
            label: "Nightly".to_owned(),
            downloads: vec![
                Download {
                    platform: "macOS".to_owned(),
                    primary: Link::new("dmg", nightly(layout::DMG)),
                    extras: Vec::new(),
                },
                Download {
                    platform: "Windows".to_owned(),
                    primary: Link::new("msi", nightly(installer::MSI)),
                    extras: vec![Link::new("zip", nightly(archive::WINDOWS.file))],
                },
                linux(repo, nightly),
            ],
        }],
        links: vec![
            Link::new("All releases", format!("{REPO}/releases")),
            Link::new("Source", REPO),
            Link::new(
                "How to run it",
                format!("{REPO}/blob/main/packaging/README.txt"),
            ),
        ],
    }
}

/// The page as HTML, every label and URL escaped.
#[must_use]
pub fn render(site: &Site) -> String {
    let name = escape(&site.name);
    let summary = escape(&site.summary);
    let channels: String = site.channels.iter().map(channel).collect();
    let links: String = site
        .links
        .iter()
        .map(|link| "<li>".to_owned() + &anchor(link, None) + "</li>\n")
        .collect();
    format!(
        "<!doctype html>\n\
         <html lang=\"en\">\n\
         <head>\n\
         <meta charset=\"utf-8\">\n\
         <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n\
         <meta name=\"color-scheme\" content=\"light dark\">\n\
         <title>{name}</title>\n\
         <style>\n{STYLE}</style>\n\
         </head>\n\
         <body>\n\
         <main>\n\
         <h1>{name}</h1>\n\
         <p>{summary}</p>\n\
         {channels}\
         <ul class=\"links\">\n{links}</ul>\n\
         </main>\n\
         </body>\n\
         </html>\n"
    )
}

/// One channel: its name, then a row per platform.
fn channel(channel: &Channel) -> String {
    let rows: String = channel.downloads.iter().map(download).collect();
    format!(
        "<section>\n<h2>{}</h2>\n<ul class=\"downloads\">\n{rows}</ul>\n</section>\n",
        escape(&channel.label)
    )
}

/// One platform's row: its name, its installer, then any other forms.
fn download(download: &Download) -> String {
    let extras: String = download
        .extras
        .iter()
        .map(|extra| " ".to_owned() + &anchor(extra, None))
        .collect();
    format!(
        "<li><span class=\"platform\">{}</span> {}{extras}</li>\n",
        escape(&download.platform),
        anchor(&download.primary, Some("primary")),
    )
}

/// `link` as an `<a>` element, of `class` if one is given.
fn anchor(link: &Link, class: Option<&str>) -> String {
    let class = class.map_or_else(String::new, |c| format!(" class=\"{c}\""));
    format!(
        "<a{class} href=\"{}\">{}</a>",
        escape(&link.href),
        escape(&link.label)
    )
}

/// Writes `site`'s page as [`INDEX`] in `out`, creating `out`, and
/// returns the page's path.
pub fn write_site(files: &mut impl Files, out: &Path, site: &Site) -> io::Result<PathBuf> {
    files.create_dir_all(out)?;
    let page = out.join(INDEX);
    files.write(&page, render(site).as_bytes())?;
    Ok(page)
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::*;
    use crate::linux::flatpak::repo;
    use crate::test_support::{Event, FakeFiles, Log};

    /// Every `href` value in `html`, in order.
    fn hrefs(html: &str) -> Vec<String> {
        html.split("href=\"")
            .skip(1)
            .map(|rest| rest.split('"').next().unwrap_or_default().to_owned())
            .collect()
    }

    /// `html` with the element `tag` and everything in it removed, each
    /// time it appears.
    fn without_element(html: &str, tag: &str) -> String {
        let open = format!("<{tag}");
        let close = format!("</{tag}>");
        let mut rest = html;
        let mut kept = String::new();
        while let Some(start) = rest.find(&open) {
            kept.push_str(&rest[..start]);
            let end = rest[start..]
                .find(&close)
                .map_or(rest.len(), |end| start + end + close.len());
            rest = &rest[end..];
        }
        kept.push_str(rest);
        kept
    }

    /// The text `html` shows outside its head, its heading and its links.
    fn body_copy(html: &str) -> String {
        let mut text = html.to_owned();
        for tag in ["head", "h1", "a"] {
            text = without_element(&text, tag);
        }
        let mut copy = String::new();
        let mut in_tag = false;
        for c in text.chars() {
            match c {
                '<' => in_tag = true,
                '>' => {
                    in_tag = false;
                    copy.push(' ');
                }
                c if !in_tag => copy.push(c),
                _ => {}
            }
        }
        copy
    }

    /// How many sentences `text` ends: a `.`, `!` or `?` followed by
    /// whitespace or the end.
    fn sentences(text: &str) -> usize {
        let chars: Vec<char> = text.chars().collect();
        chars
            .iter()
            .enumerate()
            .filter(|&(i, c)| {
                matches!(c, '.' | '!' | '?') && chars.get(i + 1).is_none_or(|n| n.is_whitespace())
            })
            .count()
    }

    fn nightly(asset: &str) -> String {
        release_asset(REPO, NIGHTLY, asset)
    }

    const BOTH: [FlatpakRepo; 2] = [FlatpakRepo::Absent, FlatpakRepo::Published];

    /// The stock page's Linux row.
    fn linux_row(repo: FlatpakRepo) -> Download {
        let site = stock(repo);
        site.channels[0]
            .downloads
            .iter()
            .find(|d| d.platform == "Linux")
            .expect("a Linux row")
            .clone()
    }

    #[test]
    fn a_published_repo_puts_the_flatpakref_first_in_the_linux_row() {
        let href = format!(
            "{PAGES}/{}/{}",
            repo::PAGES_DIR,
            repo::flatpakref_name(AppChannel::Nightly)
        );
        assert_eq!(
            href,
            "https://edpaget.github.io/ev-nova/flatpak/ev-nova-nightly.flatpakref"
        );
        assert_eq!(
            linux_row(FlatpakRepo::Published),
            Download {
                platform: "Linux".into(),
                primary: Link::new("flatpakref", href.clone()),
                extras: vec![
                    Link::new("flatpak", nightly(flatpak::ASSET)),
                    Link::new("tar.gz", nightly(archive::LINUX.file)),
                ],
            }
        );
        assert!(hrefs(&render(&stock(FlatpakRepo::Published))).contains(&href));
    }

    #[test]
    fn without_the_repo_the_linux_row_is_unchanged() {
        assert_eq!(
            linux_row(FlatpakRepo::Absent),
            Download {
                platform: "Linux".into(),
                primary: Link::new("flatpak", nightly(flatpak::ASSET)),
                extras: vec![Link::new("tar.gz", nightly(archive::LINUX.file))],
            }
        );
        assert!(
            !render(&stock(FlatpakRepo::Absent)).contains("flatpakref"),
            "no .flatpakref link without the repo"
        );
    }

    #[test]
    fn only_the_linux_row_differs_with_the_repo() {
        let (absent, published) = (stock(FlatpakRepo::Absent), stock(FlatpakRepo::Published));
        assert_eq!(absent.links, published.links);
        let others = |site: &Site| -> Vec<Download> {
            site.channels[0]
                .downloads
                .iter()
                .filter(|d| d.platform != "Linux")
                .cloned()
                .collect()
        };
        assert_eq!(others(&absent), others(&published));
    }

    #[test]
    fn a_release_asset_is_under_its_tags_download_path() {
        assert_eq!(
            release_asset("https://example.com/o/r", "nightly", "a.dmg"),
            "https://example.com/o/r/releases/download/nightly/a.dmg"
        );
        assert_eq!(
            release_asset("https://example.com/o/r", "beta", "b.msi"),
            "https://example.com/o/r/releases/download/beta/b.msi"
        );
    }

    #[test]
    fn the_stock_page_links_every_nightly_asset_by_its_packed_name() {
        for repo in BOTH {
            let links = hrefs(&render(&stock(repo)));
            for asset in [
                layout::DMG,
                installer::MSI,
                flatpak::ASSET,
                archive::WINDOWS.file,
                archive::LINUX.file,
            ] {
                let url = escape(&nightly(asset));
                assert!(links.contains(&url), "no link to {url} in {links:?}");
            }
        }
    }

    #[test]
    fn the_stock_page_links_the_releases_the_repository_and_the_readme() {
        let links = hrefs(&render(&stock(FlatpakRepo::Absent)));
        for url in [
            format!("{REPO}/releases"),
            REPO.to_owned(),
            format!("{REPO}/blob/main/packaging/README.txt"),
        ] {
            assert!(links.contains(&url), "no link to {url} in {links:?}");
        }
    }

    #[test]
    fn each_platform_has_its_installer_first_and_its_archive_beside_it() {
        let site = stock(FlatpakRepo::Absent);
        let [nightly_channel] = site.channels.as_slice() else {
            panic!("one channel: {:?}", site.channels);
        };
        assert_eq!(nightly_channel.label, "Nightly");
        let rows: Vec<_> = nightly_channel
            .downloads
            .iter()
            .map(|d| {
                (
                    d.platform.as_str(),
                    d.primary.href.clone(),
                    d.extras.iter().map(|l| l.href.clone()).collect::<Vec<_>>(),
                )
            })
            .collect();
        assert_eq!(
            rows,
            [
                ("macOS", nightly(layout::DMG), vec![]),
                (
                    "Windows",
                    nightly(installer::MSI),
                    vec![nightly(archive::WINDOWS.file)]
                ),
                (
                    "Linux",
                    nightly(flatpak::ASSET),
                    vec![nightly(archive::LINUX.file)]
                ),
            ]
        );
    }

    #[test]
    fn the_page_has_no_script_and_nothing_external_but_its_links() {
        for repo in BOTH {
            let html = render(&stock(repo)).to_lowercase();
            for banned in ["<script", "<link", "@import", "@font-face", "url("] {
                assert!(!html.contains(banned), "{banned} in {html}");
            }
            let links = hrefs(&html);
            assert!(!links.is_empty());
            for link in links {
                assert!(link.starts_with("https://"), "{link} is not absolute");
            }
        }
    }

    #[test]
    fn the_page_says_at_most_two_sentences_besides_its_links() {
        for repo in BOTH {
            let html = render(&stock(repo));
            let copy = body_copy(&html);
            assert!(sentences(&copy) <= 2, "{copy}");
            // The one line under the heading is there.
            assert!(copy.contains("Nova Files"), "{copy}");
            assert!(html.contains("<h1>EV Nova</h1>"), "{html}");
        }
    }

    #[test]
    fn the_copy_count_finds_the_sentences_it_should() {
        assert_eq!(sentences("One. Two! Three? Four."), 4);
        assert_eq!(sentences("tar.gz and 1.5 are no sentence"), 0);
        assert_eq!(
            body_copy("<head><title>T.</title></head><h1>H.</h1><p>S.</p><a href=\"x\">L. M.</a>")
                .trim(),
            "S."
        );
    }

    #[test]
    fn any_channel_and_link_the_data_holds_is_rendered() {
        let flatpakref = "https://edpaget.github.io/ev-nova/flatpak/ev-nova-nightly.flatpakref";
        let row = |tag: &str| Download {
            platform: "Linux".into(),
            primary: Link::new("flatpak", release_asset(REPO, tag, "x.flatpak")),
            extras: vec![Link::new("flatpakref", flatpakref)],
        };
        let site = Site {
            name: "N".into(),
            summary: "S".into(),
            channels: vec![
                Channel {
                    label: "Nightly".into(),
                    downloads: vec![row("nightly")],
                },
                Channel {
                    label: "Beta".into(),
                    downloads: vec![row("beta")],
                },
            ],
            links: vec![Link::new("Repo", REPO)],
        };
        let html = render(&site);
        let links = hrefs(&html);
        for url in [
            release_asset(REPO, "nightly", "x.flatpak"),
            release_asset(REPO, "beta", "x.flatpak"),
            flatpakref.to_owned(),
            REPO.to_owned(),
        ] {
            assert!(links.contains(&url), "no link to {url} in {links:?}");
        }
        assert_eq!(links.iter().filter(|l| *l == flatpakref).count(), 2);
        for text in [">Nightly<", ">Beta<", ">Linux<", ">flatpakref<", ">Repo<"] {
            assert!(html.contains(text), "no {text} in {html}");
        }
        let nightly_at = html.find(">Nightly<").expect("nightly");
        let beta_at = html.find(">Beta<").expect("beta");
        assert!(nightly_at < beta_at, "channels out of order: {html}");
    }

    #[test]
    fn every_label_and_url_is_escaped() {
        let site = Site {
            name: "A & <B>".into(),
            summary: "C & D".into(),
            channels: vec![Channel {
                label: "<E>".into(),
                downloads: vec![Download {
                    platform: "F&G".into(),
                    primary: Link::new("<h>", "https://x/?a=1&b=\"2\""),
                    extras: vec![Link::new("i&j", "https://y/<z>")],
                }],
            }],
            links: vec![Link::new("k<l", "https://m/&")],
        };
        let html = render(&site);
        for escaped in [
            "A &amp; &lt;B&gt;",
            "C &amp; D",
            "&lt;E&gt;",
            "F&amp;G",
            "&lt;h&gt;",
            "https://x/?a=1&amp;b=&quot;2&quot;",
            "i&amp;j",
            "https://y/&lt;z&gt;",
            "k&lt;l",
            "https://m/&amp;",
        ] {
            assert!(html.contains(escaped), "no {escaped} in {html}");
        }
        for raw in ["<B>", "<E>", "<h>", "<z>", "C & D"] {
            assert!(!html.contains(raw), "{raw} unescaped in {html}");
        }
    }

    #[test]
    fn the_page_fits_a_phone_and_follows_the_systems_colour_scheme() {
        let html = render(&stock(FlatpakRepo::Absent));
        for needle in [
            "<!doctype html>",
            "<meta charset=\"utf-8\">",
            "<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">",
            "<meta name=\"color-scheme\" content=\"light dark\">",
            "@media (prefers-color-scheme: dark)",
            "<title>EV Nova</title>",
            "<style>",
        ] {
            assert!(html.contains(needle), "no {needle} in {html}");
        }
    }

    fn files(log: &Log, fails: bool) -> FakeFiles {
        FakeFiles {
            log: Rc::clone(log),
            fails: Box::new(move |e| fails && matches!(e, Event::Write(..))),
            absent: Box::new(|_| false),
        }
    }

    #[test]
    fn writing_the_site_makes_the_directory_then_the_page() {
        let log = Log::default();
        let out = Path::new("/work").join("ev-nova").join("_site");
        let site = stock(FlatpakRepo::Absent);
        let page = write_site(&mut files(&log, false), &out, &site).expect("written");
        assert_eq!(page, out.join(INDEX));
        assert_eq!(
            *log.borrow(),
            [
                Event::CreateDir(out.clone()),
                Event::Write(out.join(INDEX), render(&site).into_bytes()),
            ]
        );
    }

    #[test]
    fn a_failed_write_is_the_error() {
        let log = Log::default();
        let out = Path::new("/work").join("_site");
        let error = write_site(&mut files(&log, true), &out, &stock(FlatpakRepo::Absent))
            .expect_err("fails");
        assert_eq!(error.to_string(), "disk full");
    }
}
