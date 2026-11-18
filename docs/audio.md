# Audio

Lantern mixes its own sound. The engine keeps a software mixer, fills a short queue on the audio device once a frame, and
gives the game a small set of calls in `lantern:::audio`. The game decides what plays and when; the engine decides how it
sounds from where the listener is. No sound is recorded: every placeholder is generated from arithmetic, and a file in the
right place replaces it.

## The pieces

| Piece | File | What it does |
| --- | --- | --- |
| `Mixer` | `engine/src/audio/Mixer.nox` | Voices, buses, the listener, fades, and `Mix`, which makes the next block of stereo samples. Knows nothing about devices. |
| `Synth` | `engine/src/audio/Synth.nox` | The generated placeholder sounds (`lantern:::synth`). |
| `AudioSystem` | `engine/src/engine/audio/AudioSystem.nox` | Sounds and named loops by name, bus volumes by name, and `Update`, which keeps about 60 ms queued on the device. |
| `SdlAudioOutput` | `engine/src/sdl-facade/audio/SdlAudioOutput.nox` | The audio sink over SDL3. No device is not an error: the game runs silent. |
| `AudioApi` | `engine/src/host/AudioApi.nox` | Registers `lantern:::audio` for Nyx. |

There is no audio callback thread. Each frame `AudioSystem.Update` asks the mixer for just enough frames to fill the queue
and hands them to SDL, which plays them on its own thread.

## Buses

A bus is a group of sounds with one volume and one mute switch. There are four, and `master` over all of them.

| Bus | Name | Holds | Default volume |
| --- | --- | --- | --- |
| music | `music` | the music stems | 0.5 |
| ambience | `ambience` | wind, rain, the city and station beds, anything that loops under the scene | 0.6 |
| effects | `sfx` | footsteps, jumps, pickups, anything the world makes | 0.9 |
| interface | `ui` | menu sounds and the photo shutter | 0.9 |
| master | `master` | scales the other four | 0.8 |

```nyx
lantern:::audio:::SetBusVolume("music", 0.3f);
lantern:::audio:::SetBusMuted("ambience", true);
```

A muted bus keeps its volume, so unmuting restores it. A bus name that isn't one of the five is ignored. A volume change
takes effect smoothly: the mixer ramps every voice's gain across each block, so turning a bus down never clicks.

## Sounds and where they come from

`Load(name, path)` reads a WAV file (16-bit PCM, mono or stereo, any rate; the mixer resamples to the device's
rate) and stores it under `name`. When the file isn't there, `Load` looks for a **generated sound with the same name**
and uses that instead, so a checkout with no recorded audio still has a full set of sounds. A name with no file and no
generated sound is logged and stays silent.

### Generated sounds

`lantern:::synth` builds each sound from sine tones, shaped noise, an envelope and a one-pole filter. The same name gives
the same samples every time, on every machine.

| Name | What it is |
| --- | --- |
| `footstep_stone`, `footstep_metal`, `footstep_wood`, `footstep_wet`, `footstep_grass` | one footfall on each surface, about a quarter of a second |
| `ambience_wind`, `ambience_rain`, `ambience_city`, `ambience_station`, `ambience_market` | 8-second beds that loop with no seam |
| `music_calm`, `music_tension`, `music_companion` | 8-second stems of one piece: a slow pad, a low drone with a clashing note and a pulse, and a soft figure that plays when the partner is near |
| `ui_move`, `ui_select`, `ui_back`, `ui_error`, `ui_shutter` | menu sounds and the photo shutter |

Beds are set to an average loudness of 0.05 and stems to 0.07 (not to a peak, which makes noise a loud hiss), and the mixer's limiter passes anything under 0.8 untouched and rounds only the excess toward 1.

Loops are made seamless two ways. Every tone in a bed or stem is moved to a whole number of cycles per loop, so it is at
the same phase at the end as at the start, and noise beds have their last second mixed into their first. All music stems
are the same length and start in the same frame, so they stay in step.

### Swapping in real recordings

1. Record or license a sound, and save it as a WAV file.
2. Put it where the game loads that name from. For the platformer, `games/platformer/game-assets/game-assets.nyx` lists
   each name and its path, for example `lantern:::audio:::Load("footstep_stone", "sounds/footstep_stone.wav");`, so the
   file goes at `games/platformer/assets/sounds/footstep_stone.wav`.
3. Run the game. The file wins; the generated sound is used only when the file can't be read.

For a loop, make the file loop cleanly (the end must join the start), and for music stems that play together, make every
stem the same length. For footsteps give a few variations in separate names (`footstep_stone_2`) and choose between them
in the game script; the game already varies pitch and volume a little on each step.

## Playing sounds

### One-shots

| Call | Use |
| --- | --- |
| `Play(name, volume)` | a flat sound on the `sfx` bus |
| `PlayOn(name, bus, volume, pitch)` | a flat sound on another bus; pitch 1 is normal, 2 is an octave up, and it is limited to a quarter to four times |
| `PlayAt(name, bus, position, volume, pitch, minDistance, maxDistance)` | a sound at a point in the world |

### Positional sound

