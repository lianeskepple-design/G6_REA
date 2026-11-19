#[cfg(windows)]
use std::path::Path;
#[cfg(windows)]
use std::process::Command;

const SOURCES: [&str; 7] = [
    "g6_crystalizer.c",
    "g6_crystalizer_legacy.c",
    "g6_bass.c",
    "g6_dialog.c",
    "g6_surround.c",
    "g6_svm.c",
    "g6_chain_capi.c",
];

const RERUN: [&str; 11] = [
    "../reference/g6_crystalizer.c",
    "../reference/g6_crystalizer_legacy.c",
    "../reference/g6_bass.c",
    "../reference/g6_bass.h",
    "../reference/g6_dialog.c",
    "../reference/g6_dialog.h",
    "../reference/g6_biquad.h",
    "../reference/g6_surround.c",
    "../reference/g6_svm.c",
    "../reference/g6_chain_capi.c",
    "../reference/g6_chain_capi.h",
];

#[cfg(windows)]
fn build_chain() {
    let manifest = std::path::PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"),
    );
    let profile = std::env::var("PROFILE").unwrap_or_else(|_| "dev".to_string());
    let static_crt = std::env::var_os("G6_STATIC_CRT").is_some();
    let suffix = if static_crt { "-static" } else { "" };
    let relative_out = format!("target\\g6dsp-cc-{profile}{suffix}");
    let crt_flag = if static_crt { "/MT" } else { "/MD" };
    let vcvars = std::env::var("G6_VCVARS").unwrap_or_else(|_| {
        r"C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat"
            .to_string()
    });
    if !Path::new(&vcvars).is_file() {
        panic!("g6dsp: vcvars64.bat not found at {vcvars}");
    }

    let mut objects = Vec::new();
    let mut lines = String::new();
    lines.push_str("@echo off\r\n");
    lines.push_str(&format!(
        "call \"{vcvars}\" >nul\r\nif errorlevel 1 exit /b 1\r\n"
    ));
    lines.push_str(&format!("if not exist \"{relative_out}\" mkdir \"{relative_out}\"\r\n"));
    lines.push_str(&format!("del /q \"{relative_out}\\*.obj\" 2>nul\r\n"));
    for source in SOURCES {
        let stem = Path::new(source)
            .file_stem()
            .expect("source stem")
            .to_string_lossy();
        let object = format!("{relative_out}\\{stem}.obj");
        lines.push_str(&format!(
            "cl /nologo /TC /std:c11 /W4 /fp:strict {crt_flag} /I \"..\\reference\" \
             /Fo\"{object}\" /c \"..\\reference\\{source}\" || exit /b 2\r\n"
        ));
        objects.push(object);
    }
    lines.push_str(&format!(
        "lib /nologo /OUT:\"{relative_out}\\g6dsp.lib\" {}\r\nif errorlevel 1 exit /b 3\r\n",
        objects
            .iter()
            .map(|object| format!("\"{object}\""))
            .collect::<Vec<_>>()
            .join(" ")
    ));
    lines.push_str("exit /b 0\r\n");

    let target_dir = manifest.join("target");
    if let Err(err) = std::fs::create_dir_all(&target_dir) {
        panic!("g6dsp: cannot create {}: {err}", target_dir.display());
    }
    let script = target_dir.join("g6dsp-compile.bat");
    if let Err(err) = std::fs::write(&script, lines) {
        panic!("g6dsp: cannot write {}: {err}", script.display());
    }
    let relative_script = "target\\g6dsp-compile.bat";
    let status = Command::new("cmd")
        .arg("/c")
        .arg(&relative_script)
        .current_dir(&manifest)
        .status()
        .expect("run g6dsp compile helper");
    if !status.success() {
        panic!("g6dsp: C build failed with {status}");
    }

    let lib_dir = manifest.join(format!("target/g6dsp-cc-{profile}{suffix}"));
    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    println!("cargo:rustc-link-lib=static=g6dsp");
}

#[cfg(not(windows))]
fn build_chain() {
    let mut build = cc::Build::new();
    build
        .files(SOURCES.iter().map(|source| format!("../reference/{source}")))
        .include("../reference")
        .flag_if_supported("-std=c11")
        .flag_if_supported("-W4")
        .flag_if_supported("-ffp-contract=off");
    build.compile("g6dsp");
}

fn main() {
    for path in RERUN {
        println!("cargo:rerun-if-changed={path}");
    }
    println!("cargo:rerun-if-env-changed=G6_STATIC_CRT");
    build_chain();
}
