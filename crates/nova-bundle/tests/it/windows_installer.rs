//! The packed `.msi` installed, upgraded and uninstalled for real, with
//! `msiexec` and PowerShell (Windows only, opt-in).
//!
//! It skips, passing, unless `NOVA_MSI` names a built `.msi`, so neither
//! `mise run ci` nor a developer's test run installs anything. CI's
//! Windows `build` leg packs the installer and sets it. With
//! `NOVA_MSI_PREVIOUS` also set (a second pack of the same executables:
//! the same version, another `ProductCode`), it installs that one first and
//! checks the current one replaces it in place.
//!
//! Both packs carry WinSparkle: the test checks the installed
//! `WinSparkle.dll` is the one the last pack staged in
//! `target\installer\downloads` (the file `wix build` packed, which the
//! signing phase will sign there), that both packs hold it in the same
//! component, which after the upgrade only the new product holds, and the
//! relaunch custom action's rows. It doesn't run the relaunch: that would
//! start the game on the runner.
//!
//! The Windows Installer's state is read through its COM object
//! (`WindowsInstaller.Installer`) and the shortcut through
//! `WScript.Shell`, each in a small PowerShell script printing one value
//! per line. Whatever happens, a guard uninstalls what the test installed
//! and deletes the settings sentinel it wrote.

use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use nova_bundle::bundle::metadata;
use nova_bundle::fonts::font_licences;
use nova_bundle::installer::layout::InstallerLayout;
use nova_bundle::installer::winsparkle::{DLL, LICENCES};
use nova_bundle::installer::wxs::COMPONENT_GUIDS;
use nova_bundle::installer::{
    DUMP_EXE, EXES, GAME_EXE, INSTALL_DIR, RELAUNCH_ACTION, RELAUNCH_PROPERTY, SHORTCUT,
    UPGRADE_CODE, msi_version,
};
use nova_bundle::ports::{Invocation, Process};
use nova_bundle::spec::BundleSpec;
use nova_bundle::system::StdProcess;

/// Windows Installer's summary information word count property, and its
/// "elevation not required" bit.
const PID_WORDCOUNT: u32 = 15;
const NO_ELEVATION: u32 = 8;
/// `Installer.ProductState` for an installed product.
const INSTALLED: &str = "5";
/// What [`msi_property`] prints for a property the package lacks.
const NONE: &str = "<none>";

const PRELUDE: &str = r"$ErrorActionPreference = 'Stop'
$installer = New-Object -ComObject WindowsInstaller.Installer
function Get-Prop($o, $name, $a) { $o.GetType().InvokeMember($name, 'GetProperty', $null, $o, $a) }
function Invoke-Com($o, $name, $a) { $o.GetType().InvokeMember($name, 'InvokeMethod', $null, $o, $a) }
";

/// `text` as a single-quoted PowerShell string.
fn quote(text: &str) -> String {
    format!("'{}'", text.replace('\'', "''"))
}

fn quote_path(path: &Path) -> String {
    quote(&path.display().to_string())
}

/// Runs `script` after [`PRELUDE`], returning its output's lines.
fn powershell(script: &str) -> Vec<String> {
    let invocation = Invocation::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command"])
        .arg(format!("{PRELUDE}{script}"))
        .captured();
    let stdout = StdProcess
        .run(&invocation)
        .unwrap_or_else(|e| panic!("{script}: {e}"));
    String::from_utf8_lossy(&stdout)
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(str::to_owned)
        .collect()
}

fn one(lines: Vec<String>) -> String {
    assert_eq!(lines.len(), 1, "{lines:?}");
    lines.into_iter().next().expect("one line")
}

/// A property of the package's `Property` table, or [`NONE`].
fn msi_property(msi: &Path, name: &str) -> String {
    let query = format!("SELECT `Value` FROM `Property` WHERE `Property` = '{name}'");
    one(powershell(&format!(
        "$db = Invoke-Com $installer 'OpenDatabase' @({msi}, 0)
$view = Invoke-Com $db 'OpenView' @({query})
Invoke-Com $view 'Execute' $null | Out-Null
$record = Invoke-Com $view 'Fetch' $null
if ($null -eq $record) {{ '{NONE}' }} else {{ Get-Prop $record 'StringData' @(1) }}",
        msi = quote_path(msi),
        query = quote(&query),
    )))
}

