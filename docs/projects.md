# Game projects

A project is a game that lives in its own folder, apart from the engine, and links to it. You make one, work in it, run it,
edit its world and package it from that folder. The engine checkout stays a place you update, not a place you keep your game.
The platformer in `games/platformer/` is the sample project the platformer template copies.

## Make one

With `mist run install-cli` once (it links `engine/tools/lantern` to `~/.local/bin/lantern`), from anywhere:

```sh
lantern project new ~/games/my-game --name "My Game"                 # the empty template: a floor, a sky and a camera
lantern project new ~/games/my-platformer --template platformer      # a copy of the sample game
cd ~/games/my-game
lantern run .          # play it
lantern editor .       # edit its world
lantern package .      # build ./build/package/lantern, one file to give to players
```

Without installing the command, `engine/tools/lantern` in the checkout does the same, and inside a project the `mist.json`
scripts (`mist run game`, `mist run editor`, `mist run package`, `mist run info`) call it.

## The folder

```text
my-game/
  lantern.project.json   the project's name, template, engine link and creation date (never packaged)
  game.json              the engine's manifest: window title, entry Nyx file, application class, assets folder
  main.nyx               the game; more Nyx files, UI and settings beside it as the game grows
  assets/                textures, models, sounds, music, worlds, and the Calamus files: Silhouettes, material
                         Silhouettes, the editor's palette
  mist.json              scripts: game, editor, package, info (each runs `lantern ... .`)
  README.md  .gitignore
```

The folder is also a game folder as the engine reads it, so `lantern <folder>` and `mist run package <folder>` work on
any project. Everything a game is made of stays in it, Calamus files included: the engine folder holds none of a project's
content, and a packaged game never contains a Calamus file (the build bakes the project's material Silhouettes and graphs into
Nyx first).

`lantern.project.json`:

```json
{
  "format_version": 1,
  "name": "My Game",
  "template": "empty",
  "engine": "../lantern",
  "created": "2026-11-21"
}
```

`engine` is the path to a Lantern checkout, relative to the project's folder (or absolute). A folder with `game.json` but no
project file is still a game the engine runs; it just isn't a project.

## Which engine a project uses

In order: the `LANTERN_ENGINE` environment variable, the project's `engine` path, then the checkout the `lantern` command
itself lives in. The engine checkout needs `mist install` run once, as for any use of Lantern (`docs/running.md`); a project
needs no install of its own. `lantern project relink PATH` changes the recorded path, for when the checkout moves. A project
does not pin an engine version: it uses whatever is in the folder it names.

## Commands

| Command | Does |
| --- | --- |
| `lantern project new DIRECTORY [--name NAME] [--template T]` | makes a project in a new or empty folder; the name defaults to the folder's name |
| `lantern project templates` | lists the templates: `empty`, `platformer` and any folder in `engine/templates/` |
| `lantern project info [DIRECTORY]` | prints the name, folder, template, creation date and resolved engine |
| `lantern project check [DIRECTORY]` | reports what's wrong: no manifest, a missing entry file or assets folder, an engine folder that isn't a checkout |
| `lantern project rename [DIRECTORY] NAME` | changes the project's name and the game's window title |
| `lantern project relink [DIRECTORY] ENGINE_PATH` | points the project at another engine checkout |
| `lantern project duplicate SOURCE DESTINATION [--name NAME]` | copies a project to a new folder under a new name |
| `lantern project recent` | lists the projects opened lately, newest first |
| `lantern run\|editor\|package [DIRECTORY]` | plays, edits or packages the project (default: the current folder) |
| `lantern engine [DIRECTORY]` | prints the engine checkout the project resolves to |

`new` and `duplicate` add the project to the recent list. The list is `recent-projects.json` in the per-user config folder
(`~/Library/Application Support/lantern` on macOS, `$XDG_CONFIG_HOME/lantern` or `~/.config/lantern` elsewhere;
`LANTERN_CONFIG_DIRECTORY` overrides it). It keeps ten, and a project whose folder is gone drops off.

## Templates

- **empty** (`engine/templates/empty/`): a floor and three blocks in a world file, a sun, a sky and a camera you fly with
  WASD, Q and E, Shift and Space. `main.nyx` is written as a tutorial, line by line.
- **platformer**: a copy of `games/platformer/`, taken when you make the project, so it always matches the engine checkout.

A template is a folder: drop another into `engine/templates/` and `lantern project templates` lists it. Copying leaves out
editor scratch files (`.editor-play*`), `build/`, `mist_packages/`, `.git`, and `engine-host-surface.json`.

## Where the code is

`engine/src/project/` is the library the command line (and the editor) use: `ProjectManifest.nox` (the project file),
`Projects.nox` (templates, copying, creating), `ProjectOperations.nox` (check, rename, relink, duplicate),
`RecentProjects.nox`, `ProjectPaths.nox` and `ProjectCommands.nox` (`lantern project ...`). Tests are in
`engine/tests/project/`. The decisions are D-250 to D-256 in `decisions.md`.

## In the editor

The world editor works on one project at a time. `lantern editor FOLDER` (or `mist run editor FOLDER` inside a project)
opens it on that project's assets. The File menu has:

- **New Project…**: asks for a name, a parent folder (Browse… opens a folder picker) and a template, then makes the folder
  and switches the editor to it.
- **Open Project…**: a folder picker; the folder must hold a `game.json`.
- **Recent: NAME**: the five projects opened last. A folder that has gone is dropped from the list.
- **Project Settings…**: rename, relink the engine, check and duplicate the project.

Switching asks about unsaved changes first, then closes the window and opens a new one on the other project.

## Not done yet

- Pinning an engine version, and an installed engine for people who don't keep a checkout (both open questions on the
  epic).
- Templates kept in the user's config folder.
- Windows: paths are handled with `/` only.
