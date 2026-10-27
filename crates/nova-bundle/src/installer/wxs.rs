//! The WiX source (`nova.wxs`, the WiX v4+ schema WiX v5 reads) for the
//! per-user MSI.
//!
//! - `Package Scope="perUser"`: no `ALLUSERS`, and the summary
//!   information's "elevation not required" bit, so a non-admin installs
//!   it without a UAC prompt.
//! - No `ProductCode`: WiX makes a new one for each build, and the fixed
//!   [`UPGRADE_CODE`] with `MajorUpgrade AllowSameVersionUpgrades` makes a
//!   later build of the same version replace an earlier one in place.
//! - The files go to `%LOCALAPPDATA%\Programs\Nova`, the shortcut to the
//!   user's Start menu `Programs` folder. Each component's key path is an
//!   `HKCU` value, and each folder the package makes has a `RemoveFolder`,
//!   as Windows Installer's checks (ICE38, ICE43, ICE64) require of
//!   per-user files.
//! - Nothing refers to `%APPDATA%`, where the game keeps settings and
//!   pilots.

use std::fmt::Write as _;
use std::path::Path;

use super::layout::InstallerLayout;
use super::{
    DUMP_EXE, GAME_EXE, INSTALL_DIR, InstallerError, MANUFACTURER, REGISTRY_KEY, SHORTCUT,
    UPGRADE_CODE, msi_version,
};
use crate::spec::BundleSpec;

/// The WiX v4+ schema's namespace.
pub const NAMESPACE: &str = "http://wixtoolset.org/schemas/v4/wxs";
/// The install folder's directory id, which the shortcut's target names.
pub const INSTALL_FOLDER: &str = "INSTALLFOLDER";
/// The `%LOCALAPPDATA%\Programs` folder's directory id.
pub const PROGRAMS_FOLDER: &str = "ProgramsFolder";
/// The icon's id, for the shortcut and `ARPPRODUCTICON`.
pub const ICON_ID: &str = "nova.ico";

/// Text escaped for an XML attribute value.
fn escape(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&apos;"),
            c => escaped.push(c),
        }
    }
    escaped
}

fn path(path: &Path) -> String {
    escape(&path.display().to_string())
}

/// A component in the install folder holding one file, its key path an
/// `HKCU` value named after the file.
fn file_component(doc: &mut String, id: &str, source: &Path) {
    let _ = write!(
        doc,
        r#"
      <Component Id="{id}" Directory="{INSTALL_FOLDER}">
        <File Id="{id}" Source="{source}" KeyPath="no" />
        <RegistryValue Root="HKCU" Key="{key}" Name="{id}" Type="integer" Value="1" KeyPath="yes" />
      </Component>"#,
        id = escape(id),
        source = path(source),
        key = escape(REGISTRY_KEY),
    );
}