/// The first row `query` selects from the package's tables, the text of
/// each of its `columns` columns in order, or `None` when it selects none.
fn msi_row(msi: &Path, query: &str, columns: usize) -> Option<Vec<String>> {
    // Each field by a literal index: a pipeline's `$_` reaches COM wrapped,
    // which `InvokeMember` refuses as a type mismatch.
    let fields: Vec<String> = (1..=columns)
        .map(|i| format!("  'field=' + (Get-Prop $record 'StringData' @({i}))"))
        .collect();
    let lines = powershell(&format!(
        "$db = Invoke-Com $installer 'OpenDatabase' @({msi}, 0)
$view = Invoke-Com $db 'OpenView' @({query})
Invoke-Com $view 'Execute' $null | Out-Null
$record = Invoke-Com $view 'Fetch' $null
if ($null -eq $record) {{ '{NONE}' }} else {{
{fields}
}}",
        msi = quote_path(msi),
        query = quote(query),
        fields = fields.join("\n"),
    ));
    if lines == [NONE] {
        return None;
    }
    Some(
        lines
            .iter()
            .map(|line| {
                line.strip_prefix("field=")
                    .unwrap_or_else(|| panic!("{query}: {lines:?}"))
                    .to_owned()
            })
            .collect(),
    )
}

/// `msidbCustomActionTypeExe` with `msidbCustomActionTypeSourceFile`
/// (0x12, 18: an executable the package installs), run asynchronously
/// without waiting (`msidbCustomActionTypeAsync |
/// msidbCustomActionTypeContinue`, 0xC0): WiX's `FileRef`, `ExeCommand`
/// and `Return="asyncNoWait"`.
const EXE_IN_FILE_NO_WAIT: u32 = 0x12 | 0xC0;

/// The relaunch after an update, as the built package holds it: a custom
/// action that starts the installed `nova.exe` without arguments, without
/// waiting, after `InstallFinalize`, when the feed's arguments set
/// [`RELAUNCH_PROPERTY`] and the run isn't an uninstall.
fn check_relaunch(msi: &Path) {
    let action = msi_row(
        msi,
        &format!(
            "SELECT `Type`, `Source`, `Target` FROM `CustomAction` WHERE `Action` = \
             '{RELAUNCH_ACTION}'"
        ),
        3,
    )
    .expect("the relaunch action");
    assert_eq!(
        action,
        [
            EXE_IN_FILE_NO_WAIT.to_string(),
            GAME_EXE.to_owned(),
            String::new()
        ]
    );
    let sequence = |action: &str| {
        msi_row(
            msi,
            &format!(
                "SELECT `Sequence`, `Condition` FROM `InstallExecuteSequence` WHERE `Action` = \
                 '{action}'"
            ),
            2,
        )
        .unwrap_or_else(|| panic!("{action} is sequenced"))
    };
    let relaunch = sequence(RELAUNCH_ACTION);
    let finalize = sequence("InstallFinalize");
    let number = |row: &[String]| row[0].parse::<u32>().expect("a sequence number");
    assert!(
        number(&relaunch) > number(&finalize),
        "{relaunch:?} {finalize:?}"
    );
    assert_eq!(
        relaunch[1],
        format!("{RELAUNCH_PROPERTY} = \"1\" AND NOT REMOVE")
    );
    let secure = msi_property(msi, "SecureCustomProperties");
    assert!(
        secure.split(';').any(|p| p == RELAUNCH_PROPERTY),
        "{RELAUNCH_PROPERTY} reaches the server side: {secure}"
    );
}

