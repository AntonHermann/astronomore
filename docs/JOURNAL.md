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

## Nächste inhaltliche Schritte (aus CLAUDE.md-Phasenplan)

1. **Sternenhintergrund (HYG-Katalog)** — Claude: CSV-Parsing + Punktwolken-
   Pipeline-Boilerplate; Anton: B–V-Index → RGB, Größenattenuation
2. **Saturnringe** — Anton: Annulus-Geometrie/UVs; Claude: Textur-Beschaffung,
   Alpha-Blending-Pipeline-Setup

## Archiv (erledigt)

| Faden | Abschluss |
|---|---|
| OpenGL→wgpu-Vergleichs-Doku (`docs/opengl-wgpu-mapping.html`): 8-Stufen-Pipeline-Gegenüberstellung GL11/modernes GL/wgpu mit Spec-/Stil-Etiketten | 09.07. — Commits `291172c`..`dd27dc1` auf main |
| WASM/WebGL2-Render-Fehler (textureSample in Conditional, UBO-Alignment, HTTP-Cache) | 31.05. — PRs #16–#19, Fixes auf main |
| _(ältere Einträge: siehe Git-Historie bis 31.05.)_ | |
