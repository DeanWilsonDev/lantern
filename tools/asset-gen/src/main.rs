//! Generates every placeholder asset under `assets/` from code, so the repository needs no
//! binary art pipeline and every byte can be rebuilt (decision D-008).
//!
//! ```text
//! cargo run -p asset-gen            # rewrite assets/
//! cargo run -p asset-gen -- --check # fail if assets/ differs from what this tool produces
//! ```
//!
//! Everything is deterministic: no clocks, no random seeds from the OS.

mod glb_writer;
mod synth;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use lantern_core::color::Color;
use lantern_renderer_3d::texture::TextureData;

/// One generated file: its path relative to the repository root, and its bytes.
struct GeneratedAsset {
    relative_path: &'static str,
    bytes: Vec<u8>,
}

fn generate_all_assets() -> Vec<GeneratedAsset> {
    let mut assets = Vec::new();
    let mut texture =
        |relative_path, data: TextureData| assets.push(GeneratedAsset { relative_path, bytes: data.to_png_bytes() });
    // Checkers are 2 cells per texture; world-UV boxes repeat them once per metre, so each
    // square is half a metre (see `TextureData::checker`).
    texture(
        "assets/textures/ground.png",
        TextureData::checker(128, 2, Color::rgb(0.30, 0.42, 0.26), Color::rgb(0.24, 0.34, 0.21)),
    );
    texture(
        "assets/textures/wall.png",
        TextureData::checker(128, 2, Color::rgb(0.55, 0.52, 0.48), Color::rgb(0.45, 0.42, 0.39)),
    );
    texture(
        "assets/textures/platform.png",
        TextureData::checker(128, 2, Color::rgb(0.80, 0.55, 0.25), Color::rgb(0.68, 0.45, 0.20)),
    );
    texture(
        "assets/textures/player.png",
        TextureData::checker(64, 4, Color::rgb(0.95, 0.95, 0.95), Color::rgb(0.80, 0.85, 0.95)),
    );

    assets.push(GeneratedAsset { relative_path: "assets/models/gem.glb", bytes: glb_writer::gem_glb() });

    for (relative_path, samples) in [
        ("assets/sounds/jump.wav", synth::jump()),
        ("assets/sounds/land.wav", synth::land()),
        ("assets/sounds/pickup.wav", synth::pickup()),
        ("assets/sounds/menu_move.wav", synth::menu_move()),
        ("assets/sounds/menu_select.wav", synth::menu_select()),
        ("assets/sounds/reset.wav", synth::reset()),
        ("assets/music/theme.wav", synth::theme_loop()),
    ] {
        assets.push(GeneratedAsset {
            relative_path,
            bytes: lantern_audio::wav::encode_wav_mono(&samples, synth::SAMPLE_RATE),
        });
    }
    assets
}

/// The repository root: two levels above this crate's manifest.
fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().expect("the repository root exists")
}

fn main() -> ExitCode {
    let check_only = std::env::args().any(|argument| argument == "--check");
    let root = repository_root();
    let mut mismatched_paths = Vec::new();
    for asset in generate_all_assets() {
        let path = root.join(asset.relative_path);
        if check_only {
            if std::fs::read(&path).ok().as_deref() != Some(asset.bytes.as_slice()) {
                mismatched_paths.push(asset.relative_path);
            }
            continue;
        }
        std::fs::create_dir_all(path.parent().expect("asset paths have a directory")).expect("create asset directory");
        std::fs::write(&path, &asset.bytes).unwrap_or_else(|io| panic!("write {}: {io}", path.display()));
        println!("wrote {} ({} bytes)", asset.relative_path, asset.bytes.len());
    }
    if mismatched_paths.is_empty() {
        if check_only {
            println!("assets/ is up to date");
        }
        ExitCode::SUCCESS
    } else {
        for path in mismatched_paths {
            eprintln!("out of date: {path}");
        }
        eprintln!("run `cargo run -p asset-gen` and commit the result");
        ExitCode::FAILURE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_assets_load_through_the_engine_loaders() {
        for asset in generate_all_assets() {
            let path = asset.relative_path;
            if path.ends_with(".png") {
                TextureData::from_png_bytes(&asset.bytes).unwrap_or_else(|error| panic!("{path}: {error}"));
            } else if path.ends_with(".wav") {
                let sound =
                    lantern_audio::wav::decode_wav(&asset.bytes).unwrap_or_else(|error| panic!("{path}: {error}"));
                assert!(sound.duration_seconds() > 0.03, "{path} is too short");
                let peak = sound.samples.iter().fold(0.0f32, |peak, sample| peak.max(sample.abs()));
                assert!(peak > 0.1 && peak <= 1.0, "{path} peak {peak}");
            } else if path.ends_with(".glb") {
                let mesh = lantern_renderer_3d::gltf_loader::gltf_mesh_from_bytes(&asset.bytes, Path::new("."))
                    .unwrap_or_else(|error| panic!("{path}: {error}"));
                assert!(mesh.triangle_count() >= 8);
            }
        }
    }

    #[test]
    fn generation_is_deterministic() {
        let first: Vec<Vec<u8>> = generate_all_assets().into_iter().map(|asset| asset.bytes).collect();
        let second: Vec<Vec<u8>> = generate_all_assets().into_iter().map(|asset| asset.bytes).collect();
        assert_eq!(first, second);
    }

    #[test]
    fn music_loop_ends_where_it_starts() {
        // A loop that ends far from zero clicks at the seam.
        let samples = synth::theme_loop();
        assert!(samples.first().unwrap().abs() < 0.02);
        assert!(samples.last().unwrap().abs() < 0.02);
    }
}