fn word_count(msi: &Path) -> u32 {
    one(powershell(&format!(
        "$info = Get-Prop $installer 'SummaryInformation' @({msi}, 0)
Get-Prop $info 'Property' @({PID_WORDCOUNT})",
        msi = quote_path(msi),
    )))
    .parse()
    .expect("a number")
}

fn product_state(code: &str) -> String {
    one(powershell(&format!(
        "Get-Prop $installer 'ProductState' @({})",
        quote(code)
    )))
}

fn product_info(code: &str, attribute: &str) -> String {
    one(powershell(&format!(
        "Get-Prop $installer 'ProductInfo' @({}, {})",
        quote(code),
        quote(attribute)
    )))
}

/// The product's Apps & features icon: a path Windows Installer reports
/// with environment variables (`%APPDATA%\Microsoft\Installer\…`), expanded.
fn product_icon(code: &str) -> PathBuf {
    PathBuf::from(one(powershell(&format!(
        "[Environment]::ExpandEnvironmentVariables((Get-Prop $installer 'ProductInfo' @({}, 'ProductIcon')))",
        quote(code)
    ))))
}

/// The products Windows Installer has the component `guid` installed for.
fn component_clients(guid: &str) -> Vec<String> {
    powershell(&format!(
        "Get-Prop $installer 'ComponentClients' @({}) | ForEach-Object {{ [string]$_ }}",
        quote(guid)
    ))
}

/// The `ComponentId` of the component the package installs `WinSparkle.dll`
/// with, checking the `File` row names it. WiX gives the file and its
/// component the file's name as their ids.
fn dll_component(msi: &Path) -> String {
    let row = msi_row(
        msi,
        &format!(
            "SELECT `Component`.`ComponentId`, `File`.`FileName` FROM `File`, `Component` \
             WHERE `File`.`Component_` = `Component`.`Component` AND `File`.`File` = '{DLL}'"
        ),
        2,
    )
    .unwrap_or_else(|| panic!("{} packs no {DLL}", msi.display()));
    // `short|long`, or the long name alone when it is already 8.3.
    assert_eq!(row[1].rsplit('|').next(), Some(DLL), "{row:?}");
    row[0].clone()
}

/// [`DLL`]'s component's GUID, as the `Component` table holds it.
fn dll_guid() -> String {
    let (_, guid) = COMPONENT_GUIDS
        .iter()
        .find(|(id, _)| *id == DLL)
        .expect("the DLL's component");
    format!("{{{guid}}}")
}

fn related_products() -> Vec<String> {
    powershell(&format!(
        // PowerShell enumerates the returned StringList: one line per item.
        "Get-Prop $installer 'RelatedProducts' @({}) | ForEach-Object {{ [string]$_ }}",
        quote(&format!("{{{UPGRADE_CODE}}}"))
    ))
}

/// The user's Start menu `Programs` folder.
fn start_menu_programs() -> PathBuf {
    PathBuf::from(one(powershell("[Environment]::GetFolderPath('Programs')")))
}

/// The shortcut's target and its arguments.
fn shortcut(lnk: &Path) -> (String, String) {
    let lines = powershell(&format!(
        "$link = (New-Object -ComObject WScript.Shell).CreateShortcut({})
'target=' + $link.TargetPath
'arguments=' + $link.Arguments",
        quote_path(lnk)
    ));
    let field = |name: &str| {
        lines
            .iter()
            .find_map(|l| l.strip_prefix(name))
            .unwrap_or_else(|| panic!("{name}: {lines:?}"))
            .to_owned()
    };
    (field("target="), field("arguments="))
}

/// A log `msiexec` wrote: UTF-16 with a byte-order mark, or else 8-bit.
fn log_text(bytes: &[u8]) -> String {
    match bytes {
        [0xFF, 0xFE, rest @ ..] => String::from_utf16_lossy(
            &rest
                .as_chunks::<2>()
                .0
                .iter()
                .map(|c| u16::from_le_bytes(*c))
                .collect::<Vec<_>>(),
        ),
        _ => String::from_utf8_lossy(bytes).into_owned(),
    }
}

