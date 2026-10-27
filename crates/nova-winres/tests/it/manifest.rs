//! `packaging/windows/nova.exe.manifest`, read as XML: each setting by its
//! element and namespace, as Windows reads them.

use roxmltree::{Document, Node};

const MANIFEST: &str = include_str!("../../../../packaging/windows/nova.exe.manifest");

const ASM_V1: &str = "urn:schemas-microsoft-com:asm.v1";
const ASM_V3: &str = "urn:schemas-microsoft-com:asm.v3";
const COMPATIBILITY: &str = "urn:schemas-microsoft-com:compatibility.v1";
const SETTINGS_2005: &str = "http://schemas.microsoft.com/SMI/2005/WindowsSettings";
const SETTINGS_2016: &str = "http://schemas.microsoft.com/SMI/2016/WindowsSettings";
const SETTINGS_2019: &str = "http://schemas.microsoft.com/SMI/2019/WindowsSettings";

/// Windows 10 and 11's `supportedOS` id.
const WINDOWS_10: &str = "{8e0f7a12-bfb3-4fe8-b9a5-48fd50a15a9a}";

/// The elements named `name` in `namespace`, anywhere in `doc`.
fn all<'a>(doc: &'a Document<'a>, namespace: &str, name: &str) -> Vec<Node<'a, 'a>> {
    doc.descendants()
        .filter(|node| {
            node.tag_name().namespace() == Some(namespace) && node.tag_name().name() == name
        })
        .collect()
}

/// The one element named `name` in `namespace`.
fn one<'a>(doc: &'a Document<'a>, namespace: &str, name: &str) -> Node<'a, 'a> {
    let [node] = all(doc, namespace, name)[..] else {
        panic!("one {namespace} {name}");
    };
    node
}

/// The text of the one `windowsSettings` child named `name` in `namespace`.
fn setting<'a>(doc: &'a Document<'a>, namespace: &str, name: &str) -> &'a str {
    let node = one(doc, namespace, name);
    let parent = node.parent_element().expect("a parent");
    assert_eq!(
        parent.tag_name().name(),
        "windowsSettings",
        "{name}'s parent"
    );
    assert_eq!(
        parent.tag_name().namespace(),
        Some(ASM_V3),
        "{name}'s parent"
    );
    node.text().expect("text").trim()
}

fn manifest() -> Document<'static> {
    Document::parse(MANIFEST).expect("the manifest is XML")
}

#[test]
fn the_manifest_is_an_assembly() {
    let doc = manifest();
    let root = doc.root_element();
    assert_eq!(root.tag_name().namespace(), Some(ASM_V1));
    assert_eq!(root.tag_name().name(), "assembly");
    assert_eq!(root.attribute("manifestVersion"), Some("1.0"));
    let identity = root
        .children()
        .find(|node| node.tag_name().name() == "assemblyIdentity")
        .expect("its own identity");
    assert_eq!(identity.attribute("type"), Some("win32"));
}

#[test]
fn the_dpi_awareness_is_per_monitor_v2() {
    let doc = manifest();
    assert_eq!(
        setting(&doc, SETTINGS_2016, "dpiAwareness"),
        "PerMonitorV2, PerMonitor"
    );
    assert_eq!(
        setting(&doc, SETTINGS_2005, "dpiAware"),
        "true/pm",
        "before Windows 10 1607"
    );
}

#[test]
fn the_code_page_is_utf_8_and_paths_may_be_long() {
    let doc = manifest();
    assert_eq!(setting(&doc, SETTINGS_2019, "activeCodePage"), "UTF-8");
    assert_eq!(setting(&doc, SETTINGS_2016, "longPathAware"), "true");
}

#[test]
fn it_runs_as_the_invoker_on_windows_10_and_11() {
    let doc = manifest();
    let level = one(&doc, ASM_V3, "requestedExecutionLevel");
    assert_eq!(level.attribute("level"), Some("asInvoker"));
    assert_eq!(level.attribute("uiAccess"), Some("false"));
    let supported: Vec<_> = all(&doc, COMPATIBILITY, "supportedOS")
        .iter()
        .filter_map(|node| node.attribute("Id"))
        .collect();
    assert_eq!(supported, [WINDOWS_10]);
}

#[test]
fn it_asks_for_common_controls_6() {
    let doc = manifest();
    let controls = all(&doc, ASM_V1, "assemblyIdentity")
        .into_iter()
        .find(|node| node.attribute("name") == Some("Microsoft.Windows.Common-Controls"))
        .expect("a Common Controls dependency");
    let dependent = controls.parent_element().expect("a parent");
    assert_eq!(dependent.tag_name().name(), "dependentAssembly");
    assert_eq!(controls.attribute("version"), Some("6.0.0.0"));
    assert_eq!(
        controls.attribute("publicKeyToken"),
        Some("6595b64144ccf1df")
    );
    assert_eq!(controls.attribute("type"), Some("win32"));
}
