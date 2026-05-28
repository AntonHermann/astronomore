# Sonnensystem in Rust/wgpu

## Projektübersicht

3D-Sonnensystem-Simulation in Rust, aufbauend auf einem Java/2D-Vorläufer. Ziel ist eine vollständige Echtzeit-Simulation mit physikalisch korrekter Beleuchtung, Texturen und perspektivischer Kamera – nativ und im Browser via WASM lauffähig.

**Aktueller Stand (Mai 2026):** Vollständiges Sonnensystem mit 10 Körpern (Sonne, 8 Planeten, Mond). Planeten bewegen sich auf VSOP87-Ephemeriden-Bahnen; Mond auf Kreisbahn um die Erde. Blinn-Phong-Beleuchtung aus der Sonne (Tag-/Nachtseite sichtbar), NASA-2k-Texturen für alle Körper, korrekte relative Größenverhältnisse. Zwei Kameramodi (FPS + Orbit), Mobile-Touch-Navigation, Shader-Hot-Reload nativ, egui-Overlay für Laufzeitsteuerung (Beleuchtung, Tesselierung, Zeitsteuerung, Datumstanzeige).

## Technologie-Stack

- **Sprache:** Rust (Edition 2024)
- **Grafik-API:** wgpu 29 (nativ + WASM/Browser)
- **Shader:** WGSL (Validierung via naga mit miette-Fehlerausgabe)
- **Mathematik:** glam (Vec3, Mat4)
- **UI-Overlay:** egui 0.34 (egui-wgpu + egui-winit)
- **Astronomie:** vsop87 3 (heliozentrische Planetenpositionen)
- **Fehlerbehandlung:** miette
- **Zeit:** web-time (nativ + WASM kompatibel)
- **Parallelreferenz:** Java/LWJGL-Projekt (Buch: "Beleuchtung und Rendering", 3. Auflage)

## Bereits umgesetzt

- wgpu-Grundgerüst mit winit (ApplicationHandler-API), GPU-Kontext in `gpu.rs`
- Vertex Buffer + Index Buffer
- UV-Kugel-Tessellierung per CPU (`mesh::Mesh::sphere`, konfigurierbare Meridian/Parallel-Auflösung, Laufzeit-Regler im UI)
- Tiefenpuffer (Depth Buffer, `texture::Texture::create_depth_texture`)
- Textur-Mapping: `texture::Texture` in eigenem Modul, RGBA8/JPEG-Upload via `queue.write_texture`
- Texture Bind Group (group 0): Sampler + TextureView im Fragment-Shader
- Laufzeit-Asset-Lader (`src/loader.rs`): funktioniert nativ (Dateisystem) und im Browser (Fetch-API)
- NASA-2k-Texturen für alle 10 Körper (Sonne, Merkur, Venus, Erde, Mond, Mars, Jupiter, Saturn, Uranus, Neptun)
- `Camera`-Enum: `Camera::Fps(FpsCamera)` und `Camera::Orbit(OrbitCamera)` – Laufzeit-Umschaltung per UI
- `FpsCameraController`: WASD/Pfeiltasten, Maus-Look (Left-Click + Drag), Scroll-Zoom, Space/Shift für Y-Bewegung
- `OrbitCamera`: umkreist einen wählbaren Körper (per `BodyId`), Pitch/Yaw/Distanz-Steuerung
- Mobile-Touch-Navigation: On-Screen-Buttons unten links (vor/zurück/links/rechts, hoch/runter, Zoom)
- Kamera-Uniform Buffer (group 1), Update per Frame via `queue.write_buffer`
- Model-Uniform Buffer (group 2): pro Körper eine 4×4-Modellmatrix (Orbital + Spin + Scale)
- `CelestialBody` mit `OrbitalParameters` (Eltern-ID, `OrbitalModel`-Enum)
- `OrbitalModel`-Enum: `Fixed` (Sonne), `Parametric` (Kreisbahn, Mond), `Vsop87` (Ephemeriden, Planeten)
- `Scene` mit `Vec<CelestialBody>` und `BodyId`-Newtype; Eltern müssen vor Kindern eingefügt werden
- Orbitale Hierarchie: Transforms werden per topologisch-sortierter Iteration kumuliert
- `planets.rs`: deklaratives 10-Körper-Array (`BodyDef`) – Sonne, alle 8 Planeten, Mond
- `orbital.rs`: VSOP87-Anbindung, `sim_time_to_jde()`, `jde_to_gregorian()`, `gregorian_to_jde()`, `jde_to_sim_time()` (Meeus-Algorithmus, bidirektional)
- Blinn-Phong-Beleuchtung im Fragment-Shader: ambient + diffus + spekular; Sonne als Punktlicht
- `ScenePropertiesUniform` (group 3): Laufzeit-Beleuchtungsparameter per egui-Regler einstellbar
- Normalenvisualisierung: eigene Pipeline + Shader (`normals.wgsl`), toggle per UI
- `sim_time: f64` in `SimState` (`sim.rs`), pausierbar (P), Geschwindigkeitsfaktor (PageUp/Down/0), `jump_to_date()` für direkten Zeitsprung
- Wireframe-Modus (Tab), eigene Pipeline mit `PolygonMode::Line` (nur nativ)
- Koordinatengitter für XZ/XY/YZ-Ebenen (`src/grid.rs`), eigene Pipeline + Shader, toggle via G-Taste
- Render-Pipelines in `pipelines.rs` ausgelagert (fill, wireframe, normals, grid)
- egui-UI: separates „Time Control"-Fenster (unten rechts) mit Datumsanzeige, Zeitfaktor, Steuerbuttons, Datumseingabe (DragValue, Live-Vorschau); FPS-Overlay (rahmenlos, oben links); „Simulation"-Fenster (oben rechts) für Beleuchtungs-Regler, Tesselierungs-Regler, Kamerasteuerung, Grid-Checkboxen
- Shader-Validierung via naga mit source-span-genauen miette-Fehlermeldungen (`src/shader_loader.rs`)
- Shader-Hot-Reload nativ (notify-debouncer-mini, Datei-Watcher auf `src/shaders/`)
- WASM-Pfad: WebGL-Backend, `wasm-bindgen`, Canvas-Integration via `index.html`
- Vorberechnungs-Benchmark-System (`bench.rs`, `scripts/bench-record.py`, Pre-Commit-Hook)

