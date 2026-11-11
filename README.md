# Gummy Grove

A first-person shooter set in a gummy-bear favela. They have guns. You have jellybeans.

This is a native macOS game written in Rust on a custom Metal renderer. Metal Performance Shaders (MPS) does real work every frame:

- `MPSRayIntersector` + `MPSTriangleAccelerationStructure`: ray-traced soft sun shadows and ambient occlusion
- `MPSImageGaussianBlur`: the bloom chain
- `MPSImageLanczosScale`: upscaling the internal image to the window

The gummy bears are ray-marched signed-distance shapes with absorption, refraction and light scattering through the jelly.

## Run

Needs macOS on Apple silicon and a Rust toolchain.

```bash
cargo run --release --locked
```

## Controls

| Key | Action |
| --- | --- |
| WASD | move |
| Shift | sprint |
| C | crouch, or slide while moving |
| Space | jump, climb ledges |
| Click / right click | fire / aim down sights |
| V | melee |
| E | pick up dropped weapons and armor |
| 1 2 3 4, Q, wheel | weapons |
| R | reload |
| F | Sugar Rush (bullet time) when the meter is full |
| Esc | pause |
| F11 / F3 | fullscreen / frame-rate readout in the title bar |

Progress saves to `~/Library/Application Support/GummyGrove/save.json`.

## Self-tests

All of these use a throwaway save file, never the real one.

```bash
cargo run --release --locked -- --bot
```

Plays all eight missions and nine arcade waves with an aim-bot, without a window, and reports the outcome of each.

```bash
cargo run --release --locked -- --uitest --rich
```

Clicks through the menus (briefing, deploy, pause, settings, armory purchase, arcade).

```bash
cargo run --release --locked -- --smoke 300
```

Opens the real window, plays 300 frames and prints frame timings.

```bash
cargo run --release --locked -- --shot out.png --mission 3 --t 10 --play
```

Renders one frame to a PNG. Other flags: `--screen map|brief|armory|settings|pause|results`, `--intro`, `--ads`, `--gun N`, `--give ak|pistol|sniper|rpg|minigun`, `--lineup` (a row of bears in every colour), `--pal morning|sunset|overcast|night`, `--noui`, `--w/--h/--scale`, `--cam --x --y --z --yaw --pitch --fov`.

## Layout

| Path | What |
| --- | --- |
| `src/gfx.rs`, `src/shaders.metal` | renderer: G-buffer, MPS ray tracing, lighting, water, gummies, TAA, bloom, tone mapping, UI |
| `src/world.rs`, `src/tex.rs`, `src/mesh.rs` | the favela, procedural materials and decals, mesh kit |
| `src/guns.rs`, `src/bear.rs` | weapon and gear models, gummy bear poses |
| `src/game.rs`, `src/game/hud.rs` | movement, combat, bear AI, missions, HUD and menus |
| `src/meta.rs` | campaign, upgrades, perks, save file |
| `src/audio.rs` | synthesised sound effects and adaptive music |
| `web/` | the original three.js version, kept for reference |

Dependencies are locked with a seven-day minimum release age; see `Cargo.lock`.
