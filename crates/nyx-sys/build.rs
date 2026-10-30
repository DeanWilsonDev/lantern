use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

const MINIMUM_CMAKE: (u32, u32) = (3, 30);
const STATIC_LIBRARIES: [&str; 7] =
    ["nyx-capi-static", "nyx-host-surface", "nyx-core", "lucis-json", "lucis", "firefly", "amanuensis"];

/// With the `chaos-ui` feature, the archives Chaos, Cosmos and Lantern's bridge add, linked
/// before Nyx's because they call into it.
const CHAOS_UI_LIBRARIES: [&str; 3] = ["lantern_chaos_ui", "chaos", "cosmos"];

fn main() {
    println!("cargo:rerun-if-env-changed=NYX_SOURCE_DIR");
    println!("cargo:rerun-if-env-changed=NYX_CAPI_PREFIX");
    println!("cargo:rerun-if-env-changed=CHAOS_SOURCE_DIR");
    println!("cargo:rerun-if-env-changed=COSMOS_SOURCE_DIR");

    let header = if env::var_os("CARGO_FEATURE_CHAOS_UI").is_some() {
        if env::var_os("NYX_CAPI_PREFIX").is_some() {
            panic!(
                "nyx-sys's `chaos-ui` feature builds Nyx, Chaos and Cosmos together so they share one copy \
                 of Nyx, so it can't use a prebuilt NYX_CAPI_PREFIX; unset it."
            );
        }
        build_with_chaos_ui(&nyx_source_dir())
    } else {
        match env::var_os("NYX_CAPI_PREFIX") {
            Some(prefix) => link_installed_prefix(Path::new(&prefix)),
            None => build_from_source(&nyx_source_dir()),
        }
    };
    link_cpp_standard_library();
    generate_bindings(&header);
}

fn nyx_source_dir() -> PathBuf {
    if let Some(dir) = env::var_os("NYX_SOURCE_DIR") {
        return PathBuf::from(dir);
    }
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("cargo sets CARGO_MANIFEST_DIR"));
    let sibling = manifest_dir.join("../../../nyx-scripting-language");
    if sibling.join("include/nyx/nyx.h").is_file() {
        return sibling;
    }
    panic!(
        "nyx-sys needs a nyx-scripting-language checkout that has the C API, but {} was not found. \
         Check out a branch with include/nyx/nyx.h there (proto once the C API is merged into it), \
         set NYX_SOURCE_DIR to another checkout, or set NYX_CAPI_PREFIX to a \
         `cmake --install build --component nyx-capi` prefix.",
        sibling.join("include/nyx/nyx.h").display()
    );
}

fn check_cmake_version() {
    let output = Command::new(env::var("CMAKE").unwrap_or_else(|_| "cmake".to_string()))
        .arg("--version")
        .output()
        .unwrap_or_else(|error| {
            panic!("nyx-sys needs CMake {}.{} or later on PATH: {error}", MINIMUM_CMAKE.0, MINIMUM_CMAKE.1)
        });
    let text = String::from_utf8_lossy(&output.stdout);
    let version = text
        .split_whitespace()
        .find(|word| word.chars().next().is_some_and(|first| first.is_ascii_digit()))
        .unwrap_or("0.0");
    let mut parts = version.split('.').map(|part| part.parse::<u32>().unwrap_or(0));
    let found = (parts.next().unwrap_or(0), parts.next().unwrap_or(0));
    if found < MINIMUM_CMAKE {
        panic!(
            "nyx-sys needs CMake {}.{} or later, found {version}. Nyx also needs GCC 14 or Clang 19; set CC and CXX to them.",
            MINIMUM_CMAKE.0, MINIMUM_CMAKE.1
        );
    }
}

fn build_from_source(source: &Path) -> PathBuf {
    check_cmake_version();
    for watched in ["include/nyx/nyx.h", "src", "CMakeLists.txt", "external"] {
        println!("cargo:rerun-if-changed={}", source.join(watched).display());
    }
    let destination = cmake::Config::new(source)
        .profile("Release")
        .define("NYX_BUILD_CAPI", "ON")
        .define("NYX_BUILD_TESTS", "OFF")
        .define("NYX_BUILD_TOOLS", "OFF")
        .define("NYX_BUILD_EXAMPLES", "OFF")
        .define("NYX_BUILD_LSP_LIBRARY", "OFF")
        .build_target("nyx-capi-static")
        .build();
    let build_dir = destination.join("build");
    for library in STATIC_LIBRARIES {
        let archive = find_archive(&build_dir, library)
            .unwrap_or_else(|| panic!("the nyx build made no lib{library}.a under {}", build_dir.display()));
        println!("cargo:rustc-link-search=native={}", archive.parent().expect("an archive has a directory").display());
        println!("cargo:rustc-link-lib=static={library}");
    }
    source.join("include/nyx/nyx.h")
}

