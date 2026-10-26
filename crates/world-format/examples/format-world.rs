//! Rewrites world files in the canonical layout `WorldFile::to_json` produces, after checking
//! them. Run after editing a world by hand:
//!
//! ```sh
//! cargo run -p lantern-world-format --example format-world -- assets/worlds/course.world.json
//! ```

use std::path::Path;
use std::process::ExitCode;

use lantern_world_format::WorldFile;

fn main() -> ExitCode {
    let paths: Vec<String> = std::env::args().skip(1).collect();
    if paths.is_empty() {
        eprintln!("usage: format-world <file.world.json>...");
        return ExitCode::FAILURE;
    }
    let mut failed = false;
    for path in &paths {
        let path = Path::new(path);
        match WorldFile::load(path).and_then(|world| world.save(path)) {
            Ok(()) => println!("formatted {}", path.display()),
            Err(error) => {
                eprintln!("error: {error}");
                failed = true;
            }
        }
    }
    if failed { ExitCode::FAILURE } else { ExitCode::SUCCESS }
}
