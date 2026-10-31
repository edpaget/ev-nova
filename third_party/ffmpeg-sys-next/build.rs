extern crate bindgen;
extern crate cc;
extern crate num_cpus;

use std::env;
use std::fmt::Write as FmtWrite;
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::str;

use sha2::{Digest, Sha256};

use bindgen::callbacks::{
    EnumVariantCustomBehavior, EnumVariantValue, IntKind, MacroParsingBehavior, ParseCallbacks,
};

#[derive(Debug)]
struct Library {
    name: &'static str,
    is_feature: bool,
}

impl Library {
    fn feature_name(&self) -> Option<String> {
        if self.is_feature {
            Some("CARGO_FEATURE_".to_string() + &self.name.to_uppercase())
        } else {
            None
        }
    }
}

static LIBRARIES: &[Library] = &[
    Library {
        name: "avcodec",
        is_feature: true,
    },
    Library {
        name: "avdevice",
        is_feature: true,
    },
    Library {
        name: "avfilter",
        is_feature: true,
    },
    Library {
        name: "avformat",
        is_feature: true,
    },
    Library {
        name: "avresample",
        is_feature: true,
    },
    Library {
        name: "avutil",
        is_feature: false,
    },
    Library {
        name: "postproc",
        is_feature: true,
    },
    Library {
        name: "swresample",
        is_feature: true,
    },
    Library {
        name: "swscale",
        is_feature: true,
    },
];

#[derive(Debug)]
struct Callbacks;

impl ParseCallbacks for Callbacks {
    fn int_macro(&self, _name: &str, value: i64) -> Option<IntKind> {
        let ch_layout_prefix = "AV_CH_";
        let codec_cap_prefix = "AV_CODEC_CAP_";
        let codec_flag_prefix = "AV_CODEC_FLAG_";
        let error_max_size = "AV_ERROR_MAX_STRING_SIZE";

        if _name.starts_with(ch_layout_prefix) {
            Some(IntKind::ULongLong)
        } else if value >= i32::MIN as i64
            && value <= i32::MAX as i64
            && (_name.starts_with(codec_cap_prefix) || _name.starts_with(codec_flag_prefix))
        {
            Some(IntKind::UInt)
        } else if _name == error_max_size {
            Some(IntKind::Custom {
                name: "usize",
                is_signed: false,
            })
        } else if value >= i32::MIN as i64 && value <= i32::MAX as i64 {
            Some(IntKind::Int)
        } else {
            None
        }
    }

    fn enum_variant_behavior(
        &self,
        _enum_name: Option<&str>,
        original_variant_name: &str,
        _variant_value: EnumVariantValue,
    ) -> Option<EnumVariantCustomBehavior> {
        let dummy_codec_id_prefix = "AV_CODEC_ID_FIRST_";
        if original_variant_name.starts_with(dummy_codec_id_prefix) {
            Some(EnumVariantCustomBehavior::Constify)
        } else {
            None
        }
    }

    // https://github.com/rust-lang/rust-bindgen/issues/687#issuecomment-388277405
    fn will_parse_macro(&self, name: &str) -> MacroParsingBehavior {
        use MacroParsingBehavior::*;

        match name {
            "FP_INFINITE" => Ignore,
            "FP_NAN" => Ignore,
            "FP_NORMAL" => Ignore,
            "FP_SUBNORMAL" => Ignore,
            "FP_ZERO" => Ignore,
            _ => Default,
        }
    }
}

/// The crate's FFmpeg series, `major.minor`, which the source record's
/// version must belong to.
fn version() -> String {
    let major: u8 = env::var("CARGO_PKG_VERSION_MAJOR")
        .unwrap()
        .parse()
        .unwrap();
    let minor: u8 = env::var("CARGO_PKG_VERSION_MINOR")
        .unwrap()
        .parse()
        .unwrap();

    format!("{major}.{minor}")
}

fn output() -> PathBuf {
    PathBuf::from(env::var("OUT_DIR").unwrap())
}

/// Where the source record's tarball is unpacked.
fn source() -> PathBuf {
    output().join(format!("ffmpeg-{}", SourceRecord::read().version))
}

fn search() -> PathBuf {
    let mut absolute = env::current_dir().unwrap();
    absolute.push(output());
    absolute.push("dist");

    absolute
}

/// The FFmpeg source this copy builds, from `../ffmpeg-source.toml`: one
/// release tarball, pinned by its SHA-256.
struct SourceRecord {
    version: String,
    url: String,
    sha256: String,
}

impl SourceRecord {
    fn path() -> PathBuf {
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap())
            .join("..")
            .join("ffmpeg-source.toml")
    }

    /// Reads the record: `key = "value"` lines, comments and blank lines
    /// skipped.
    fn read() -> Self {
        let path = Self::path();
        let text = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("can't read {}: {e}", path.display()));
        let value = |key: &str| {
            text.lines()
                .filter_map(|line| line.split_once('='))
                .find(|(k, _)| k.trim() == key)
                .map(|(_, v)| v.trim().trim_matches('"').to_string())
                .unwrap_or_else(|| panic!("{} has no {key}", path.display()))
        };
        let record = Self {
            version: value("version"),
            url: value("url"),
            sha256: value("sha256").to_ascii_lowercase(),
        };
        let series = format!("{}.", version());
        assert!(
            record.version.starts_with(&series),
            "{} names FFmpeg {}, but this crate binds FFmpeg {}",
            path.display(),
            record.version,
            version()
        );
        record
    }

    /// The tarball's file name, the last part of its URL.
    fn file_name(&self) -> &str {
        self.url.rsplit('/').next().unwrap()
    }
}

/// The SHA-256 of the file at `path`, as lowercase hex.
fn sha256_of(path: &Path) -> io::Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    io::copy(&mut file, &mut hasher)?;
    Ok(hasher
        .finalize()
        .iter()
        .fold(String::new(), |mut hex, byte| {
            let _ = write!(hex, "{byte:02x}");
            hex
        }))
}

/// Where downloaded tarballs are kept: `$CARGO_HOME/nova-ffmpeg`, so a
/// machine downloads each one once, whatever its target directories.
fn tarball_cache() -> PathBuf {
    let cargo_home = env::var_os("CARGO_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            env::var_os("HOME")
                .or_else(|| env::var_os("USERPROFILE"))
                .map(|home| PathBuf::from(home).join(".cargo"))
        })
        .unwrap_or_else(output);
    cargo_home.join("nova-ffmpeg")
}