/// The WiX source for `spec`'s installer, packing the executables in
/// `layout.exes`, `README.txt` from `packaging/` and `LICENSE` from the
/// workspace root.
pub fn wxs(spec: &BundleSpec, layout: &InstallerLayout) -> Result<String, InstallerError> {
    let version = msi_version(&spec.version)?;
    let mut doc = String::new();
    let _ = write!(
        doc,
        r#"<?xml version="1.0" encoding="utf-8"?>
<Wix xmlns="{NAMESPACE}">
  <Package Name="{name}" Manufacturer="{manufacturer}" Version="{version}" UpgradeCode="{UPGRADE_CODE}" Scope="perUser" Language="1033">
    <MajorUpgrade AllowSameVersionUpgrades="yes" DowngradeErrorMessage="A newer version of [ProductName] is already installed." />
    <MediaTemplate EmbedCab="yes" />
    <Icon Id="{ICON_ID}" SourceFile="{icon}" />
    <Property Id="ARPPRODUCTICON" Value="{ICON_ID}" />
    <Property Id="ARPNOMODIFY" Value="1" />
    <StandardDirectory Id="LocalAppDataFolder">
      <Directory Id="{PROGRAMS_FOLDER}" Name="Programs">
        <Directory Id="{INSTALL_FOLDER}" Name="{install_dir}" />
      </Directory>
    </StandardDirectory>
    <StandardDirectory Id="ProgramMenuFolder" />
    <Feature Id="Main" Title="{name}">"#,
        name = escape(&spec.name),
        manufacturer = escape(MANUFACTURER),
        icon = path(&layout.icon()),
        install_dir = escape(INSTALL_DIR),
    );
    file_component(&mut doc, GAME_EXE, &layout.exe(GAME_EXE));
    file_component(&mut doc, DUMP_EXE, &layout.exe(DUMP_EXE));
    file_component(
        &mut doc,
        "README.txt",
        &spec.workspace_root.join("packaging").join("README.txt"),
    );
    file_component(&mut doc, "LICENSE", &spec.workspace_root.join("LICENSE"));
    let _ = write!(
        doc,
        r#"
      <Component Id="Shortcut" Directory="ProgramMenuFolder">
        <Shortcut Id="Shortcut" Name="{shortcut}" Target="[{INSTALL_FOLDER}]{GAME_EXE}" WorkingDirectory="{INSTALL_FOLDER}" Icon="{ICON_ID}" />
        <RemoveFolder Id="RemoveInstallFolder" Directory="{INSTALL_FOLDER}" On="uninstall" />
        <RemoveFolder Id="RemoveProgramsFolder" Directory="{PROGRAMS_FOLDER}" On="uninstall" />
        <RegistryValue Root="HKCU" Key="{key}" Name="Shortcut" Type="integer" Value="1" KeyPath="yes" />
      </Component>
    </Feature>
  </Package>
</Wix>
"#,
        shortcut = escape(SHORTCUT),
        key = escape(REGISTRY_KEY),
    );
    Ok(doc)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use roxmltree::{Document, Node};

    use super::*;
    use crate::spec::tests::sample_metadata;

    fn spec() -> BundleSpec {
        BundleSpec::from_metadata(&sample_metadata()).expect("spec")
    }

    fn layout(spec: &BundleSpec) -> InstallerLayout {
        InstallerLayout::new(&spec.target_dir, None, None).expect("layout")
    }

    fn source() -> String {
        let spec = spec();
        wxs(&spec, &layout(&spec)).expect("wxs")
    }

    /// The elements named `name`, in the WiX namespace.
    fn all<'a>(doc: &'a Document<'a>, name: &str) -> Vec<Node<'a, 'a>> {
        doc.descendants()
            .filter(|n| n.is_element() && n.tag_name().name() == name)
            .inspect(|n| assert_eq!(n.tag_name().namespace(), Some(NAMESPACE)))
            .collect()
    }

    fn one<'a>(doc: &'a Document<'a>, name: &str) -> Node<'a, 'a> {
        let nodes = all(doc, name);
        assert_eq!(nodes.len(), 1, "{name}");
        nodes[0]
    }

    fn by_id<'a>(doc: &'a Document<'a>, name: &str, id: &str) -> Node<'a, 'a> {
        all(doc, name)
            .into_iter()
            .find(|n| n.attribute("Id") == Some(id))
            .unwrap_or_else(|| panic!("{name} {id}"))
    }

    fn component_of<'a>(node: Node<'a, 'a>) -> Node<'a, 'a> {
        node.ancestors()
            .find(|n| n.tag_name().name() == "Component")
            .expect("in a component")
    }

    #[test]
    fn the_package_is_per_user_with_the_spec_s_identity_and_no_product_code() {
        let text = source();
        let doc = Document::parse(&text).expect("xml");
        assert_eq!(doc.root_element().tag_name().name(), "Wix");
        let package = one(&doc, "Package");
        assert_eq!(package.attribute("Scope"), Some("perUser"));
        assert_eq!(package.attribute("Name"), Some(spec().name.as_str()));
        assert_eq!(package.attribute("Manufacturer"), Some(MANUFACTURER));
        assert_eq!(
            package.attribute("Version"),
            Some(msi_version(&spec().version).expect("version").as_str())
        );
        assert_eq!(package.attribute("UpgradeCode"), Some(UPGRADE_CODE));
        assert_eq!(package.attribute("ProductCode"), None);
        assert_eq!(package.attribute("InstallScope"), None);
    }

    #[test]
    fn a_same_version_build_upgrades_and_an_older_one_is_refused() {
        let text = source();
        let doc = Document::parse(&text).expect("xml");
        let upgrade = one(&doc, "MajorUpgrade");
        assert_eq!(upgrade.attribute("AllowSameVersionUpgrades"), Some("yes"));
        assert!(
            upgrade
                .attribute("DowngradeErrorMessage")
                .is_some_and(|m| !m.is_empty())
        );
    }

    #[test]
    fn the_cabinet_is_embedded() {
        let text = source();
        let doc = Document::parse(&text).expect("xml");
        assert_eq!(
            one(&doc, "MediaTemplate").attribute("EmbedCab"),
            Some("yes")
        );
    }

    #[test]
    fn installs_into_local_app_data_programs_nova() {
        let text = source();
        let doc = Document::parse(&text).expect("xml");
        let install = by_id(&doc, "Directory", INSTALL_FOLDER);
        assert_eq!(install.attribute("Name"), Some(INSTALL_DIR));
        let programs = install.parent_element().expect("parent");
        assert_eq!(programs.tag_name().name(), "Directory");
        assert_eq!(programs.attribute("Name"), Some("Programs"));
        let root = programs.parent_element().expect("root");
        assert_eq!(root.tag_name().name(), "StandardDirectory");
        assert_eq!(root.attribute("Id"), Some("LocalAppDataFolder"));
    }

    #[test]
    fn packs_the_executables_from_the_exes_dir_and_the_readme_and_licence() {
        let spec = spec();
        let exes = Path::new("/signed").join("exes");
        let layout =
            InstallerLayout::new(&spec.target_dir, None, Some(exes.clone())).expect("layout");
        let text = wxs(&spec, &layout).expect("wxs");
        let doc = Document::parse(&text).expect("xml");
        let sources: Vec<PathBuf> = all(&doc, "File")
            .iter()
            .map(|f| PathBuf::from(f.attribute("Source").expect("source")))
            .collect();
        // Text built from the same bases the generator joins onto.
        let expected = [
            exes.join("nova.exe"),
            exes.join("nova-dump.exe"),
            spec.workspace_root.join("packaging").join("README.txt"),
            spec.workspace_root.join("LICENSE"),
        ];
        assert_eq!(
            sources
                .iter()
                .map(|p| p.display().to_string())
                .collect::<Vec<_>>(),
            expected
                .iter()
                .map(|p| p.display().to_string())
                .collect::<Vec<_>>()
        );
        for file in all(&doc, "File") {
            assert_eq!(
                component_of(file).attribute("Directory"),
                Some(INSTALL_FOLDER)
            );
        }
    }

    #[test]
    fn the_start_menu_shortcut_starts_nova_exe_in_the_install_folder() {
        let text = source();
        let doc = Document::parse(&text).expect("xml");
        let shortcut = one(&doc, "Shortcut");
        assert_eq!(shortcut.attribute("Name"), Some(SHORTCUT));
        assert_eq!(
            shortcut.attribute("Target"),
            Some("[INSTALLFOLDER]nova.exe")
        );
        assert_eq!(shortcut.attribute("Arguments"), None);
        assert_eq!(shortcut.attribute("WorkingDirectory"), Some(INSTALL_FOLDER));
        assert_eq!(shortcut.attribute("Icon"), Some(ICON_ID));
        assert_eq!(
            component_of(shortcut).attribute("Directory"),
            Some("ProgramMenuFolder")
        );
        assert!(
            all(&doc, "StandardDirectory")
                .iter()
                .any(|d| d.attribute("Id") == Some("ProgramMenuFolder"))
        );
    }

    #[test]
    fn apps_and_features_shows_the_icon_and_offers_no_modify() {
        let spec = spec();
        let layout = layout(&spec);
        let text = wxs(&spec, &layout).expect("wxs");
        let doc = Document::parse(&text).expect("xml");
        let icon = by_id(&doc, "Icon", ICON_ID);
        assert_eq!(
            icon.attribute("SourceFile"),
            Some(layout.icon().display().to_string().as_str())
        );
        assert_eq!(
            by_id(&doc, "Property", "ARPPRODUCTICON").attribute("Value"),
            Some(ICON_ID)
        );
        assert_eq!(
            by_id(&doc, "Property", "ARPNOMODIFY").attribute("Value"),
            Some("1")
        );
    }

    #[test]
    fn every_component_s_key_path_is_a_per_user_registry_value() {
        let text = source();
        let doc = Document::parse(&text).expect("xml");
        let components = all(&doc, "Component");
        assert_eq!(components.len(), 5);
        for component in components {
            let key_paths: Vec<_> = component
                .descendants()
                .filter(|n| n.attribute("KeyPath") == Some("yes"))
                .collect();
            assert_eq!(key_paths.len(), 1, "{:?}", component.attribute("Id"));
            let key_path = key_paths[0];
            assert_eq!(key_path.tag_name().name(), "RegistryValue");
            assert_eq!(key_path.attribute("Root"), Some("HKCU"));
            assert_eq!(key_path.attribute("Key"), Some(REGISTRY_KEY));
        }
        let names: Vec<_> = all(&doc, "RegistryValue")
            .iter()
            .map(|v| v.attribute("Name").expect("name"))
            .collect();
        let mut unique = names.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(unique.len(), names.len(), "{names:?}");
    }

    #[test]
    fn every_folder_the_package_makes_is_removed_on_uninstall() {
        let text = source();
        let doc = Document::parse(&text).expect("xml");
        let made: Vec<_> = all(&doc, "Directory")
            .iter()
            .map(|d| d.attribute("Id").expect("id"))
            .collect();
        assert_eq!(made, [PROGRAMS_FOLDER, INSTALL_FOLDER]);
        for id in made {
            let removal = all(&doc, "RemoveFolder")
                .into_iter()
                .find(|r| r.attribute("Directory") == Some(id))
                .unwrap_or_else(|| panic!("no RemoveFolder for {id}"));
            assert_eq!(removal.attribute("On"), Some("uninstall"));
        }
    }

    #[test]
    fn nothing_refers_to_the_roaming_app_data_folder() {
        let text = source();
        let doc = Document::parse(&text).expect("xml");
        for node in doc.descendants().filter(Node::is_element) {
            for attribute in node.attributes() {
                assert!(
                    !attribute.value().contains("AppDataFolder")
                        || attribute.value() == "LocalAppDataFolder",
                    "{}={}",
                    attribute.name(),
                    attribute.value()
                );
            }
        }
    }

    #[test]
    fn markup_in_the_name_is_escaped() {
        let mut spec = spec();
        spec.name = r#"EV & <Nova> "it's""#.into();
        let text = wxs(&spec, &layout(&spec)).expect("wxs");
        let doc = Document::parse(&text).expect("xml");
        assert_eq!(
            one(&doc, "Package").attribute("Name"),
            Some(spec.name.as_str())
        );
        assert_eq!(
            one(&doc, "Feature").attribute("Title"),
            Some(spec.name.as_str())
        );
    }

    #[test]
    fn markup_in_a_path_is_escaped() {
        let mut spec = spec();
        spec.workspace_root = Path::new("/work").join("R&D <\"x\">");
        let text = wxs(&spec, &layout(&spec)).expect("wxs");
        let doc = Document::parse(&text).expect("xml");
        let licence = spec.workspace_root.join("LICENSE").display().to_string();
        assert!(
            all(&doc, "File")
                .iter()
                .any(|f| f.attribute("Source") == Some(licence.as_str()))
        );
    }

    #[test]
    fn a_version_an_msi_cannot_hold_is_an_error() {
        let mut spec = spec();
        spec.version = "256.0.0".into();
        assert!(matches!(
            wxs(&spec, &layout(&spec)),
            Err(InstallerError::VersionTooLarge(_))
        ));
    }
}