/// Runs `msiexec` with `args`, quietly and logging verbosely to `log`,
/// panicking with the log when it fails.
fn msiexec(args: &[OsString], log: &Path) {
    let invocation = Invocation::new("msiexec")
        .args(args)
        .args(["/qn", "/norestart", "/l*v"])
        .arg(log);
    if let Err(e) = StdProcess.run(&invocation) {
        let text = fs::read(log).map(|b| log_text(&b)).unwrap_or_default();
        panic!("`{invocation}` failed: {e}\n{text}");
    }
}

fn install(msi: &Path, log: &Path) {
    msiexec(&["/i".into(), msi.into()], log);
}

fn uninstall(code: &str, log: &Path) {
    msiexec(&["/x".into(), code.into()], log);
}

fn same_file(a: &Path, b: &Path) -> bool {
    fs::canonicalize(a).is_ok_and(|a| fs::canonicalize(b).ok() == Some(a))
}

/// Uninstalls every product the test installed that is still installed,
/// and deletes its settings sentinel (and `%APPDATA%\nova`, if the test
/// made it), even when an assertion failed.
struct Guard {
    codes: Vec<String>,
    log_dir: PathBuf,
    sentinel: PathBuf,
    made_settings_dir: bool,
}

impl Drop for Guard {
    fn drop(&mut self) {
        for code in &self.codes {
            if product_state(code) == INSTALLED {
                let invocation = Invocation::new("msiexec")
                    .args(["/x", code, "/qn", "/norestart"])
                    .arg("/l*v")
                    .arg(self.log_dir.join("cleanup.log"));
                if let Err(e) = StdProcess.run(&invocation) {
                    eprintln!("cleanup: `{invocation}` failed: {e}");
                }
            }
        }
        let _ = fs::remove_file(&self.sentinel);
        if self.made_settings_dir
            && let Some(dir) = self.sentinel.parent()
        {
            let _ = fs::remove_dir(dir);
        }
    }
}

fn var_path(name: &str) -> Option<PathBuf> {
    env::var_os(name)
        .filter(|v| !v.is_empty())
        .map(|v| std::path::absolute(PathBuf::from(v)).expect("an absolute path"))
}

/// The spec, read from `cargo metadata` as the packer reads it.
fn spec() -> BundleSpec {
    let json = StdProcess
        .run(&metadata(&OsString::from(env!("CARGO"))))
        .expect("cargo metadata");
    BundleSpec::from_metadata(&json).expect("spec")
}

/// The installer's version, read through the spec.
fn expected_version() -> String {
    msi_version(&spec().version).expect("version")
}

/// The `WinSparkle.dll` the last pack staged and `wix build` packed: the
/// pinned DLL, or, once the signing phase signs it there, that signed DLL.
fn staged_dll() -> PathBuf {
    InstallerLayout::new(&spec().target_dir, None, None)
        .expect("layout")
        .winsparkle_dll()
}

