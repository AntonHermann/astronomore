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
| UI-Zoom vor Präsentation fixen | UI-Scale-Steuerung (`src/ui.rs` / `src/lib.rs`, Tasten `[` / `]`) | Zur Laufzeit anpassbarer UI-Scale-Slider teilweise unbenutzbar, wenn man versehentlich den Slider erwischt | Erstmal auf festen Wert zurück; Git-History nach früher genutzten guten Fixwerten durchsuchen (vor Anpassbarmachung). Vorbereitung Projektpräsentation |
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
| Inhaltsreview der Vergleichs-Doku: alle `datei:zeile`-Referenzen und Code-Zitate der wgpu-Spalte gegen den echten Code verifiziert (Stufen 2–8 korrekt); Fixes: Stufe-1-Zeilennummern nach Commit `b3ffec2` aktualisiert (lib.rs 1546/1473/296), Stufe-3-Fundstelle des `Mesh::sphere`-Aufrufs korrigiert (celestial_body.rs:76 statt mesh.rs:123), zwei Ungenauigkeiten der mittleren Spalte präzisiert (wgpu-Layout-Validierung = Pipeline-Erstellung statt „Compile-Zeit"; Compat-Built-ins nicht „nur #version 130"). Publiziertes Claude-Artefakt auf denselben Stand gebracht | 10.07. — gemergt in main (`b7b150b`), Branch gelöscht |
| Zweite Reviewrunde gegen echte Quellen (Vorlesungsfolien in `docs/vl_slides/` + Buch-Src in `../Eclipsepaket-Dinobuch_Auflage_3/`): linke Spalte durchgängig bestätigt — Torus-Shader wörtlich korrekt zitiert (die drei „Auffälligkeiten" sind echte Buch-Macken, kein Übertragungsfehler → mögliche Vortrags-Pointe); Immediate Mode, `glClearColor`/`glClear`, `glColor3d`-State, Transformationskette (Folie 41/42 in *4.2 OpenGL Grundlagen*) und GLSL-Built-ins/Uniforms (*6.2 Vertex- & Fragment-Shader*) decken sich mit Folien **und** Buch-Src. Fix: Quellen-Footer nannte nicht existierende `Grundlegendes_zu_OpenGL.pdf` → auf die echten Vorlesungsdecks korrigiert, VBO/VAO-„nicht behandelt"-Hinweis ergänzt | 10.07. — gemergt in main (`750b402`), Branch gelöscht |
| OpenGL→wgpu-Vergleichs-Doku (`docs/opengl-wgpu-mapping.html`): 8-Stufen-Pipeline-Gegenüberstellung GL11/modernes GL/wgpu mit Spec-/Stil-Etiketten; via GitHub Pages deployt (Workflow um `docs/`-Kopie erweitert), erreichbar unter `https://antonhermann.github.io/astronomore/docs/opengl-wgpu-mapping.html`. Begleitdoku `docs/opengl-wgpu-analysis.md` (Präsentationszeit/Lernwert/Komplexität/Perf-Achsen, Munzner-Diagnose, Interaktions-Ideen) ergänzt, Footer-Backlink zwischen beiden Dateien gesetzt | 09.07. — Commits `291172c`..`1a22fee`, `7671921`, `8002a28` auf main |
| WASM/WebGL2-Render-Fehler (textureSample in Conditional, UBO-Alignment, HTTP-Cache) | 31.05. — PRs #16–#19, Fixes auf main |
| _(ältere Einträge: siehe Git-Historie bis 31.05.)_ | |