`SetListener(position, forward, up)` says where the camera is; the game calls it every frame. A positional sound is then heard relative to the listener:

- **Distance.** Full volume inside `minDistance`, silent beyond `maxDistance`, and between them a smooth fall: the square
  of a straight line from 1 down to 0. Defaults are 2 m and 30 m. A tall, loud thing (a train) takes a larger range; a
  footstep a smaller one.
- **Pan.** The sideways part of the direction to the sound, against the listener's right-hand direction (forward crossed
  with up), gives -1 (all left) to 1 (all right). The gains are equal-power, so the sound doesn't dip as it crosses the
  middle. Straight ahead and straight behind both sound central.
- **Mono.** A positional sound is mixed down to one channel before panning, as a point in the world has one position.

Not modelled: height, a duller sound behind the listener, Doppler, and walls getting in the way. These are listed in
`known-gaps.md`.

### Loops: layers and emitters

A loop has a name. A **layer** is flat (ambience, music); an **emitter** is at a point (a humming lamp, a stall).

```nyx
lantern:::audio:::StartLayer("ambience:wind", "ambience_wind", "ambience", 1.0f, 0.0f);
lantern:::audio:::FadeLayer("ambience:wind", 1.0f, 2.0f);
lantern:::audio:::StartEmitter("lamp:3", "ambience_city", "ambience", lampPosition, 1.0f, 2.0f, 18.0f);
lantern:::audio:::StopLayer("ambience:wind", 3.0f);
```

- `StartLayer` and `StartEmitter` do nothing when the name is already playing, so a game can ask every frame.
- `level` goes from 0 to 1 and is separate from `volume`: `volume` is how loud the sound is, `level` is the fade.
  A layer started at level 0 is silent until faded in.
- `FadeLayer(name, level, seconds)` moves the level in a straight line. A crossfade is two fades at once: one layer to 0,
  the other to 1.
- `StopLayer(name, seconds)` fades out over that time and then frees the voice. A time of 0 stops at once.
- `MoveEmitter` moves an emitter; the change is ramped, so a moving source doesn't click.
- `LayerLevel(name)` and `IsPlaying(name)` read a loop back.

The mixer plays up to 64 voices. When all are busy the oldest one-shot is dropped; loops are never dropped for a new
sound.

## In the platformer

Two systems in `games/platformer/game-systems/` use the calls above. The engine knows nothing of them.

- **`AudioDirector`** (`audio-director.nyx`) runs every frame from `OnRender`. It sets the listener from the camera,
  starts the level's ambience beds (`city`, `rain`, `wind`, `station` or `market`, chosen by the `ambience` property on the
  level's spawn) and crossfades from the last level's over 3 seconds. It also eases four music layers: `theme` in the
  menus, `calm` while playing, `tension` while a watcher's beam is on the player, and `companion` while the partner is
  between 6 and 14 m away. Pausing ducks the music.
- **`FootstepSystem`** (`footstep-system.nyx`) plays `footstep_<surface>` at each walker's feet once per stride. The
  surface comes from the spawn's `surface` property, else the chapter's default. A block with `surface` and
  `surface_radius` patches the ground near it.
- **UI sounds**: menu buttons play `ui_move` on focus and `ui_select` on press, pausing plays `ui_select` and resuming
  `ui_back`, and the photo shutter plays `ui_shutter`, then `pickup` when a sight is ready or `ui_error` when not.
- `StopAllLayers(seconds)` ends every named loop; leaving the game calls it, and so does a new session.

`LANTERN_DEBUG_HUD` and the debug values `Audio/Ambience`, `Audio/Theme`, `Audio/Calm`, `Audio/Tension`,
`Audio/Companion`, `Audio/Surface` and `Audio/Footsteps` show what the director and footsteps are doing.

`menu_move.wav` and `menu_select.wav` are no longer loaded; use `ui_move.wav` and `ui_select.wav`. Every generated name
is loaded by `LoadGeneratedSounds` in `game-assets/game-assets.nyx`, so a file at `games/platformer/assets/sounds/<name>.wav` replaces it.

## Testing without a device

No test needs a sound card.

- `Mixer` tests call `Mix(frames)` and look at the samples (`engine/tests/audio/`): buses, falloff and pan, fades, pitch,
  the generated sounds.
- `AudioSystem` with no sink returns from `Update` before it mixes, so voices do not advance there; a headless test calls `Mixer.Mix` itself to advance fades and stops (`engine/tests/engine/`).
- `engine/tests/host/the-music-and-ambience-follow-the-game` runs the game's director and footsteps on a headless host.
- The SDL output is opened with the `dummy` audio driver (`engine/tests/sdl/audio-output-queues-pushed-frames/`), which
  accepts samples and discards them.

To hear the result, run the game on a machine with a sound card (`mist run game`). Set `LANTERN_DEBUG_HUD=1` for the
audio line: whether a device is open, how many voices are playing, how much is queued and how many times the queue ran dry.

To find which kind of sound a problem is in, mute buses for one run: `LANTERN_AUDIO_MUTE=ambience,music mist run game` leaves only footsteps and menu sounds. The names are `master`, `music`, `ambience`, `sfx` and `ui`.
