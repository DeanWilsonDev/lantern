use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

const MINIMUM_CMAKE: (u32, u32) = (3, 30);
const STATIC_LIBRARIES: [&str; 5] = ["nyx-capi-static", "nyx-core", "lucis", "firefly", "amanuensis"];

fn main() {
    println!("cargo:rerun-if-env-changed=NYX_SOURCE_DIR");
    println!("cargo:rerun-if-env-changed=NYX_CAPI_PREFIX");

    let header = match env::var_os("NYX_CAPI_PREFIX") {
        Some(prefix) => link_installed_prefix(Path::new(&prefix)),
        None => build_from_source(&nyx_source_dir()),
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
        "nyx-sys needs a nyx-scripting-language checkout on its proto branch. Set NYX_SOURCE_DIR to it, \
         put it beside this repository, or set NYX_CAPI_PREFIX to a `cmake --install build --component nyx-capi` prefix."
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
