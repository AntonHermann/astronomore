# Projekt-Journal — offene Fäden

> **Zweck:** Einzige Quelle der Wahrheit über offene Arbeitsstränge, über
> Chat-Sessions und Branches hinweg. **Jede Claude-Session liest diese Datei
> am Anfang und aktualisiert sie am Ende.** Erledigte Fäden wandern ins Archiv
> (unten), statt gelöscht zu werden.

## Offene Fäden

| Faden | Ort | Stand | Nächster Schritt |
|---|---|---|---|
| Unit-Tests (Orbital-Mathe, Sim-Clock, Kamera, Shader-Validierung) | Branch `claude/evaluate-testing-strategy-zxa84` | Fertig seit 29.05., nie gemergt — main hat 0 Tests | Reviewen, auf aktuellen main rebasen, mergen |
| Code-Vereinfachung aus PR #15 (−184 Zeilen toter Code, `gpu::`-Helfer) | Branch `claude/wasm-pack-incremental-builds-lN1Mm` | Auf GitHub als merged markiert, aber bei main-History-Rewrite verloren gegangen | Auf aktuellen main re-applien und mergen |
| Alte Review-Findings | `claude-code-review.md` (Repo-Root) | Transkript vom Mai; teils erledigt (sim_time f64), teils offen (Maus-Delta-Akkumulation, WASM-Wireframe-Feedback, Multiplier-Untergrenze) | Gültige Findings als Issues anlegen, Datei löschen |
| CI für Branches | — | Pages-Workflow baut nur main; Branch-Brüche (vgl. PR #18/#19) bleiben unentdeckt | Workflow mit `cargo check` (nativ + wasm32) + clippy auf alle Pushes |
| Statusbericht & Workflow-Regeln | Branch `claude/project-status-workflow-bim869` | Dieser Branch: `docs/STATUS-2026-06-10.md`, `docs/JOURNAL.md`, CLAUDE.md-Abschnitt | Anton: lesen, ggf. anpassen, mergen |
| UI-Scaling + Mobile-Eingabe (WASM) | `src/lib.rs`, `src/web_ui.rs`, `index.html` (Branch `claude/ui-scaling-input-keyboard-civtzv`) | **Erledigt, wartet auf Review/Gerätetest.** Zwei Bugs + eine Architektur-Umstellung: (1) egui-Scale-Slider war unbenutzbar durch Feedback-Loop — `set_pixels_per_point` skalierte die UI mid-drag, der Griff rutschte weg. Fix nativ: Scale während Drag konstant halten (`applied_ui_scale`/`ui_scale_dragging`), erst bei Loslassen übernehmen. (2) Ursache Mobile-Tastatur gefunden: winit 0.30.13 hat `set_ime_allowed` im Web-Backend als No-op → Canvas kann keine Bildschirmtastatur öffnen. Lösung: **web-only HTML-Overlay über dem Canvas** (`web_ui.rs` + Markup/CSS in `index.html`) — `<input type="date">` (nativer Picker/Tastatur auf Mobile) + toggelbarer `<input type="range">` für UI-Scale (liegt außerhalb egui → bleibt bei jeder egui-Skalierung erreichbar = Escape-Hatch). Events via wasm-bindgen in einen `Rc<RefCell>`-Puffer, pro Frame in `State` gezogen. Auf Web sind die egui-Datumszeile + egui-Scale-Slider ausgeblendet; nativ unverändert (DragValues + Slider + `[`/`]`). Nativ + wasm32 kompilieren grün | **Auf echtem Handy/Browser testen** (hier kein wasm-pack/Browser-E2E möglich — Proxy blockt Toolchain-Downloads): Datum-Picker öffnet Tastatur? Scale-Toggle + Live-Skalierung? Dann mergen. Optionaler Polish: Date-Picker beim Start auf aktuelles Sim-Datum statt 2000 seeden |
| KI-Nutzungs-Reflexion für Projektpräsentation | Präsentation (Demo + Gegenüberstellung + Reflexion) | Präsentation = (a) kurze Live-Demo, (b) Gegenüberstellung (`docs/opengl-wgpu-mapping.html`, deployt), (c) knappe KI-Nutzungs-Reflexion; Ehrlichkeits-/Vertrauensgrad-Disclaimer in der Doc (Commit `a6a0b9a`) ist erster Baustein | KI-Nutzung im Projekt zusammenfassen und Einbau in die Präsentation überlegen |
| Munzner-Grundsatzentscheidung zur Vergleichs-Doku | `docs/opengl-wgpu-analysis.md`, Abschnitt „Offener Entscheidungspunkt" (Dateiende) | Analyse-Begleitdoku liegt vor (Analyse-Achsen, Munzner-Nested-Model-Diagnose, Interaktions-Ideen); bewusst nicht ins Artefakt gemergt — Trennung Vortragsmaterial vs. privates Entscheidungs-Backlog. Footer-Backlink zwischen beiden Dateien ergänzt (Commit `8002a28`) | Mit Anton klären: bleibt `opengl-wgpu-mapping.html` reines Vergleichs-Tool, oder wird es zur Overview-Matrix + Drill-down (zwei koordinierte Views) ausgebaut; danach ggf. Teil 1/2 der Analyse selektiv einarbeiten. Bewusst für neue Session vertagt |

## Nächste inhaltliche Schritte (aus CLAUDE.md-Phasenplan)

1. **Sternenhintergrund (HYG-Katalog)** — Claude: CSV-Parsing + Punktwolken-
   Pipeline-Boilerplate; Anton: B–V-Index → RGB, Größenattenuation
2. **Saturnringe** — Anton: Annulus-Geometrie/UVs; Claude: Textur-Beschaffung,
   Alpha-Blending-Pipeline-Setup

## Archiv (erledigt)

| Faden | Abschluss |
|---|---|
| OpenGL→wgpu-Vergleichs-Doku (`docs/opengl-wgpu-mapping.html`): 8-Stufen-Pipeline-Gegenüberstellung GL11/modernes GL/wgpu mit Spec-/Stil-Etiketten; via GitHub Pages deployt (Workflow um `docs/`-Kopie erweitert), erreichbar unter `https://antonhermann.github.io/astronomore/docs/opengl-wgpu-mapping.html`. Begleitdoku `docs/opengl-wgpu-analysis.md` (Präsentationszeit/Lernwert/Komplexität/Perf-Achsen, Munzner-Diagnose, Interaktions-Ideen) ergänzt, Footer-Backlink zwischen beiden Dateien gesetzt | 09.07. — Commits `291172c`..`1a22fee`, `7671921`, `8002a28` auf main |
| WASM/WebGL2-Render-Fehler (textureSample in Conditional, UBO-Alignment, HTTP-Cache) | 31.05. — PRs #16–#19, Fixes auf main |
| _(ältere Einträge: siehe Git-Historie bis 31.05.)_ | |
