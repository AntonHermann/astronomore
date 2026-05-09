# Sonnensystem in Rust/wgpu

## Projektübersicht

3D-Sonnensystem-Simulation in Rust, aufbauend auf einem Java/2D-Vorläufer. Ziel ist eine vollständige Echtzeit-Simulation mit physikalisch korrekter Beleuchtung, Texturen und perspektivischer Kamera – nativ und im Browser via WASM lauffähig.

**Aktueller Stand:** Texturiertes 2D-Polygon (Pentagon) mit funktionierender Perspektivkamera und interaktiver Kamerasteuerung (nativ + WASM).

## Technologie-Stack

- **Sprache:** Rust (Edition 2024)
- **Grafik-API:** wgpu 29 (nativ + WASM/Browser)
- **Shader:** WGSL
- **Mathematik:** glam (Vec3, Mat4 – ersetzt cgmath)
- **Fehlerbehandlung:** miette
- **Parallelreferenz:** Java/LWJGL-Projekt (Buch: "Beleuchtung und Rendering", 3. Auflage)

## Bereits umgesetzt

- wgpu-Grundgerüst mit winit (ApplicationHandler-API)
- Vertex Buffer + Index Buffer (Pentagon aus 5 Vertices, 3 Dreiecke)
- Textur-Mapping: `texture::Texture` in eigenem Modul (`src/texture.rs`), RGBA8-Upload via `queue.write_texture`
- Texture Bind Group (group 0): Sampler + TextureView im Fragment-Shader
- Perspektivkamera (`Camera` + `CameraUniform`) mit View-Projection-Matrix als Uniform Buffer (group 1)
- `CameraController`: WASD/Pfeiltasten, orbitiert um den Ursprung, Escape beendet
- Kamera-Update pro Frame via `queue.write_buffer`
- WASM-Pfad: WebGL-Backend, `wasm-bindgen`, Canvas-Integration via `index.html`

## Dateistruktur

```
src/
  lib.rs              – State, Camera, CameraController, App, ApplicationHandler
  main.rs             – Einstiegspunkt (nativ)
  texture.rs          – Texture::from_bytes / from_image
  happy-tree.png      – Testtextur (aktuell hardcodiert via include_bytes!)
  shaders/
    shader.wgsl       – Vertex- und Fragment-Shader (UV-Textur + Kamera-Uniform)
assets/textures/      – (noch nicht angelegt, für NASA-Texturen vorgesehen)
```

## Build & Entwicklung

```sh
just run        # nativ bauen und starten
just serve      # WASM bauen + Python-HTTP-Server auf :8080
just san        # cargo fmt + clippy -D warnings
```

## Phasenplan

### Phase 1 – Fundament ✅ (größtenteils)
- [x] Uniforms & MVP-Transformationsmatrizen (View + Projection via glam)
- [x] Index Buffer
- [x] Perspektivkamera hinter einfachem Struct
- [ ] Perspektivkamera hinter Trait oder `enum CameraMode { Free, Orbit(PlanetId) }`
- [x] Textur-Mapping (UV-Koordinaten, Bind Group)
- [ ] UV-Kugel tessellieren (Vertices per CPU aus Breiten-/Längengraden)
- [ ] Tiefenpuffer (Depth Buffer) – noch nicht aktiviert

### Phase 2 – Sonnensystem 3D
- `struct CelestialBody { name, position, radius, parent, ... }` – Szene als querybare Liste
- Drei Kugeln mit korrekten Größenverhältnissen (Sonne > Erde > Mond)
- Orbitale Animation gegen abstrakte `sim_time: f64` (nicht echte Uhrzeit) – pausierbar, skalierbar
- Textur-Mapping mit NASA-Texturen (UV-Koordinaten bei Tessellierung mitausgeben)
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

### Phase 5 – Echte astronomische Daten
- Planetenpositionen via VSOP87 (Rust-Crate `vsop87`, kein Live-API nötig)
- Sternenhintergrund aus HYG-Datenbank (~120.000 Sterne als Punktwolke, Farbe aus Spektralklasse)
- Echtzeit-Simulation: Datum/Uhrzeit als Eingabe, Zeitraffer steuerbar
- Logarithmische oder benutzerdefinierte Skalierungsstrategie für echte Abstände

## Architektur-Leitlinien

- **Kamera:** Früh hinter Trait oder Enum abstrahieren – verhindert späteren Umbau für Orbit-Kamera
- **Szenen-Datenstruktur:** `CelestialBody`-Liste von Anfang an querybar halten (Voraussetzung für "fliege zu Planet X")
- **Simulationszeit:** Immer `sim_time: f64` verwenden, niemals `std::time::Instant` direkt in Orbitlogik – ermöglicht Pause und Zeitraffer ohne Refactoring
- **Tiefenpuffer:** Muss aktiviert werden, bevor mehrere Objekte gezeichnet werden (RenderPipeline + RenderPass beide anpassen)
- **WASM-Kompatibilität:** Bei jedem neuen Feature prüfen, ob es im Browser funktioniert (kein Threading ohne `wasm-bindgen-rayon`, kein Dateisystem)

## Konventionen

- Shader-Dateien: `*.wgsl` in `src/shaders/`
- Texturen: `assets/textures/` (NASA-Quellen bevorzugen, lizenzfrei)
- GLSL-Konzepte aus dem Buch 1:1 in WGSL übertragen; Unterschiede dokumentieren
- Neue Phasen erst beginnen, wenn alle Checkboxen der aktuellen Phase abgehakt sind