/// The record's tarball, checked against its SHA-256: `NOVA_FFMPEG_TARBALL`
/// when it is set (an offline build, such as the Flatpak's), else the
/// cached copy, downloaded with `curl` on a miss.
fn tarball(record: &SourceRecord) -> io::Result<PathBuf> {
    println!("cargo:rerun-if-env-changed=NOVA_FFMPEG_TARBALL");
    let path = if let Some(given) = env::var_os("NOVA_FFMPEG_TARBALL") {
        PathBuf::from(given)
    } else {
        let cache = tarball_cache();
        fs::create_dir_all(&cache)?;
        let cached = cache.join(record.file_name());
        let fresh = sha256_of(&cached).is_ok_and(|sum| sum == record.sha256);
        if !fresh {
            let part = cache.join(format!("{}.{}.part", record.file_name(), std::process::id()));
            let status = Command::new("curl")
                .args(["-fsSL", "--retry", "3", "-o"])
                .arg(&part)
                .arg(&record.url)
                .status()?;
            if !status.success() {
                let _ = fs::remove_file(&part);
                return Err(io::Error::other(format!("curl couldn't download {}", record.url)));
            }
            fs::rename(&part, &cached)?;
        }
        cached
    };
    let sum = sha256_of(&path)?;
    if sum != record.sha256 {
        return Err(io::Error::other(format!(
            "{} has SHA-256 {sum}, not the {} that {} records",
            path.display(),
            record.sha256,
            SourceRecord::path().display()
        )));
    }
    Ok(path)
}

/// Unpacks the record's tarball into `OUT_DIR`, replacing any earlier
/// copy.
fn fetch(record: &SourceRecord) -> io::Result<()> {
    let tarball = tarball(record)?;
    let _ = fs::remove_dir_all(source());
    let mut tar = if cfg!(windows) {
        // MSYS2's tar, which can read .xz through MSYS2's xz; it needs
        // --force-local, or it reads `C:` as a remote host.
        let msys = msys_bin()?;
        let mut tar = Command::new(msys.join("tar.exe"));
        tar.env("PATH", prepend_path(&[msys]));
        tar.arg("--force-local");
        tar
    } else {
        Command::new("tar")
    };
    let status = tar
        .arg("-xJf")
        .arg(posix(&tarball))
        .arg("-C")
        .arg(posix(&output()))
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other(format!("tar couldn't unpack {}", tarball.display())))
    }
}

/// `path` with `/` for every separator: what MSYS2's tools and FFmpeg's
/// configure read on Windows. Elsewhere it is unchanged.
fn posix(path: &Path) -> String {
    let text = path.to_string_lossy().into_owned();
    if cfg!(windows) {
        text.replace('\\', "/")
    } else {
        text
    }
}

/// `PATH` with `dirs` ahead of it.
fn prepend_path(dirs: &[PathBuf]) -> std::ffi::OsString {
    let path = env::var_os("PATH").unwrap_or_default();
    env::join_paths(dirs.iter().cloned().chain(env::split_paths(&path))).unwrap()
}

/// MSYS2's `usr/bin` (under `MSYS2_ROOT`, else `C:\msys64`), which holds
/// the `sh`, `make` and the POSIX tools FFmpeg's build runs with on
/// Windows.
fn msys_bin() -> io::Result<PathBuf> {
    println!("cargo:rerun-if-env-changed=MSYS2_ROOT");
    let root = env::var_os("MSYS2_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\msys64"));
    let bin = root.join("usr").join("bin");
    for tool in ["sh.exe", "make.exe"] {
        if !bin.join(tool).is_file() {
            return Err(io::Error::other(format!(
                "FFmpeg is built with MSYS2's sh and make, and {} has no {tool}: install MSYS2 \
                 (and `pacman -S make diffutils`), or set MSYS2_ROOT to it",
                bin.display()
            )));
        }
    }
    Ok(bin)
}

/// The configure line: the stripped-down, LGPL-only build that EV Nova's
/// movies need, and nothing else. `--disable-everything` turns every
/// component off; the mov demuxer, the file protocol and the decoders the
/// stock movies (H.264) and the QuickTime codecs plug-ins may use come back
/// on. mov marks H.264 streams for header parsing, hence the h264 parser.
/// No `--enable-gpl`: every component is LGPL. `--disable-x86asm` means no
/// nasm on any machine; arm64 keeps its NEON assembly, which the C compiler
/// assembles. No `-march=native`: the build is shipped.
fn configure_args() -> Vec<String> {
    let mut args: Vec<String> = [
        "--disable-everything",
        "--disable-autodetect",
        "--disable-programs",
        "--disable-doc",
        "--disable-network",
        "--disable-hwaccels",
        "--disable-avdevice",
        "--disable-avfilter",
        "--disable-swresample",
        "--enable-static",
        "--disable-shared",
        "--enable-pic",
        "--enable-avcodec",
        "--enable-avformat",
        "--enable-swscale",
        "--enable-demuxer=mov",
        "--enable-protocol=file",
        "--enable-parser=h264",
        "--enable-decoder=h264,cinepak,svq1,svq3,qtrle,rpza,smc",
        "--disable-x86asm",
        // Skip the warnings: thousands of them slow the build.
        "--extra-cflags=-w",
    ]
    .iter()
    .map(|arg| (*arg).to_string())
    .collect();
    if cfg!(target_env = "msvc") {
        args.push("--toolchain=msvc".to_string());
        // The dynamic C runtime, as Rust's MSVC targets link.
        args.push("--extra-cflags=-MD".to_string());
    } else {
        args.push("--enable-pthreads".to_string());
    }
    args
}

/// What a finished build in `dist/` was built from: the tarball's SHA-256
/// and the configure line. A build from anything else is redone.
fn stamp(record: &SourceRecord) -> String {
    format!("{}\n{}\n", record.sha256, configure_args().join("\n"))
}

fn stamp_path() -> PathBuf {
    search().join("nova-ffmpeg-build.stamp")
}

/// Whether `dist/` already holds this build.
fn built(record: &SourceRecord) -> bool {
    fs::read_to_string(stamp_path()).is_ok_and(|text| text == stamp(record))
}

