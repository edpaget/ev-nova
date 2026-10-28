//! The AppStream metainfo, `<identifier>.metainfo.xml`: what software
//! centres (GNOME Software, KDE Discover) show for the game.

use super::EXECUTABLE;
use super::desktop::desktop_file_name;
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
/// developer and homepage, launched by the desktop file, providing the
/// [`EXECUTABLE`] binary.
///
/// Its OARS rating is mild fantasy violence: ships fight and explode in
/// space. It has no `<releases>`: the renderer reads no clock, and the
/// validators pass without one.
#[must_use]
pub fn metainfo(spec: &BundleSpec) -> String {
    let id = escape(&spec.identifier);
    let license = escape(&spec.license);
    let name = escape(&spec.name);
    let summary = escape(&spec.summary);
    let description = escape(&spec.description);
    let launchable = escape(&desktop_file_name(spec));
    let developer = escape(&spec.developer);
    let homepage = escape(&spec.homepage);
    let binary = escape(EXECUTABLE);
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
  <developer>
    <name>{developer}</name>
  </developer>
  <url type="homepage">{homepage}</url>
  <content_rating type="oars-1.1">
    <content_attribute id="violence-fantasy">mild</content_attribute>
  </content_rating>
  <provides>
    <binary>{binary}</binary>
  </provides>
</component>
"#
    )
}

#[cfg(test)]
mod tests {
    use roxmltree::{Document, Node};

    use super::*;
    use crate::spec::tests::sample_metadata;

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
        let xml = metainfo(&spec());
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
        assert_eq!(text(child(component, "provides"), "binary"), "nova");
    }

    #[test]
    fn rates_the_content_with_oars() {
        let xml = metainfo(&spec());
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
        let xml = metainfo(&spec);
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
    fn says_it_is_utf_8() {
        assert!(metainfo(&spec()).starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n"));
    }
}