#[test]
fn installs_for_the_user_alone_upgrades_in_place_and_uninstalls_keeping_settings() {
    let Some(msi) = var_path("NOVA_MSI") else {
        eprintln!("NOVA_MSI is unset: skipping the installer test");
        return;
    };
    let previous = var_path("NOVA_MSI_PREVIOUS");

    // No admin needed: per-user, with no ALLUSERS to make it per-machine.
    assert_ne!(word_count(&msi) & NO_ELEVATION, 0, "elevation not required");
    assert_eq!(msi_property(&msi, "ALLUSERS"), NONE);
    check_relaunch(&msi);
    assert_eq!(dll_component(&msi), dll_guid());
    // The upgrade replaces the DLL with the DLL, in the same component.
    if let Some(previous) = &previous {
        assert_eq!(dll_component(previous), dll_guid(), "the previous pack");
    }
    let code = msi_property(&msi, "ProductCode");
    let previous_code = previous.as_ref().map(|p| msi_property(p, "ProductCode"));
    if let Some(previous_code) = &previous_code {
        assert_ne!(previous_code, &code, "the two packs share a ProductCode");
    }

    let logs = tempfile::tempdir().expect("temp dir");
    let settings = var_path("APPDATA").expect("APPDATA").join("nova");
    let made_settings_dir = !settings.exists();
    fs::create_dir_all(&settings).expect("settings dir");
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let sentinel = settings.join(format!(
        "installer-test-sentinel-{}-{nanos}",
        std::process::id()
    ));
    fs::write(&sentinel, b"kept").expect("sentinel");
    let _guard = Guard {
        codes: previous_code
            .iter()
            .cloned()
            .chain([code.clone()])
            .collect(),
        log_dir: logs.path().to_owned(),
        sentinel: sentinel.clone(),
        made_settings_dir,
    };

    if let Some(previous) = &previous {
        install(previous, &logs.path().join("previous.log"));
    }
    install(&msi, &logs.path().join("install.log"));

    // Installed for this user, not the machine.
    assert_eq!(product_state(&code), INSTALLED);
    assert_eq!(product_info(&code, "AssignmentType"), "0", "per-user");

    // An upgrade in place: one product of the UpgradeCode, the new one.
    if let Some(previous_code) = &previous_code {
        assert_eq!(related_products(), std::slice::from_ref(&code));
        assert_ne!(product_state(previous_code), INSTALLED);
    }

    let folder = var_path("LOCALAPPDATA")
        .expect("LOCALAPPDATA")
        .join("Programs")
        .join(INSTALL_DIR);
    let licences = LICENCES.map(|(_, installed)| installed);
    let fonts = font_licences(Path::new("")).map(|(_, name)| name);
    for file in EXES
        .iter()
        .chain(&["README.txt", "LICENSE"])
        .chain(&fonts)
        .chain(&[DLL])
        .chain(&licences)
    {
        assert!(folder.join(file).is_file(), "{file}");
    }
    // The DLL wix packed, byte for byte, its component the new product's
    // alone after the upgrade.
    assert_eq!(
        fs::read(folder.join(DLL)).expect("the installed DLL"),
        fs::read(staged_dll()).expect("the staged DLL"),
        "the installed {DLL} is the one packed"
    );
    assert_eq!(
        component_clients(&dll_guid()),
        std::slice::from_ref(&code),
        "{DLL}'s component is installed for the new product alone"
    );
    StdProcess
        .run(
            &Invocation::new(folder.join(DUMP_EXE))
                .arg("--help")
                .captured(),
        )
        .expect("the installed nova-dump.exe --help");

    let lnk = start_menu_programs().join(format!("{SHORTCUT}.lnk"));
    assert!(lnk.is_file(), "{}", lnk.display());
    let (target, arguments) = shortcut(&lnk);
    assert!(
        same_file(Path::new(&target), &folder.join(GAME_EXE)),
        "{target}"
    );
    assert_eq!(arguments, "");

    // Apps & features: the version and an icon.
    assert_eq!(product_info(&code, "VersionString"), expected_version());
    let icon = product_icon(&code);
    assert!(icon.is_file(), "ProductIcon {}", icon.display());

    uninstall(&code, &logs.path().join("uninstall.log"));
    assert_ne!(product_state(&code), INSTALLED);
    assert!(related_products().is_empty());
    assert!(!folder.exists(), "{}", folder.display());
    assert!(!lnk.exists(), "{}", lnk.display());
    assert_eq!(fs::read(&sentinel).expect("settings kept"), b"kept");
}

#[test]
fn quotes_powershell_strings() {
    assert_eq!(quote("it's"), "'it''s'");
}

#[test]
fn reads_a_utf16_or_8_bit_log() {
    let utf16: Vec<u8> = [0xFF, 0xFE]
        .into_iter()
        .chain("ok".encode_utf16().flat_map(u16::to_le_bytes))
        .collect();
    assert_eq!(log_text(&utf16), "ok");
    assert_eq!(log_text(b"ok"), "ok");
}