/// The environment FFmpeg's configure and make run in. On Windows: MSVC's
/// own (`cl.exe`'s `PATH`, `INCLUDE`, `LIB` and `LIBPATH`, as
/// `cc::windows_registry` finds them), with MSVC's directories ahead of
/// MSYS2's `usr/bin` on `PATH`, so MSYS2's `link` never shadows
/// `link.exe`. Elsewhere, nothing changes.
fn build_env() -> io::Result<Vec<(std::ffi::OsString, std::ffi::OsString)>> {
    if !cfg!(windows) {
        return Ok(Vec::new());
    }
    let msys = msys_bin()?;
    let target = env::var("TARGET").unwrap();
    let mut vars = Vec::new();
    let mut path = None;
    if cfg!(target_env = "msvc") {
        let cl = cc::windows_registry::find_tool(&target, "cl.exe")
            .ok_or_else(|| io::Error::other("can't find MSVC's cl.exe for FFmpeg's build"))?;
        for (key, value) in cl.env() {
            if key.eq_ignore_ascii_case("PATH") {
                // MSVC's directories, then the PATH it was found from.
                let prior = env::split_paths(&env::var_os("PATH").unwrap_or_default()).count();
                let all: Vec<PathBuf> = env::split_paths(value).collect();
                let msvc = all.len().saturating_sub(prior);
                let dirs = all[..msvc]
                    .iter()
                    .cloned()
                    .chain([msys.clone()])
                    .chain(all[msvc..].iter().cloned());
                path = Some(env::join_paths(dirs).unwrap());
            } else {
                vars.push((key.clone(), value.clone()));
            }
        }
    }
    vars.push((
        "PATH".into(),
        path.unwrap_or_else(|| prepend_path(&[msys])),
    ));
    Ok(vars)
}

/// A command run with FFmpeg's build environment: `program` itself, or on
/// Windows MSYS2's copy of it, run through its `sh` when it is a script.
fn build_command(program: &str, script: bool) -> io::Result<Command> {
    let mut command = if cfg!(windows) {
        let msys = msys_bin()?;
        if script {
            let mut sh = Command::new(msys.join("sh.exe"));
            sh.arg(program);
            sh
        } else {
            Command::new(msys.join(format!("{program}.exe")))
        }
    } else {
        Command::new(program)
    };
    command.envs(build_env()?);
    command.current_dir(source());
    Ok(command)
}

