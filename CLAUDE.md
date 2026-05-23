# Sonnensystem in Rust/wgpu

## Projektübersicht

3D-Sonnensystem-Simulation in Rust, aufbauend auf einem Java/2D-Vorläufer. Ziel ist eine vollständige Echtzeit-Simulation mit physikalisch korrekter Beleuchtung, Texturen und perspektivischer Kamera – nativ und im Browser via WASM lauffähig.

**Aktueller Stand:** Erde und Mond als UV-Kugeln mit NASA-Texturen, orbitaler Animation, Tiefenpuffer, FPS-Kamera (WASD + Maus), egui-Overlay zur Laufzeitsteuerung, togglebare Gitterebenen, Wireframe-Modus und miette-formatierte WGSL-Shader-Fehler.

## Technologie-Stack

- **Sprache:** Rust (Edition 2024)
- **Grafik-API:** wgpu 29 (nativ + WASM/Browser)
- **Shader:** WGSL (Validierung via naga mit miette-Fehlerausgabe)
- **Mathematik:** glam (Vec3, Mat4)
- **UI-Overlay:** egui 0.34 (egui-wgpu + egui-winit)
- **Fehlerbehandlung:** miette
- **Zeit:** web-time (nativ + WASM kompatibel)
- **Parallelreferenz:** Java/LWJGL-Projekt (Buch: "Beleuchtung und Rendering", 3. Auflage)

## Bereits umgesetzt

- wgpu-Grundgerüst mit winit (ApplicationHandler-API)
- Vertex Buffer + Index Buffer
- UV-Kugel-Tessellierung per CPU (`mesh::Mesh::sphere`, konfigurierbare Meridian/Parallel-Auflösung)
- Tiefenpuffer (Depth Buffer, `texture::Texture::create_depth_texture`)
- Textur-Mapping: `texture::Texture` in eigenem Modul, RGBA8/JPEG-Upload via `queue.write_texture`
- Texture Bind Group (group 0): Sampler + TextureView im Fragment-Shader
- Laufzeit-Asset-Lader (`src/loader.rs`): funktioniert nativ (Dateisystem) und im Browser (Fetch-API)
- NASA-Texturen: Erde (`2k_earth_daymap.jpg`) und Mond (`2k_moon.jpg`)
- `Camera`-Enum (`Camera::Fps(FpsCamera)`) mit separatem `Projection`-Struct
- `FpsCameraController`: WASD/Pfeiltasten, Maus-Look (Left-Click + Drag), Scroll-Zoom, Space/Shift für Y-Bewegung
- Kamera-Uniform Buffer (group 1), Update per Frame via `queue.write_buffer`
- Model-Uniform Buffer (group 2): pro Körper eine 4×4-Modellmatrix (Orbital + Spin + Scale)
- `CelestialBody` mit `OrbitalParameters` (Eltern-ID, Radius, Winkelgeschwindigkeit)
- `Scene` mit `Vec<CelestialBody>` und `BodyId`-Newtype; Eltern müssen vor Kindern eingefügt werden
- Orbitale Hierarchie: Transforms werden per topologisch-sortierter Iteration kumuliert (Mond umkreist Erde)
- `sim_time: f64`, pausierbar (P), Geschwindigkeitsfaktor (PageUp/Down/0)
- Wireframe-Modus (Tab), eigene Pipeline mit `PolygonMode::Line` (nur nativ)
- Koordinatengitter für XZ/XY/YZ-Ebenen (`src/grid.rs`), eigene Pipeline + Shader, toggle via G-Taste
- egui-Overlay: FPS, Zeit-Faktor-Buttons, Pause, Wireframe, Grid-Checkboxen, Kamera-Position und -Regler
- Shader-Validierung via naga mit source-span-genauen miette-Fehlermeldungen (`src/shader_loader.rs`)
- WASM-Pfad: WebGL-Backend, `wasm-bindgen`, Canvas-Integration via `index.html`

## Dateistruktur

```
src/
  lib.rs              – State, App, ApplicationHandler, CameraUniform, Render-/Grid-Pipeline
  main.rs             – Einstiegspunkt (nativ)
  camera.rs           – Camera-Enum, FpsCamera, Projection, FpsCameraController
  celestial_body.rs   – CelestialBody, OrbitalParameters, ModelUniform, DrawCelestialBody-Trait
  grid.rs             – ColorVertex, GridMesh (XZ/XY/YZ), DrawGrid-Trait
  loader.rs           – load_bytes / load_str (nativ: Dateisystem, WASM: Fetch-API)
  mesh.rs             – Vertex, Mesh (sphere, pentagon, planes), DrawMesh-Trait
  scene.rs            – Scene, BodyId, hierarchische Transform-Berechnung
  shader_loader.rs    – validate_wgsl (naga), make_shader_module, WgslError (miette::Diagnostic)
  texture.rs          – Texture::from_bytes / create_depth_texture
  shaders/
    shader.wgsl       – Vertex- und Fragment-Shader (UV-Textur + Kamera + Modell-Uniform)
    grid.wgsl         – Gitter-Shader (ColorVertex, unlit)
assets/textures/
  dbg.png             – Debug-Textur
  2k_earth_daymap.jpg – NASA-Erdtextur
  2k_moon.jpg         – NASA-Mondtextur
```

