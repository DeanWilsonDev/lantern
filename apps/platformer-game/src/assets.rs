//! Finding and loading the game's assets, with a generated fallback for every file.
//!
//! A missing or broken file is logged once with its path and the reason, then replaced by a
//! procedural stand-in, so a fresh checkout without `assets/` still runs (brief: "generated
//! fallback assets and clear load errors").

use std::path::{Path, PathBuf};

use lantern_core::color::Color;
use lantern_engine::audio::AudioSystem;
use lantern_renderer_3d::gltf_loader::load_gltf_mesh;
use lantern_renderer_3d::mesh::MeshData;
use lantern_renderer_3d::texture::TextureData;
use lantern_renderer_3d::{MeshHandle, TextureHandle, primitives, text};
use lantern_sdl_gpu::{GpuError, GpuRenderer};

/// The player's capsule: 0.35 m radius, 1.7 m tall. Shared with the physics controller.
pub const PLAYER_RADIUS: f32 = 0.35;
pub const PLAYER_HEIGHT: f32 = 1.7;

/// Finds the `assets/` directory. Checked in order, first existing wins:
/// 1. `LANTERN_ASSETS` (an explicit override);
/// 2. `assets/` next to the executable (a packaged build);
/// 3. the repository's `assets/` (running with `cargo run` from anywhere in the checkout);
/// 4. `assets/` in the working directory.
pub fn find_asset_directory() -> Option<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(explicit) = std::env::var_os("LANTERN_ASSETS") {
        candidates.push(PathBuf::from(explicit));
    }
    if let Some(executable_directory) =
        std::env::current_exe().ok().and_then(|path| path.parent().map(Path::to_path_buf))
    {
        candidates.push(executable_directory.join("assets"));
    }
    candidates.push(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets"));
    candidates.push(PathBuf::from("assets"));
    let found = candidates.into_iter().find(|candidate| candidate.is_dir());
    match &found {
        Some(directory) => log::info!(target: "assets", "assets directory: {}", directory.display()),
        None => log::warn!(target: "assets", "no assets directory found; using generated fallbacks for everything"),
    }
    found
}

/// GPU resources that live as long as the game. Level meshes are not here: they belong to the
/// level scene and are released when it unloads.
pub struct GameAssets {
    pub font_atlas: TextureHandle,
    pub ground_texture: TextureHandle,
    pub wall_texture: TextureHandle,
    pub platform_texture: TextureHandle,
    pub player_texture: TextureHandle,
    pub player_mesh: MeshHandle,
    pub gem_mesh: MeshHandle,
}

/// Every sound the game plays, by the name gameplay code and scripts use.
pub const SOUND_FILES: [(&str, &str); 7] = [
    ("jump", "sounds/jump.wav"),
    ("land", "sounds/land.wav"),
    ("pickup", "sounds/pickup.wav"),
    ("menu_move", "sounds/menu_move.wav"),
    ("menu_select", "sounds/menu_select.wav"),
    ("reset", "sounds/reset.wav"),
    ("theme", "music/theme.wav"),
];

fn load_texture_or(
    directory: Option<&Path>,
    relative_path: &str,
    fallback: impl FnOnce() -> TextureData,
) -> TextureData {
    if let Some(directory) = directory {
        match TextureData::load_png(&directory.join(relative_path)) {
            Ok(texture) => return texture,
            Err(error) => log::warn!(target: "assets", "{error}; using a generated texture"),
        }
    }
    fallback()
}

fn load_mesh_or(directory: Option<&Path>, relative_path: &str, fallback: impl FnOnce() -> MeshData) -> MeshData {
    if let Some(directory) = directory {
        match load_gltf_mesh(&directory.join(relative_path)) {
            Ok(mesh) => return mesh,
            Err(error) => log::warn!(target: "assets", "{error}; using a generated mesh"),
        }
    }
    fallback()
}

impl GameAssets {
    pub fn load(directory: Option<&Path>, renderer: &mut GpuRenderer) -> Result<GameAssets, GpuError> {
        let checker = |first: u32, second: u32| {
            move || TextureData::checker(64, 2, Color::from_hex(first), Color::from_hex(second))
        };
        let ground = load_texture_or(directory, "textures/ground.png", checker(0x4D6B42, 0x3E5735));
        let wall = load_texture_or(directory, "textures/wall.png", checker(0x8C857A, 0x736C63));
        let platform = load_texture_or(directory, "textures/platform.png", checker(0xCC8C40, 0xAD7333));
        let player = load_texture_or(directory, "textures/player.png", || TextureData::solid(Color::WHITE));
        let gem = load_mesh_or(directory, "models/gem.glb", || primitives::gem(0.25, 0.5));
        let cylinder_half_height = PLAYER_HEIGHT * 0.5 - PLAYER_RADIUS;
        Ok(GameAssets {
            font_atlas: renderer.create_texture(&text::font_atlas())?,
            ground_texture: renderer.create_texture(&ground)?,
            wall_texture: renderer.create_texture(&wall)?,
            platform_texture: renderer.create_texture(&platform)?,
            player_texture: renderer.create_texture(&player)?,
            player_mesh: renderer.create_mesh(&primitives::capsule(PLAYER_RADIUS, cylinder_half_height, 24, 8))?,
            gem_mesh: renderer.create_mesh(&gem)?,
        })
    }
}

/// Loads every sound in `SOUND_FILES`. Missing sounds are logged and stay silent.
pub fn load_sounds(directory: Option<&Path>, audio: &mut AudioSystem) {
    let Some(directory) = directory else { return };
    for (name, relative_path) in SOUND_FILES {
        audio.load_sound(name, &directory.join(relative_path));
    }
}
