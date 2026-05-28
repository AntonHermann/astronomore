# Astronomore

A real-time 3D solar system simulation built with Rust and wgpu. Runs natively and in the browser via WebAssembly.

All 8 planets + Moon orbit the Sun using VSOP87 astronomical ephemerides. Blinn-Phong lighting makes the day/night terminator visible on each body. Camera modes include free-flight (FPS) and per-body orbital tracking.

## Features

- **10 celestial bodies** – Sun, Mercury, Venus, Earth, Moon, Mars, Jupiter, Saturn, Uranus, Neptune
- **Accurate orbits** – VSOP87 heliocentric ephemerides for planets; parametric circular orbit for the Moon
- **Blinn-Phong lighting** – ambient, diffuse, specular; Sun as positional light source; day/night terminator visible
- **NASA textures** – 2 k texture maps for every body
- **Two camera modes** – FPS (free-flight) and Orbit (tracks any body)
- **Mobile-friendly** – on-screen touch navigation buttons; tested on Android via WebGL
- **Real-time controls** – pause, ×2/÷2 speed, date display, tessellation sliders, lighting parameter tuning
- **Wireframe & normal overlays** – toggleable debug views (wireframe native-only)
- **Cross-platform** – native (Linux / macOS / Windows) + WASM/WebGL via wasm-pack
- **Shader hot-reload** – native build watches `src/shaders/` and reloads on save

## Tech Stack

| Area | Crate |
|------|-------|
| GPU / rendering | wgpu 29, WGSL shaders |
| Windowing | winit 0.30 |
| Math | glam 0.32 |
| UI overlay | egui 0.34 (egui-wgpu + egui-winit) |
| Astronomy | vsop87 3 |
| Error diagnostics | miette 7 |
| Logging | tracing |
| WASM bridge | wasm-bindgen, web-sys |

## Build & Run

```sh
just run        # native build + launch
just serve      # WASM build + Python HTTP server on :8080
just san        # cargo fmt + clippy -D warnings
```

Requires Rust (edition 2024). For WASM: `wasm-pack` and Python 3.

## Controls

| Input | Action |
|-------|--------|
| WASD / Arrow keys | Move camera |
| Mouse (LMB + drag) | Look around |
| Scroll | Zoom |
| Space / Shift | Move up / down |
| P | Pause simulation |
| PageUp / PageDown | Time speed ×2 / ÷2 |
| 0 | Reset time speed |
| Tab | Wireframe mode (native only) |
| N | Toggle normals overlay |
| G | Toggle all grid planes |
| Escape | Quit |

On mobile / touch: use the on-screen navigation buttons in the bottom-left corner.

## Project Structure

```
src/
  lib.rs              – State, App, render loop, event handling
  main.rs             – Native entry point
  camera.rs           – Camera enum (Fps / Orbit), controller, projection
  celestial_body.rs   – CelestialBody, OrbitalParameters, ModelUniform
  scene.rs            – Scene, BodyId newtype, hierarchical transforms
  orbital.rs          – OrbitalModel (Fixed / Parametric / Vsop87), time conversion
  planets.rs          – 10-body solar system definition (BodyDef array)
  sim.rs              – SimState (time, multiplier, pause)
  scene_properties.rs – Lighting uniform (Blinn-Phong parameters)
  ui.rs               – ViewOptions, EguiLayer
  gpu.rs              – GPU context, surface, device, queue
  pipelines.rs        – Render pipelines (fill, wireframe, normals, grid)
  mesh.rs             – Vertex, UV-sphere tessellation
  texture.rs          – Texture upload, depth buffer
  grid.rs             – Grid mesh (XZ / XY / YZ planes)
  loader.rs           – Asset loader (native FS + WASM Fetch API)
  shader_loader.rs    – WGSL validation via naga (miette diagnostics)
  shaders/
    shader.wgsl       – Main vertex + fragment shader (Blinn-Phong + textures)
    grid.wgsl         – Grid shader (unlit)
    normals.wgsl      – Normal vector visualisation
assets/textures/      – NASA 2 k texture maps for all 10 bodies
```