/// A sibling checkout of `name` next to the lantern checkout (like Nyx's), or the directory in
/// `variable`; None lets the native build fetch it from GitHub.
fn optional_source_dir(variable: &str, name: &str, marker: &str) -> Option<PathBuf> {
    let dir = match env::var_os(variable) {
        Some(dir) => PathBuf::from(dir),
        None => {
            let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("cargo sets CARGO_MANIFEST_DIR"));
            let sibling = manifest_dir.join("../../..").join(name);
            if !sibling.join("CMakeLists.txt").is_file() {
                return None;
            }
            sibling
        }
    };
    if !dir.join(marker).is_file() {
        panic!(
            "{} has no {marker}; run `git submodule update --init --recursive` there, or set {variable} \
             to another checkout",
            dir.display()
        );
    }
    Some(dir)
}

/// Nyx, Chaos, Cosmos and Lantern's Chaos bridge in one CMake build
/// (crates/chaos-ui/native/CMakeLists.txt), so Chaos runs on the same Nyx the C API does.
fn build_with_chaos_ui(nyx_source: &Path) -> PathBuf {
    check_cmake_version();
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("cargo sets CARGO_MANIFEST_DIR"));
    let native = manifest_dir.join("../chaos-ui/native");
    let chaos = optional_source_dir("CHAOS_SOURCE_DIR", "chaos", "libs/umbra-interfaces/CMakeLists.txt");
    let cosmos = optional_source_dir("COSMOS_SOURCE_DIR", "cosmos", "libs/amanuensis/CMakeLists.txt");
    for watched in [native.join("CMakeLists.txt"), native.join("src"), native.join("include")] {
        println!("cargo:rerun-if-changed={}", watched.display());
    }
    for watched in ["include/nyx", "src", "CMakeLists.txt"] {
        println!("cargo:rerun-if-changed={}", nyx_source.join(watched).display());
    }
    for source in chaos.iter().chain(cosmos.iter()) {
        for watched in ["include", "src", "CMakeLists.txt"] {
            println!("cargo:rerun-if-changed={}", source.join(watched).display());
        }
    }
    let path_text = |path: &Path| path.to_string_lossy().into_owned();
    let mut config = cmake::Config::new(&native);
    config.profile("Release").define("LANTERN_NYX_SOURCE_DIR", path_text(nyx_source));
    if let Some(chaos) = &chaos {
        config.define("LANTERN_CHAOS_SOURCE_DIR", path_text(chaos));
    }
    if let Some(cosmos) = &cosmos {
        config.define("LANTERN_COSMOS_SOURCE_DIR", path_text(cosmos));
    }
    let destination = config.build_target("lantern_native_stack").build();
    let build_dir = destination.join("build");
    for library in CHAOS_UI_LIBRARIES.iter().chain(STATIC_LIBRARIES.iter()) {
        let archive = find_archive(&build_dir, library)
            .unwrap_or_else(|| panic!("the chaos-ui build made no lib{library}.a under {}", build_dir.display()));
        println!("cargo:rustc-link-search=native={}", archive.parent().expect("an archive has a directory").display());
        println!("cargo:rustc-link-lib=static={library}");
    }
    nyx_source.join("include/nyx/nyx.h")
}

fn link_installed_prefix(prefix: &Path) -> PathBuf {
    let header = prefix.join("include/nyx/nyx.h");
    if !header.is_file() {
        panic!("NYX_CAPI_PREFIX {} has no include/nyx/nyx.h", prefix.display());
    }
    for library_dir in ["lib", "lib64"] {
        println!("cargo:rustc-link-search=native={}", prefix.join(library_dir).display());
    }
    for library in STATIC_LIBRARIES {
        println!("cargo:rustc-link-lib=static={library}");
    }
    println!("cargo:rerun-if-changed={}", header.display());
    header
}

fn find_archive(dir: &Path, library: &str) -> Option<PathBuf> {
    let wanted = format!("lib{library}.a");
    let entries = std::fs::read_dir(dir).ok()?;
    let mut subdirectories = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            subdirectories.push(path);
        } else if path.file_name().is_some_and(|name| name == wanted.as_str()) {
            return Some(path);
        }
    }
    subdirectories.into_iter().find_map(|subdirectory| find_archive(&subdirectory, library))
}

fn link_cpp_standard_library() {
    let target = env::var("TARGET").unwrap_or_default();
    if target.contains("apple") || target.contains("freebsd") {
        println!("cargo:rustc-link-lib=dylib=c++");
    } else if !target.contains("msvc") {
        println!("cargo:rustc-link-lib=dylib=stdc++");
    }
}

fn generate_bindings(header: &Path) {
    let bindings = bindgen::Builder::default()
        .header(header.to_string_lossy())
        .allowlist_function("nyx_.*")
        .allowlist_type("nyx_.*")
        .allowlist_var("NYX_.*")
        .prepend_enum_name(false)
        .layout_tests(true)
        .generate()
        .expect("bindgen couldn't read include/nyx/nyx.h");
    let out = PathBuf::from(env::var("OUT_DIR").expect("cargo sets OUT_DIR")).join("nyx_bindings.rs");
    bindings.write_to_file(&out).expect("couldn't write the nyx bindings");
}