## Build & Entwicklung

```sh
just run        # nativ bauen und starten
just serve      # WASM bauen + Python-HTTP-Server auf :8080
just san        # cargo fmt + clippy -D warnings
```

## Tastenbelegung

| Taste | Funktion |
|---|---|
| WASD / Pfeiltasten | Kamera bewegen |
| Maus (LMB + Drag) | Kamera drehen |
| Space / Shift | Hoch / Runter |
| Scroll | Zoom (Kamera vorwärts) |
| P | Simulation pausieren |
| PageUp / PageDown | Zeit-Faktor ×2 / ÷2 |
| 0 | Zeit-Faktor zurücksetzen |
| Tab | Wireframe-Modus (nur nativ) |
| G | Alle Gitter umschalten |
| Escape | Beenden |

## Phasenplan

### Phase 1 – Fundament ✅ (bis auf offene TODOs)
- [x] Uniforms & MVP-Transformationsmatrizen (View + Projection via glam)
- [x] Index Buffer
- [x] Perspektivkamera hinter `enum Camera { Fps(FpsCamera) }`
- [x] Textur-Mapping (UV-Koordinaten, Bind Group)
- [x] UV-Kugel tessellieren (Vertices per CPU aus Breiten-/Längengraden)
- [x] Tiefenpuffer (Depth Buffer)
- [x] `sim_time` von `f32` auf `f64` umgestellt (State, CelestialBody, Scene)
- [ ] **TODO (ggf. vorziehen):** Orbitalkamera als `Camera::Orbit(BodyId)` – Kamera folgt einem Himmelskörper
- [x] Mobile-Navigation: Touch-Buttons unten links (vor/zurück/links/rechts, hoch/runter, Zoom) für Bedienung ohne Tastatur
- [x] Orbit-Kamera-Reset: Standard-Pitch 45° nach unten

### Phase 2 – Sonnensystem 3D (teilweise)
- [x] `struct CelestialBody` mit `OrbitalParameters` – Szene als querybare Liste (`Scene` + `BodyId`)
- [x] Orbitale Animation gegen abstrakte `sim_time` – pausierbar, skalierbar
- [x] Textur-Mapping mit NASA-Texturen (Erde + Mond)
- [ ] Sonne als dritter Körper (Mittelpunkt, Emissivmaterial, kein Elternteil)
- [ ] Korrekte relative Größenverhältnisse (Sonne >> Erde > Mond)
- *Buchbezug: Kap. 7 – Texture-Mapping*

### Phase 3 – Beleuchtung
- Phong-Beleuchtungsmodell im Fragment-Shader (WGSL): ambient, diffus, spekular
- Sonne als Positionslicht; emissives Material für die Sonne selbst
- Tag-/Nachtseite auf Erde und Mond sichtbar
- *Buchbezug: Kap. 5 – Beleuchtungsmodelle, Kap. 6 – Shading*

### Phase 4 – Polish (optional)
- Normal-Map auf der Erde (Gebirge/Ozeane)
- Atmosphären-Glow für die Erde (Post-Processing)
- Sternenhintergrund (Skybox oder Billboard-Punkte)
- Bloom-Effekt für die Sonne
- *Buchbezug: Kap. 8 – Normal-Mapping, Kap. 13 – Displacement-Mapping*

### Phase 5 – Echte astronomische Daten (ggf. vorziehen)
- **Planetenpositionen via VSOP87** (Rust-Crate `vsop87`, kein Live-API nötig) – ggf. früher einbauen
- Sternenhintergrund aus HYG-Datenbank (~120.000 Sterne als Punktwolke, Farbe aus Spektralklasse)
- Echtzeit-Simulation: Datum/Uhrzeit als Eingabe, Zeitraffer steuerbar
- Logarithmische oder benutzerdefinierte Skalierungsstrategie für echte Abstände

## Architektur-Leitlinien

- **Kamera:** `Camera`-Enum erlaubt spätere `Orbit(BodyId)`-Variante ohne Umbau des Controllers
- **Szenen-Datenstruktur:** `BodyId`-Newtype schützt vor falscher Indizierung; Eltern müssen immer vor Kindern in `celestial_bodies` stehen (debug_assert prüft das)
- **Simulationszeit:** Immer `sim_time: f64` verwenden, niemals `std::time::Instant` direkt in Orbitlogik – ermöglicht Pause und Zeitraffer ohne Refactoring
- **Shader-Fehler:** Neue Shader immer über `shader_loader::validate_wgsl` laufen lassen, bevor `create_shader_module` aufgerufen wird – liefert source-span-genaue miette-Fehler statt wgpu-Panic
- **WASM-Kompatibilität:** `loader::load_bytes/load_str` für alle Assets verwenden; kein `std::fs` direkt; kein Threading ohne `wasm-bindgen-rayon`

## Konventionen

- Shader-Dateien: `*.wgsl` in `src/shaders/`
- Texturen: `assets/textures/` (NASA-Quellen bevorzugen, lizenzfrei)
- GLSL-Konzepte aus dem Buch 1:1 in WGSL übertragen; Unterschiede dokumentieren
- Alle `pub`-Items bekommen einen `///`-Doc-Comment in Englisch
- Neue Phasen erst beginnen, wenn alle Checkboxen der aktuellen Phase abgehakt sind