fn build(record: &SourceRecord) -> io::Result<()> {
    if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        // essential librareis on windowsw
        println!("cargo:rustc-link-lib=dylib=ole32");
        println!("cargo:rustc-link-lib=dylib=oleaut32");
        println!("cargo:rustc-link-lib=dylib=gdi32");
        println!("cargo:rustc-link-lib=dylib=user32");
        println!("cargo:rustc-link-lib=dylib=vfw32");
        println!("cargo:rustc-link-lib=dylib=strmiids");
        println!("cargo:rustc-link-lib=dylib=bcrypt");
        println!("cargo:rustc-link-lib=dylib=shlwapi");
        println!("cargo:rustc-link-lib=dylib=shell32");
    }
    if built(record) {
        return Ok(());
    }

    fs::create_dir_all(output())?;
    fetch(record)?;

    let mut configure = build_command("./configure", true)?;
    configure.arg(format!("--prefix={}", posix(&search())));
    configure.args(configure_args());
    let output = configure
        .output()
        .unwrap_or_else(|_| panic!("{:?} failed", configure));
    if !output.status.success() {
        println!(
            "configure stdout: {}",
            String::from_utf8_lossy(&output.stdout)
        );
        println!(
            "configure stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let log = source().join("ffbuild").join("config.log");
        if let Ok(log) = fs::read_to_string(log) {
            let tail: Vec<&str> = log.lines().rev().take(80).collect();
            println!("config.log (last 80 lines):");
            for line in tail.into_iter().rev() {
                println!("{line}");
            }
        }

        return Err(io::Error::other(format!(
            "configure failed {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }

    // run make
    if !build_command("make", false)?
        .arg("-j")
        .arg(num_cpus::get().to_string())
        .status()?
        .success()
    {
        return Err(io::Error::other("make failed"));
    }

    // run make install
    if !build_command("make", false)?
        .arg("install")
        .status()?
        .success()
    {
        return Err(io::Error::other("make install failed"));
    }

    fs::write(stamp_path(), stamp(record))
}


fn check_features(
    include_paths: Vec<PathBuf>,
    infos: &[(&'static str, Option<&'static str>, &'static str)],
) {
    let mut includes_code = String::new();
    let mut main_code = String::new();

    for &(header, feature, var) in infos {
        if let Some(feature) = feature
            && env::var(format!("CARGO_FEATURE_{}", feature.to_uppercase())).is_err()
        {
            continue;
        }

        let include = format!("#include <{header}>");
        if !includes_code.contains(&include) {
            includes_code.push_str(&include);
            includes_code.push('\n');
        }
        let _ = write!(
            includes_code,
            r#"
            #ifndef {var}_is_defined
            #ifndef {var}
            #define {var} 0
            #define {var}_is_defined 0
            #else
            #define {var}_is_defined 1
            #endif
            #endif
        "#
        );

        let _ = write!(
            main_code,
            r#"printf("[{var}]%d%d\n", {var}, {var}_is_defined);
            "#
        );
    }

    let version_check_info = [("avcodec", 56, 64, 0, 108)];
    for &(lib, begin_version_major, end_version_major, begin_version_minor, end_version_minor) in
        version_check_info.iter()
    {
        for version_major in begin_version_major..end_version_major {
            for version_minor in begin_version_minor..end_version_minor {
                let _ = write!(
                    main_code,
                    r#"printf("[{lib}_version_greater_than_{version_major}_{version_minor}]%d\n", LIB{lib_uppercase}_VERSION_MAJOR > {version_major} || (LIB{lib_uppercase}_VERSION_MAJOR == {version_major} && LIB{lib_uppercase}_VERSION_MINOR > {version_minor}));
                    "#,
                    lib = lib,
                    lib_uppercase = lib.to_uppercase(),
                    version_major = version_major,
                    version_minor = version_minor
                );
            }
        }
    }

    let out_dir = output();

    write!(
        File::create(out_dir.join("check.c")).expect("Failed to create file"),
        r#"
            #include <stdio.h>
            {includes_code}

            int main()
            {{
                {main_code}
                return 0;
            }}
           "#
    )
    .expect("Write failed");

    let executable = out_dir.join(if cfg!(windows) { "check.exe" } else { "check" });
    let mut compiler = cc::Build::new()
        .target(&env::var("HOST").unwrap()) // don't cross-compile this
        .get_compiler()
        .to_command();

    for dir in include_paths {
        compiler.arg("-I");
        compiler.arg(dir.to_string_lossy().into_owned());
    }
    if !compiler
        .current_dir(&out_dir)
        .arg("-o")
        .arg(&executable)
        .arg("check.c")
        .status()
        .expect("Command failed")
        .success()
    {
        panic!("Compile failed");
    }

    let check_output = Command::new(out_dir.join(&executable))
        .current_dir(&out_dir)
        .output()
        .expect("Check failed");
    if !check_output.status.success() {
        panic!(
            "{} failed: {}\n{}",
            executable.display(),
            String::from_utf8_lossy(&check_output.stdout),
            String::from_utf8_lossy(&check_output.stderr)
        );
    }

    let stdout = str::from_utf8(&check_output.stdout).unwrap();

    println!("stdout of {}={}", executable.display(), stdout);

    for &(_, feature, var) in infos {
        if let Some(feature) = feature
            && env::var(format!("CARGO_FEATURE_{}", feature.to_uppercase())).is_err()
        {
            continue;
        }
        // Here so the features are listed for rust-ffmpeg at build time. Does
        // NOT represent activated features, just features that exist (hence the
        // lack of "=true" at the end)
        println!(r#"cargo:{var}="#);

        let var_str = format!("[{var}]");
        let pos = var_str.len()
            + stdout
                .find(&var_str)
                .unwrap_or_else(|| panic!("Variable '{}' not found in stdout output", var_str));
        if &stdout[pos..pos + 1] == "1" {
            println!(r#"cargo:rustc-cfg=feature="{}""#, var.to_lowercase());
            println!(r#"cargo:{}=true"#, var.to_lowercase());
        }

        // Also find out if defined or not (useful for cases where only the definition of a macro
        // can be used as distinction)
        if &stdout[pos + 1..pos + 2] == "1" {
            println!(
                r#"cargo:rustc-cfg=feature="{}_is_defined""#,
                var.to_lowercase()
            );
            println!(r#"cargo:{}_is_defined=true"#, var.to_lowercase());
        }
    }

    for &(lib, begin_version_major, end_version_major, begin_version_minor, end_version_minor) in
        version_check_info.iter()
    {
        for version_major in begin_version_major..end_version_major {
            for version_minor in begin_version_minor..end_version_minor {
                let search_str =
                    format!("[{lib}_version_greater_than_{version_major}_{version_minor}]");
                let pos = stdout
                    .find(&search_str)
                    .expect("Variable not found in output")
                    + search_str.len();

                if &stdout[pos..pos + 1] == "1" {
                    println!(
                        r#"cargo:rustc-cfg=feature="{}""#,
                        &search_str[1..(search_str.len() - 1)]
                    );
                    println!(r#"cargo:{}=true"#, &search_str[1..(search_str.len() - 1)]);
                }
            }
        }
    }

    let ffmpeg_lavc_versions = [
        ("ffmpeg_3_0", 57, 24),
        ("ffmpeg_3_1", 57, 48),
        ("ffmpeg_3_2", 57, 64),
        ("ffmpeg_3_3", 57, 89),
        ("ffmpeg_3_4", 57, 107),
        ("ffmpeg_4_0", 58, 18),
        ("ffmpeg_4_1", 58, 35),
        ("ffmpeg_4_2", 58, 54),
        ("ffmpeg_4_3", 58, 91),
        ("ffmpeg_4_4", 58, 100),
        ("ffmpeg_5_0", 59, 18),
        ("ffmpeg_5_1", 59, 37),
        ("ffmpeg_6_0", 60, 3),
        ("ffmpeg_6_1", 60, 31),
        ("ffmpeg_7_0", 61, 3),
        ("ffmpeg_7_1", 61, 19),
        ("ffmpeg_8_0", 62, 8),
        ("ffmpeg_8_1", 62, 28),
        ("ffmpeg_9_0", 63, 1),
    ];
    for &(ffmpeg_version_flag, lavc_version_major, lavc_version_minor) in
        ffmpeg_lavc_versions.iter()
    {
        let search_str = format!(
            "[avcodec_version_greater_than_{lavc_version_major}_{lavc_version_minor}]",
            lavc_version_major = lavc_version_major,
            lavc_version_minor = lavc_version_minor - 1
        );
        let pos = stdout
            .find(&search_str)
            .expect("Variable not found in output")
            + search_str.len();
        println!(
            r#"cargo:rustc-check-cfg=cfg(feature, values("{}"))"#,
            ffmpeg_version_flag
        );
        if &stdout[pos..pos + 1] == "1" {
            println!(r#"cargo:rustc-cfg=feature="{ffmpeg_version_flag}""#);
            println!(r#"cargo:{ffmpeg_version_flag}=true"#);
        } else {
            println!(r#"cargo:{ffmpeg_version_flag}="#);
        }
    }
}

fn search_include(include_paths: &[PathBuf], header: &str) -> String {
    for dir in include_paths {
        let include = dir.join(header);
        if fs::metadata(&include).is_ok() {
            if let Some(path_str) = include.as_path().to_str() {
                return path_str.to_string();
            }
            // Fall back to lossy conversion if path contains non-UTF8 characters
            return include.to_string_lossy().to_string();
        }
    }
    format!("/usr/include/{header}")
}

fn maybe_search_include(include_paths: &[PathBuf], header: &str) -> Option<String> {
    let path = search_include(include_paths, header);
    if fs::metadata(&path).is_ok() {
        Some(path)
    } else {
        None
    }
}

fn link_to_libraries(statik: bool, target_os: &str) {
    let ffmpeg_ty = if statik { "static" } else { "dylib" };
    for lib in LIBRARIES {
        let feat_is_enabled = lib.feature_name().and_then(|f| env::var(f).ok()).is_some();
        if !lib.is_feature || feat_is_enabled {
            println!("cargo:rustc-link-lib={}={}", ffmpeg_ty, lib.name);
        }
    }
    // Keep GNU ld from dropping libraries whose symbols it thinks are unused.
    // Apple's ld and MSVC's link.exe reject the option and don't drop them
    // anyway, so skip it there.
    if !matches!(
        target_os,
        "macos" | "ios" | "tvos" | "watchos" | "visionos" | "windows"
    ) {
        println!("cargo:rustc-link-arg=-Wl,--no-as-needed");
    }
    if env::var("CARGO_FEATURE_BUILD_ZLIB").is_ok() && target_os == "linux" {
        println!("cargo:rustc-link-lib=z");
    }
}

fn main() {
    // This copy always builds the recorded FFmpeg source: it never looks
    // for an installed FFmpeg (FFMPEG_DIR, vcpkg or pkg-config).
    assert!(
        env::var("CARGO_FEATURE_BUILD").is_ok(),
        "this ffmpeg-sys-next only builds FFmpeg from source: turn on its `build` feature"
    );
    assert!(
        env::var("HOST") == env::var("TARGET"),
        "FFmpeg is built natively only: build for {} on a {} host",
        env::var("TARGET").unwrap_or_default(),
        env::var("HOST").unwrap_or_default()
    );
    println!(
        "cargo:rerun-if-changed={}",
        SourceRecord::path().display()
    );

    let statik = true;
    let ffmpeg_major_version: u32 = env!("CARGO_PKG_VERSION_MAJOR").parse().unwrap();
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();

    let include_paths: Vec<PathBuf> = {
        println!(
            "cargo:rustc-link-search=native={}",
            search().join("lib").to_string_lossy()
        );
        link_to_libraries(statik, &target_os);
        let record = SourceRecord::read();
        build(&record).unwrap();

        // Check additional required libraries.
        {
            let config_mak = source().join("ffbuild/config.mak");
            let file = File::open(config_mak).unwrap();
            let reader = BufReader::new(file);
            let extra_linker_args = reader
                .lines()
                .filter_map(|line| {
                    let line = line.as_ref().ok()?;

                    if line.starts_with("EXTRALIBS") {
                        Some(
                            line.split('=')
                                .next_back()
                                .unwrap()
                                .split(' ')
                                .map(|s| s.to_string())
                                .collect::<Vec<_>>(),
                        )
                    } else {
                        None
                    }
                })
                .flatten()
                .collect::<Vec<_>>();

            extra_linker_args
                .iter()
                .filter(|flag| flag.starts_with("-l"))
                .map(|lib| &lib[2..])
                .for_each(|lib| println!("cargo:rustc-link-lib={lib}"));

            // Apple's frameworks, as configure found them needed (with
            // --disable-autodetect, only avutil's CoreFoundation, CoreVideo
            // and CoreMedia), rather than a fixed list.
            extra_linker_args
                .windows(2)
                .filter(|pair| pair[0] == "-framework")
                .for_each(|pair| println!("cargo:rustc-link-lib=framework={}", pair[1]));

            // MSVC's configure spells its libraries `name.lib`.
            extra_linker_args
                .iter()
                .filter_map(|flag| flag.strip_suffix(".lib"))
                .filter(|lib| !lib.is_empty() && !lib.starts_with('-'))
                .for_each(|lib| println!("cargo:rustc-link-lib={lib}"));

            extra_linker_args
                .iter()
                .filter(|v| v.starts_with("-L"))
                .map(|flag| {
                    let path = &flag[2..];
                    if path.starts_with('/') {
                        PathBuf::from(path)
                    } else {
                        source().join(path)
                    }
                })
                .for_each(|lib_search_path| {
                    println!(
                        "cargo:rustc-link-search=native={}",
                        lib_search_path.to_string_lossy()
                    );
                })
        }

        vec![search().join("include")]
    };

    check_features(
        include_paths.clone(),
        &[
            ("libavutil/avutil.h", None, "FF_API_OLD_AVOPTIONS"),
            ("libavutil/avutil.h", None, "FF_API_PIX_FMT"),
            ("libavutil/avutil.h", None, "FF_API_CONTEXT_SIZE"),
            ("libavutil/avutil.h", None, "FF_API_PIX_FMT_DESC"),
            ("libavutil/avutil.h", None, "FF_API_AV_REVERSE"),
            ("libavutil/avutil.h", None, "FF_API_AUDIOCONVERT"),
            ("libavutil/avutil.h", None, "FF_API_CPU_FLAG_MMX2"),
            ("libavutil/avutil.h", None, "FF_API_LLS_PRIVATE"),
            ("libavutil/avutil.h", None, "FF_API_AVFRAME_LAVC"),
            ("libavutil/avutil.h", None, "FF_API_VDPAU"),
            (
                "libavutil/avutil.h",
                None,
                "FF_API_GET_CHANNEL_LAYOUT_COMPAT",
            ),
            ("libavutil/avutil.h", None, "FF_API_XVMC"),
            ("libavutil/avutil.h", None, "FF_API_OPT_TYPE_METADATA"),
            ("libavutil/avutil.h", None, "FF_API_DLOG"),
            ("libavutil/avutil.h", None, "FF_API_HMAC"),
            ("libavutil/avutil.h", None, "FF_API_VAAPI"),
            ("libavutil/avutil.h", None, "FF_API_PKT_PTS"),
            ("libavutil/avutil.h", None, "FF_API_ERROR_FRAME"),
            ("libavutil/avutil.h", None, "FF_API_FRAME_QP"),
            (
                "libavcodec/avcodec.h",
                Some("avcodec"),
                "FF_API_VIMA_DECODER",
            ),
            (
                "libavcodec/avcodec.h",
                Some("avcodec"),
                "FF_API_REQUEST_CHANNELS",
            ),
            (
                "libavcodec/avcodec.h",
                Some("avcodec"),
                "FF_API_OLD_DECODE_AUDIO",
            ),
            (
                "libavcodec/avcodec.h",
                Some("avcodec"),
                "FF_API_OLD_ENCODE_AUDIO",
            ),
            (
                "libavcodec/avcodec.h",
                Some("avcodec"),
                "FF_API_OLD_ENCODE_VIDEO",
            ),
            ("libavcodec/avcodec.h", Some("avcodec"), "FF_API_CODEC_ID"),
            (
                "libavcodec/avcodec.h",
                Some("avcodec"),
                "FF_API_AUDIO_CONVERT",
            ),
            (
                "libavcodec/avcodec.h",
                Some("avcodec"),
                "FF_API_AVCODEC_RESAMPLE",
            ),
            (
                "libavcodec/avcodec.h",
                Some("avcodec"),
                "FF_API_DEINTERLACE",
            ),
            (
                "libavcodec/avcodec.h",
                Some("avcodec"),
                "FF_API_DESTRUCT_PACKET",
            ),
            ("libavcodec/avcodec.h", Some("avcodec"), "FF_API_GET_BUFFER"),
            (
                "libavcodec/avcodec.h",
                Some("avcodec"),
                "FF_API_MISSING_SAMPLE",
            ),
            ("libavcodec/avcodec.h", Some("avcodec"), "FF_API_LOWRES"),
            ("libavcodec/avcodec.h", Some("avcodec"), "FF_API_CAP_VDPAU"),
            ("libavcodec/avcodec.h", Some("avcodec"), "FF_API_BUFS_VDPAU"),
            ("libavcodec/avcodec.h", Some("avcodec"), "FF_API_VOXWARE"),
            (
                "libavcodec/avcodec.h",
                Some("avcodec"),
                "FF_API_SET_DIMENSIONS",
            ),
            ("libavcodec/avcodec.h", Some("avcodec"), "FF_API_DEBUG_MV"),
            ("libavcodec/avcodec.h", Some("avcodec"), "FF_API_AC_VLC"),
            (
                "libavcodec/avcodec.h",
                Some("avcodec"),
                "FF_API_OLD_MSMPEG4",
            ),
            (
                "libavcodec/avcodec.h",
                Some("avcodec"),
                "FF_API_ASPECT_EXTENDED",
            ),
            (
                "libavcodec/avcodec.h",
                Some("avcodec"),
                "FF_API_THREAD_OPAQUE",
            ),
            ("libavcodec/avcodec.h", Some("avcodec"), "FF_API_CODEC_PKT"),
            ("libavcodec/avcodec.h", Some("avcodec"), "FF_API_ARCH_ALPHA"),
            ("libavcodec/avcodec.h", Some("avcodec"), "FF_API_ERROR_RATE"),
            (
                "libavcodec/avcodec.h",
                Some("avcodec"),
                "FF_API_QSCALE_TYPE",
            ),
            ("libavcodec/avcodec.h", Some("avcodec"), "FF_API_MB_TYPE"),
            (
                "libavcodec/avcodec.h",
                Some("avcodec"),
                "FF_API_MAX_BFRAMES",
            ),
            (
                "libavcodec/avcodec.h",
                Some("avcodec"),
                "FF_API_NEG_LINESIZES",
            ),
            ("libavcodec/avcodec.h", Some("avcodec"), "FF_API_EMU_EDGE"),
            ("libavcodec/avcodec.h", Some("avcodec"), "FF_API_ARCH_SH4"),
            ("libavcodec/avcodec.h", Some("avcodec"), "FF_API_ARCH_SPARC"),
            (
                "libavcodec/avcodec.h",
                Some("avcodec"),
                "FF_API_UNUSED_MEMBERS",
            ),
            (
                "libavcodec/avcodec.h",
                Some("avcodec"),
                "FF_API_IDCT_XVIDMMX",
            ),
            (
                "libavcodec/avcodec.h",
                Some("avcodec"),
                "FF_API_INPUT_PRESERVED",
            ),
            (
                "libavcodec/avcodec.h",
                Some("avcodec"),
                "FF_API_NORMALIZE_AQP",
            ),
            ("libavcodec/avcodec.h", Some("avcodec"), "FF_API_GMC"),
            ("libavcodec/avcodec.h", Some("avcodec"), "FF_API_MV0"),
            ("libavcodec/avcodec.h", Some("avcodec"), "FF_API_CODEC_NAME"),
            ("libavcodec/avcodec.h", Some("avcodec"), "FF_API_AFD"),
            ("libavcodec/avcodec.h", Some("avcodec"), "FF_API_VISMV"),
            (
                "libavcodec/avcodec.h",
                Some("avcodec"),
                "FF_API_DV_FRAME_PROFILE",
            ),
            (
                "libavcodec/avcodec.h",
                Some("avcodec"),
                "FF_API_AUDIOENC_DELAY",
            ),
            (
                "libavcodec/avcodec.h",
                Some("avcodec"),
                "FF_API_VAAPI_CONTEXT",
            ),
            (
                "libavcodec/avcodec.h",
                Some("avcodec"),
                "FF_API_AVCTX_TIMEBASE",
            ),
            ("libavcodec/avcodec.h", Some("avcodec"), "FF_API_MPV_OPT"),
            (
                "libavcodec/avcodec.h",
                Some("avcodec"),
                "FF_API_STREAM_CODEC_TAG",
            ),
            ("libavcodec/avcodec.h", Some("avcodec"), "FF_API_QUANT_BIAS"),
            (
                "libavcodec/avcodec.h",
                Some("avcodec"),
                "FF_API_RC_STRATEGY",
            ),
            (
                "libavcodec/avcodec.h",
                Some("avcodec"),
                "FF_API_CODED_FRAME",
            ),
            ("libavcodec/avcodec.h", Some("avcodec"), "FF_API_MOTION_EST"),
            (
                "libavcodec/avcodec.h",
                Some("avcodec"),
                "FF_API_WITHOUT_PREFIX",
            ),
            (
                "libavcodec/avcodec.h",
                Some("avcodec"),
                "FF_API_CONVERGENCE_DURATION",
            ),
            (
                "libavcodec/avcodec.h",
                Some("avcodec"),
                "FF_API_PRIVATE_OPT",
            ),
            ("libavcodec/avcodec.h", Some("avcodec"), "FF_API_CODER_TYPE"),
            (
                "libavcodec/avcodec.h",
                Some("avcodec"),
                "FF_API_RTP_CALLBACK",
            ),
            ("libavcodec/avcodec.h", Some("avcodec"), "FF_API_STAT_BITS"),
            ("libavcodec/avcodec.h", Some("avcodec"), "FF_API_VBV_DELAY"),
            (
                "libavcodec/avcodec.h",
                Some("avcodec"),
                "FF_API_SIDEDATA_ONLY_PKT",
            ),
            ("libavcodec/avcodec.h", Some("avcodec"), "FF_API_AVPICTURE"),
            (
                "libavformat/avformat.h",
                Some("avformat"),
                "FF_API_LAVF_BITEXACT",
            ),
            (
                "libavformat/avformat.h",
                Some("avformat"),
                "FF_API_LAVF_FRAC",
            ),
            (
                "libavformat/avformat.h",
                Some("avformat"),
                "FF_API_URL_FEOF",
            ),
            (
                "libavformat/avformat.h",
                Some("avformat"),
                "FF_API_PROBESIZE_32",
            ),
            (
                "libavformat/avformat.h",
                Some("avformat"),
                "FF_API_LAVF_AVCTX",
            ),
            (
                "libavformat/avformat.h",
                Some("avformat"),
                "FF_API_OLD_OPEN_CALLBACKS",
            ),
            (
                "libavfilter/avfilter.h",
                Some("avfilter"),
                "FF_API_AVFILTERPAD_PUBLIC",
            ),
            (
                "libavfilter/avfilter.h",
                Some("avfilter"),
                "FF_API_FOO_COUNT",
            ),
            (
                "libavfilter/avfilter.h",
                Some("avfilter"),
                "FF_API_OLD_FILTER_OPTS",
            ),
            (
                "libavfilter/avfilter.h",
                Some("avfilter"),
                "FF_API_OLD_FILTER_OPTS_ERROR",
            ),
            (
                "libavfilter/avfilter.h",
                Some("avfilter"),
                "FF_API_AVFILTER_OPEN",
            ),
            (
                "libavfilter/avfilter.h",
                Some("avfilter"),
                "FF_API_OLD_FILTER_REGISTER",
            ),
            (
                "libavfilter/avfilter.h",
                Some("avfilter"),
                "FF_API_OLD_GRAPH_PARSE",
            ),
            (
                "libavfilter/avfilter.h",
                Some("avfilter"),
                "FF_API_NOCONST_GET_NAME",
            ),
            (
                "libavresample/avresample.h",
                Some("avresample"),
                "FF_API_RESAMPLE_CLOSE_OPEN",
            ),
            (
                "libswscale/swscale.h",
                Some("swscale"),
                "FF_API_SWS_CPU_CAPS",
            ),
            ("libswscale/swscale.h", Some("swscale"), "FF_API_ARCH_BFIN"),
        ],
    );

    let clang_includes = include_paths
        .iter()
        .map(|include| format!("-I{}", include.to_string_lossy()));

    println!("cargo:rerun-if-changed=hwcontext_wrapper.h");
    println!("cargo:rerun-if-changed=hwcontext_stubs/vulkan/vulkan.h");
    println!("cargo:rerun-if-changed=hwcontext_stubs/VideoToolbox/VideoToolbox.h");
    println!("cargo:rerun-if-changed=hwcontext_stubs/va/va.h");
    println!("cargo:rerun-if-changed=hwcontext_stubs/mfxvideo.h");
    println!("cargo:rerun-if-changed=hwcontext_stubs/mfx/mfxvideo.h");

    // SDK shims (vulkan/, VideoToolbox/, va/, mfxvideo.h) used by hwcontext_wrapper.h.
    let hwcontext_stub_dir = format!(
        "-I{}/hwcontext_stubs",
        env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR")
    );

    // The bindgen::Builder is the main entry point
    // to bindgen, and lets you build up options for
    // the resulting bindings.
    let mut builder = bindgen::Builder::default()
        // Shim dir first, so our stubs win over the real SDK headers on the path (by design, to avoid generating bindings for full SDKs).
        .clang_arg(&hwcontext_stub_dir)
        .clang_arg("-fno-builtin")
        .clang_args(clang_includes)
        .ctypes_prefix("libc")
        // https://github.com/rust-lang/rust-bindgen/issues/550
        .blocklist_type("max_align_t")
        .blocklist_function("_.*")
        // Blocklist functions with u128 in signature.
        // https://github.com/zmwangx/rust-ffmpeg-sys/issues/1
        // https://github.com/rust-lang/rust-bindgen/issues/1549
        .blocklist_function("acoshl")
        .blocklist_function("acosl")
        .blocklist_function("asinhl")
        .blocklist_function("asinl")
        .blocklist_function("atan2l")
        .blocklist_function("atanhl")
        .blocklist_function("atanl")
        .blocklist_function("cbrtl")
        .blocklist_function("ceill")
        .blocklist_function("copysignl")
        .blocklist_function("coshl")
        .blocklist_function("cosl")
        .blocklist_function("dreml")
        .blocklist_function("ecvt_r")
        .blocklist_function("erfcl")
        .blocklist_function("erfl")
        .blocklist_function("exp2l")
        .blocklist_function("expl")
        .blocklist_function("expm1l")
        .blocklist_function("fabsl")
        .blocklist_function("fcvt_r")
        .blocklist_function("fdiml")
        .blocklist_function("finitel")
        .blocklist_function("floorl")
        .blocklist_function("fmal")
        .blocklist_function("fmaxl")
        .blocklist_function("fminl")
        .blocklist_function("fmodl")
        .blocklist_function("frexpl")
        .blocklist_function("gammal")
        .blocklist_function("hypotl")
        .blocklist_function("ilogbl")
        .blocklist_function("isinfl")
        .blocklist_function("isnanl")
        .blocklist_function("j0l")
        .blocklist_function("j1l")
        .blocklist_function("jnl")
        .blocklist_function("ldexpl")
        .blocklist_function("lgammal")
        .blocklist_function("lgammal_r")
        .blocklist_function("llrintl")
        .blocklist_function("llroundl")
        .blocklist_function("log10l")
        .blocklist_function("log1pl")
        .blocklist_function("log2l")
        .blocklist_function("logbl")
        .blocklist_function("logl")
        .blocklist_function("lrintl")
        .blocklist_function("lroundl")
        .blocklist_function("modfl")
        .blocklist_function("nanl")
        .blocklist_function("nearbyintl")
        .blocklist_function("nextafterl")
        .blocklist_function("nexttoward")
        .blocklist_function("nexttowardf")
        .blocklist_function("nexttowardl")
        .blocklist_function("powl")
        .blocklist_function("qecvt")
        .blocklist_function("qecvt_r")
        .blocklist_function("qfcvt")
        .blocklist_function("qfcvt_r")
        .blocklist_function("qgcvt")
        .blocklist_function("remainderl")
        .blocklist_function("remquol")
        .blocklist_function("rintl")
        .blocklist_function("roundl")
        .blocklist_function("scalbl")
        .blocklist_function("scalblnl")
        .blocklist_function("scalbnl")
        .blocklist_function("significandl")
        .blocklist_function("sinhl")
        .blocklist_function("sinl")
        .blocklist_function("sqrtl")
        .blocklist_function("strtold")
        .blocklist_function("tanhl")
        .blocklist_function("tanl")
        .blocklist_function("tgammal")
        .blocklist_function("truncl")
        .blocklist_function("y0l")
        .blocklist_function("y1l")
        .blocklist_function("ynl")
        .opaque_type("__mingw_ldbl_type_t")
        .default_enum_style(bindgen::EnumVariation::Rust {
            non_exhaustive: env::var("CARGO_FEATURE_NON_EXHAUSTIVE_ENUMS").is_ok(),
        })
        .prepend_enum_name(false)
        .wrap_unsafe_ops(true)
        .derive_eq(true)
        .size_t_is_usize(true)
        .parse_callbacks(Box::new(Callbacks));

    // The input headers we would like to generate
    // bindings for.
    if env::var("CARGO_FEATURE_AVCODEC").is_ok() {
        builder = builder
            .header(search_include(&include_paths, "libavcodec/avcodec.h"))
            .header(search_include(&include_paths, "libavcodec/dv_profile.h"))
            .header(search_include(&include_paths, "libavcodec/vorbis_parser.h"));

        let bsf_path = search_include(&include_paths, "libavcodec/bsf.h");
        if std::path::Path::new(&bsf_path).exists() {
            builder = builder.header(bsf_path);
        }

        if ffmpeg_major_version < 5 {
            builder = builder.header(search_include(&include_paths, "libavcodec/vaapi.h"));
        }
        if ffmpeg_major_version < 8
            && let Some(avfft_path) = maybe_search_include(&include_paths, "libavcodec/avfft.h")
        {
            builder = builder.header(avfft_path);
        }
    }

    if env::var("CARGO_FEATURE_AVDEVICE").is_ok() {
        builder = builder.header(search_include(&include_paths, "libavdevice/avdevice.h"));
    }

    if env::var("CARGO_FEATURE_AVFILTER").is_ok() {
        builder = builder
            .header(search_include(&include_paths, "libavfilter/buffersink.h"))
            .header(search_include(&include_paths, "libavfilter/buffersrc.h"))
            .header(search_include(&include_paths, "libavfilter/avfilter.h"));
    }

    if env::var("CARGO_FEATURE_AVFORMAT").is_ok() {
        builder = builder
            .header(search_include(&include_paths, "libavformat/avformat.h"))
            .header(search_include(&include_paths, "libavformat/avio.h"));
    }

    if env::var("CARGO_FEATURE_AVRESAMPLE").is_ok() {
        builder = builder.header(search_include(&include_paths, "libavresample/avresample.h"));
    }

    builder = builder
        .header(search_include(&include_paths, "libavutil/adler32.h"))
        .header(search_include(&include_paths, "libavutil/aes.h"))
        .header(search_include(&include_paths, "libavutil/audio_fifo.h"))
        .header(search_include(&include_paths, "libavutil/base64.h"))
        .header(search_include(&include_paths, "libavutil/blowfish.h"))
        .header(search_include(&include_paths, "libavutil/bprint.h"))
        .header(search_include(&include_paths, "libavutil/buffer.h"))
        .header(search_include(&include_paths, "libavutil/camellia.h"))
        .header(search_include(&include_paths, "libavutil/cast5.h"))
        .header(search_include(&include_paths, "libavutil/channel_layout.h"))
        // Here until https://github.com/rust-lang/rust-bindgen/issues/2192 /
        // https://github.com/rust-lang/rust-bindgen/issues/258 is fixed.
        .header("channel_layout_fixed.h")
        .header(search_include(&include_paths, "libavutil/cpu.h"))
        .header(search_include(&include_paths, "libavutil/crc.h"))
        .header(search_include(&include_paths, "libavutil/dict.h"))
        .header(search_include(&include_paths, "libavutil/display.h"))
        .header(search_include(&include_paths, "libavutil/downmix_info.h"))
        .header(search_include(&include_paths, "libavutil/error.h"))
        .header(search_include(&include_paths, "libavutil/eval.h"))
        .header(search_include(&include_paths, "libavutil/fifo.h"))
        .header(search_include(&include_paths, "libavutil/file.h"))
        .header(search_include(&include_paths, "libavutil/frame.h"))
        .header(search_include(&include_paths, "libavutil/hash.h"))
        .header(search_include(&include_paths, "libavutil/hmac.h"))
        .header(search_include(&include_paths, "libavutil/hwcontext.h"))
        .header(search_include(&include_paths, "libavutil/imgutils.h"))
        .header(search_include(&include_paths, "libavutil/lfg.h"))
        .header(search_include(&include_paths, "libavutil/log.h"))
        .header(search_include(&include_paths, "libavutil/lzo.h"))
        .header(search_include(&include_paths, "libavutil/macros.h"))
        .header(search_include(&include_paths, "libavutil/mathematics.h"))
        .header(search_include(&include_paths, "libavutil/md5.h"))
        .header(search_include(&include_paths, "libavutil/mem.h"))
        .header(search_include(&include_paths, "libavutil/motion_vector.h"))
        .header(search_include(&include_paths, "libavutil/murmur3.h"))
        .header(search_include(&include_paths, "libavutil/opt.h"))
        .header(search_include(&include_paths, "libavutil/parseutils.h"))
        .header(search_include(&include_paths, "libavutil/pixdesc.h"))
        .header(search_include(&include_paths, "libavutil/pixfmt.h"))
        .header(search_include(&include_paths, "libavutil/random_seed.h"))
        .header(search_include(&include_paths, "libavutil/rational.h"))
        .header(search_include(&include_paths, "libavutil/replaygain.h"))
        .header(search_include(&include_paths, "libavutil/ripemd.h"))
        .header(search_include(&include_paths, "libavutil/samplefmt.h"))
        .header(search_include(&include_paths, "libavutil/sha.h"))
        .header(search_include(&include_paths, "libavutil/sha512.h"))
        .header(search_include(&include_paths, "libavutil/spherical.h"))
        .header(search_include(&include_paths, "libavutil/stereo3d.h"))
        .header(search_include(&include_paths, "libavutil/avstring.h"))
        .header(search_include(&include_paths, "libavutil/threadmessage.h"))
        .header(search_include(&include_paths, "libavutil/time.h"))
        .header(search_include(&include_paths, "libavutil/timecode.h"))
        .header(search_include(&include_paths, "libavutil/twofish.h"))
        .header(search_include(&include_paths, "libavutil/avutil.h"))
        .header(search_include(&include_paths, "libavutil/xtea.h"));

    let raw_color_params_path = search_include(&include_paths, "libavutil/raw_color_params.h");
    if std::path::Path::new(&raw_color_params_path).exists() {
        builder = builder.header(raw_color_params_path);
    }

    if env::var("CARGO_FEATURE_POSTPROC").is_ok() {
        let postproc_path = search_include(&include_paths, "libpostproc/postprocess.h");
        if std::path::Path::new(&postproc_path).exists() {
            builder = builder.header(postproc_path);
        }
    }

    if env::var("CARGO_FEATURE_SWRESAMPLE").is_ok() {
        builder = builder.header(search_include(&include_paths, "libswresample/swresample.h"));
    }

    if env::var("CARGO_FEATURE_SWSCALE").is_ok() {
        builder = builder.header(search_include(&include_paths, "libswscale/swscale.h"));
    }

    if let Some(hwcontext_drm_header) =
        maybe_search_include(&include_paths, "libavutil/hwcontext_drm.h")
    {
        builder = builder.header(hwcontext_drm_header);
    }

    // Per-API hwcontext structs (CUDA, D3D11/12, VideoToolbox, MediaCodec,
    // Vulkan, VAAPI, QSV) all bind through one wrapper.
    builder = builder.header("hwcontext_wrapper.h");

    // Finish the builder and generate the bindings.
    let bindings = builder
        .generate()
        // Unwrap the Result and panic on failure.
        .expect("Unable to generate bindings");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    bindings
        .write_to_file(output().join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
