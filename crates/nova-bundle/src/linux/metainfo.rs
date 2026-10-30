//! The AppStream metainfo, `<identifier>.metainfo.xml`: what software
//! centres (GNOME Software, KDE Discover) show for the game.

use super::EXECUTABLE;
use super::desktop::desktop_file_name;
use crate::build_id::BuildId;
use crate::spec::BundleSpec;
use crate::xml::escape;

/// The licence of the metainfo file itself, which AppStream requires to
/// be permissive.
pub const METADATA_LICENSE: &str = "CC0-1.0";

/// The metainfo file's name.
#[must_use]
pub fn metainfo_file_name(spec: &BundleSpec) -> String {
    format!("{}.metainfo.xml", spec.identifier)
}

/// Renders the metainfo XML: a desktop application with the spec's
/// identifier, licence, name, summary, description (one paragraph),
/// developer (its `name`, and its reverse-DNS `id` attribute) and
/// homepage, launched by the desktop file, providing the
/// [`EXECUTABLE`] binary, with one `<release>` for `build`.
///
/// Its OARS rating is mild fantasy violence: ships fight and explode in
/// space. The release's `version` is the build ID itself, as macOS's
/// `CFBundleVersion` is, and its `date` the ID's day: the 14 digits sort
/// in time order under AppStream's version compare.
#[must_use]
pub fn metainfo(spec: &BundleSpec, build: &BuildId) -> String {
    let id = escape(&spec.identifier);
    let license = escape(&spec.license);
    let name = escape(&spec.name);
    let summary = escape(&spec.summary);
    let description = escape(&spec.description);
    let launchable = escape(&desktop_file_name(spec));
    let developer = escape(&spec.developer);
    let developer_id = escape(&spec.developer_id);
    let homepage = escape(&spec.homepage);
    let binary = escape(EXECUTABLE);
    let version = build.as_str();
    let date = build.date();
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<component type="desktop-application">
  <id>{id}</id>
  <metadata_license>{METADATA_LICENSE}</metadata_license>
  <project_license>{license}</project_license>
  <name>{name}</name>
  <summary>{summary}</summary>
  <description>
    <p>{description}</p>
  </description>
  <launchable type="desktop-id">{launchable}</launchable>
  <developer id="{developer_id}">
    <name>{developer}</name>
  </developer>
  <url type="homepage">{homepage}</url>
  <content_rating type="oars-1.1">
    <content_attribute id="violence-fantasy">mild</content_attribute>
  </content_rating>
  <provides>
    <binary>{binary}</binary>
  </provides>
  <releases>
    <release version="{version}" date="{date}"/>
  </releases>
</component>
"#
    )
}

#[cfg(test)]
mod tests {
    use roxmltree::{Document, Node};

    use super::*;
    use crate::spec::tests::sample_metadata;
    use crate::test_support::build;

    fn spec() -> BundleSpec {
        BundleSpec::from_metadata(&sample_metadata()).expect("spec")
    }

    fn child<'a, 'i>(node: Node<'a, 'i>, name: &str) -> Node<'a, 'i> {
        node.children()
            .find(|n| n.has_tag_name(name))
            .unwrap_or_else(|| panic!("no <{name}>"))
    }

    fn text<'a>(node: Node<'a, '_>, name: &str) -> &'a str {
        child(node, name).text().unwrap_or_default()
    }

    #[test]
    fn the_file_is_named_for_the_identifier() {
        assert_eq!(metainfo_file_name(&spec()), "io.example.nova.metainfo.xml");
    }

    #[test]
    fn describes_a_desktop_application_from_the_spec() {
        let xml = metainfo(&spec(), &build());
        let doc = Document::parse(&xml).expect("xml");
        let component = doc.root_element();
        assert_eq!(component.tag_name().name(), "component");
        assert_eq!(component.attribute("type"), Some("desktop-application"));
        for (name, want) in [
            ("id", "io.example.nova"),
            ("metadata_license", "CC0-1.0"),
            ("project_license", "GPL-3.0-or-later"),
            ("name", "EV Nova"),
            ("summary", "Fly the stars"),
            ("url", "https://example.com/nova"),
            ("launchable", "io.example.nova.desktop"),
        ] {
            assert_eq!(text(component, name), want, "<{name}>");
        }
        assert_eq!(child(component, "url").attribute("type"), Some("homepage"));
        assert_eq!(
            child(component, "launchable").attribute("type"),
            Some("desktop-id")
        );
        let description = child(component, "description");
        assert_eq!(text(description, "p"), "A space trading and combat game.");
        assert_eq!(
            text(child(component, "developer"), "name"),
            "the nova authors"
        );
        assert_eq!(
            child(component, "developer").attribute("id"),
            Some("io.example")
        );
        assert_eq!(text(child(component, "provides"), "binary"), "nova");
    }

    #[test]
    fn has_one_release_for_the_build() {
        let build = crate::build_id::BuildId::parse("20261028123456").expect("an ID");
        let xml = metainfo(&spec(), &build);
        let doc = Document::parse(&xml).expect("xml");
        let releases = child(doc.root_element(), "releases");
        let release: Vec<_> = releases.children().filter(Node::is_element).collect();
        assert_eq!(release.len(), 1, "{xml}");
        assert_eq!(release[0].tag_name().name(), "release");
        assert_eq!(release[0].attribute("version"), Some("20261028123456"));
        assert_eq!(release[0].attribute("date"), Some("2026-10-28"));
    }

    #[test]
    fn rates_the_content_with_oars() {
        let xml = metainfo(&spec(), &build());
        let doc = Document::parse(&xml).expect("xml");
        let rating = child(doc.root_element(), "content_rating");
        assert_eq!(rating.attribute("type"), Some("oars-1.1"));
        let attributes: Vec<_> = rating
            .children()
            .filter(|n| n.has_tag_name("content_attribute"))
            .map(|n| {
                (
                    n.attribute("id").expect("an id"),
                    n.text().unwrap_or_default(),
                )
            })
            .collect();
        assert_eq!(attributes, [("violence-fantasy", "mild")]);
    }

    #[test]
    fn escapes_markup_in_the_values() {
        let mut spec = spec();
        spec.name = "EV <Nova> & co".into();
        spec.summary = "\"Fly\" & 'trade'".into();
        spec.description = "<p>not a paragraph</p>".into();
        spec.developer = "A & B".into();
        let xml = metainfo(&spec, &build());
        let doc = Document::parse(&xml).expect("xml");
        let component = doc.root_element();
        assert_eq!(text(component, "name"), "EV <Nova> & co");
        assert_eq!(text(component, "summary"), "\"Fly\" & 'trade'");
        assert_eq!(
            text(child(component, "description"), "p"),
            "<p>not a paragraph</p>"
        );
        assert_eq!(text(child(component, "developer"), "name"), "A & B");
    }

    #[test]
    fn escapes_markup_in_the_developer_id() {
        let mut spec = spec();
        spec.developer_id = "a\"b<c&d".into();
        let xml = metainfo(&spec, &build());
        let doc = Document::parse(&xml).expect("xml");
        assert_eq!(
            child(doc.root_element(), "developer").attribute("id"),
            Some("a\"b<c&d")
        );
    }

    #[test]
    fn says_it_is_utf_8() {
        assert!(
            metainfo(&spec(), &build()).starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n")
        );
    }
}