## Dateistruktur

```
src/
  lib.rs              – State, App, ApplicationHandler, Render-Loop, Event-Handling
  main.rs             – Einstiegspunkt (nativ)
  camera.rs           – Camera-Enum (Fps/Orbit), FpsCamera, OrbitCamera, Projection, Controller
  celestial_body.rs   – CelestialBody, OrbitalParameters, ModelUniform, DrawCelestialBody-Trait
  scene.rs            – Scene, BodyId, hierarchische Transform-Berechnung
  orbital.rs          – OrbitalModel (Fixed/Parametric/Vsop87), sim_time_to_jde, jde_to_gregorian, gregorian_to_jde, jde_to_sim_time
  planets.rs          – SolarSystemBody-Enum, BodyDef, 10-Körper-Array (Sonne + Planeten + Mond)
  sim.rs              – SimState (time: f64, multiplier, is_paused), jump_to_date
  scene_properties.rs – ScenePropertiesUniform (Blinn-Phong-Parameter, Lichtposition/-farbe)
  ui.rs               – ViewOptions, EguiLayer
  gpu.rs              – GpuContext (Surface, Device, Queue, Adapter)
  pipelines.rs        – Render-Pipelines (fill, wireframe, normals, grid)
  grid.rs             – ColorVertex, GridMesh (XZ/XY/YZ), DrawGrid-Trait
  loader.rs           – load_bytes / load_str (nativ: Dateisystem, WASM: Fetch-API)
  mesh.rs             – Vertex, Mesh (sphere), DrawMesh-Trait
  shader_loader.rs    – validate_wgsl (naga), make_shader_module, WgslError (miette::Diagnostic)
  texture.rs          – Texture::from_bytes / create_depth_texture
  shaders/
    shader.wgsl       – Vertex- und Fragment-Shader (Blinn-Phong + UV-Textur + Uniforms)
    grid.wgsl         – Gitter-Shader (ColorVertex, unlit)
    normals.wgsl      – Normalenvisualisierungs-Shader
  bin/
    bench.rs          – Performance-Benchmark-Binary
assets/textures/
  2k_earth_daymap.jpg, 2k_moon.jpg, 2k_sun.jpg
  2k_mercury.jpg, 2k_venus_surface.jpg, 2k_mars.jpg
  2k_jupiter.jpg, 2k_saturn.jpg, 2k_uranus.jpg, 2k_neptune.jpg
  8k_earth_daymap.jpg – hochaufgelöste Erdtextur (optional)
  dbg.png             – Debug-Textur
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

### Phase 1 – Fundament ✅
- [x] Uniforms & MVP-Transformationsmatrizen (View + Projection via glam)
- [x] Index Buffer
- [x] Perspektivkamera hinter `enum Camera { Fps(FpsCamera) }`
- [x] Textur-Mapping (UV-Koordinaten, Bind Group)
- [x] UV-Kugel tessellieren (Vertices per CPU aus Breiten-/Längengraden)
- [x] Tiefenpuffer (Depth Buffer)
- [x] `sim_time` von `f32` auf `f64` umgestellt (State, CelestialBody, Scene)
- [x] Orbitalkamera als `Camera::Orbit(OrbitCamera)` – Kamera folgt einem Himmelskörper
- [x] Mobile-Navigation: Touch-Buttons unten links (vor/zurück/links/rechts, hoch/runter, Zoom) für Bedienung ohne Tastatur

### Phase 2 – Sonnensystem 3D ✅
- [x] `struct CelestialBody` mit `OrbitalParameters` – Szene als querybare Liste (`Scene` + `BodyId`)
- [x] Orbitale Animation gegen abstrakte `sim_time` – pausierbar, skalierbar
- [x] NASA-Texturen für alle 10 Körper (Sonne, 8 Planeten, Mond)
- [x] Sonne als zentraler Körper (Fixed, kein Elternteil)
- [x] Korrekte relative Größenverhältnisse (Sonne >> Erde > Mond)
- *Buchbezug: Kap. 7 – Texture-Mapping*

### Phase 3 – Beleuchtung ✅
- [x] Blinn-Phong-Beleuchtungsmodell im Fragment-Shader (WGSL): ambient, diffus, spekular
- [x] Sonne als Positionslicht; Beleuchtungsparameter per egui-Regler einstellbar
- [x] Tag-/Nachtseite auf allen Körpern sichtbar
- *Buchbezug: Kap. 5 – Beleuchtungsmodelle, Kap. 6 – Shading*

### Phase 5 – Echte astronomische Daten (teilweise vorgezogen) ✅/⬜
- [x] **Planetenpositionen via VSOP87** (Crate `vsop87`) – Merkur bis Neptun auf echten Ephemeridenbahnen
- [x] Simulationszeit → Julianisches Datum → Gregorianisches Datum (Meeus-Algorithmus)
- [x] Zeitraffer steuerbar (Faktor, Pause)
- [x] Datum/Uhrzeit als direkter Eingabe-Parameter (Sprung zu einem bestimmten Zeitpunkt)
- [ ] Sternenhintergrund aus HYG-Datenbank (~120.000 Sterne als Punktwolke, Farbe aus Spektralklasse)
- [ ] Logarithmische oder benutzerdefinierte Skalierungsstrategie für echte Abstände

### Phase 4 – Polish ⬜
- [ ] Normal-Map auf der Erde (Gebirge/Ozeane)
- [ ] Atmosphären-Glow für die Erde (Post-Processing oder Halo-Quad)
- [ ] Sternenhintergrund (Skybox oder Billboard-Punkte) ← auch Phase 5
- [ ] Bloom-Effekt für die Sonne
- [ ] Saturnringe
- *Buchbezug: Kap. 8 – Normal-Mapping, Kap. 13 – Displacement-Mapping*

## Mögliche nächste Schritte

### Hoher Mehrwert, überschaubar
1. **Sternenhintergrund** – HYG-Katalog als CSV einlesen, ~120 k Punkte als Punktwolke rendern (eigene Pipeline, `gl_PointSize` via WGSL `point-size`-Feature oder Billboard-Quads); Farbe aus Spektralklasse (B–V-Index → RGB)
2. **Saturnringe** – flaches Ringmesh (Annulus-Geometrie), eigene Textur (`2k_saturn_ring_alpha.png` NASA), Alpha-Blending

### Mittlere Komplexität
4. **Atmosphären-Halo** – screen-space Overlay oder billboardierter Halo-Ring um Erde/Venus; keine echte Volumetrik nötig
5. **Emissives Material für die Sonne** – separater Beleuchtungspfad (kein Phong, nur volle Farbe/Bloom-Vorbereitung); `use_texture`-Flag bereits im Uniform vorhanden
6. **Echtzeit-Datum als Startparameter** – beim Start `SystemTime::now()` → JDE → `sim_time` setzen, sodass die Simulation in der Gegenwart beginnt

### Größerer Aufwand
7. **Normal-Maps** – zweite Textur-Bind-Group, TBN-Matrix im Vertex-Shader, Tangent-Attribute in `Vertex`
8. **Bloom** – separater Render-Pass (Bright-Pass-Filter → Gaussian-Blur → Additive Composite)
9. **Abstandsskalierung** – logarithmische oder hybride Skala (reale AE-Abstände sind für Visualisierung unbrauchbar ohne Skalierung); UI-Regler für Skalenexponent

## Architektur-Leitlinien

- **Kamera:** `Camera`-Enum mit `Fps` und `Orbit(OrbitCamera)`; `OrbitCamera` enthält das Ziel über `target: BodyId` – Laufzeit-Umschaltung ohne Controller-Umbau
- **Szenen-Datenstruktur:** `BodyId`-Newtype schützt vor falscher Indizierung; Eltern müssen immer vor Kindern in `celestial_bodies` stehen (debug_assert prüft das)
- **Simulationszeit:** Immer `sim_time: f64` aus `SimState` verwenden, niemals `std::time::Instant` direkt in Orbitlogik – ermöglicht Pause und Zeitraffer ohne Refactoring
- **Shader-Fehler:** Neue Shader immer über `shader_loader::validate_wgsl` laufen lassen, bevor `create_shader_module` aufgerufen wird – liefert source-span-genaue miette-Fehler statt wgpu-Panic
- **WASM-Kompatibilität:** `loader::load_bytes/load_str` für alle Assets verwenden; kein `std::fs` direkt; kein Threading ohne `wasm-bindgen-rayon`

## Konventionen

- Shader-Dateien: `*.wgsl` in `src/shaders/`
- Texturen: `assets/textures/` (NASA-Quellen bevorzugen, lizenzfrei)
- GLSL-Konzepte aus dem Buch 1:1 in WGSL übertragen; Unterschiede dokumentieren
- Alle `pub`-Items bekommen einen `///`-Doc-Comment in Englisch
- Neue Phasen erst beginnen, wenn alle Checkboxen der aktuellen Phase abgehakt sind
