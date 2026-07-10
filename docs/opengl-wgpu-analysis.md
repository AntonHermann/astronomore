# OpenGL→wgpu-Vergleich — Analyse & Konzept-Backlog

> **Begleitdokument** zu [`opengl-wgpu-mapping.html`](https://antonhermann.github.io/astronomore/docs/opengl-wgpu-mapping.html).
> Sammelt die Research-Ergebnisse zu der Vergleichs-Doku, die (Stand jetzt)
> **bewusst NICHT** ins Artefakt selbst eingeflossen sind. Dient als
> Entscheidungsgrundlage für die Weiterentwicklung und die Vortragsvorbereitung.

> ⚠ **Ehrlichkeit zur Entstehung.** Die folgenden Einschätzungen (Präsentationszeit,
> Lernwert, Komplexität, Performance, sowie die Konzept-/Diagnose-Teile) sind
> **KI-generiert (Claude) und nicht verifiziert.** Zahlen wie „6,5 min" oder
> „10×–1000×" sind grobe Schätzungen, keine Messungen. Wo etwas für den Vortrag
> zählt, an der Primärquelle gegenprüfen. Die Modern-OpenGL-Bezüge kenne ich
> selbst nicht aus eigener Praxis.

Die Vergleichs-Doku gliedert die Rendering-Pipeline in **8 Stufen**:

1. Fenster & Render-Loop
2. Bildschirm löschen (Clear)
3. Geometrie zeichnen (Immediate Mode vs. Vertex/Index-Buffer)
4. Transformationskette (Matrix-Stack vs. explizite Ausdrücke)
5. Daten in den Shader (Built-ins vs. Bind Groups)
6. Beleuchtung (Per-Vertex vs. Per-Fragment Blinn-Phong)
7. Material (glColor3d vs. Uniform-Flag)
8. Shader laden (GL-Info-Log vs. naga-Validierung)

---

## Teil 1 — Vier Analyse-Achsen je Stufe (Research-Runde 1)

### Master-Übersicht

| # | Stufe | ⏱ Präsentation | 🎓 Lernwert | 🧩 Verteilungs-Last | ⚡ Perf-Hebel |
|---|---|---|---|---|---|
| 1 | Render-Loop | 5 min | niedrig | ● gering | klein (Idle-CPU) |
| 2 | Clear | 2,5 min | niedrig–mittel | ○ minimal | keiner |
| 3 | **Geometrie (VBO)** | 4,5 min | **hoch** | ●● mittel | **sehr groß** |
| 4 | **Transformation** | **6,5 min** | **hoch** | ●●● hoch | keiner (Runtime) |
| 5 | **Bind Groups** | 4 min | mittel | ●●●● **sehr hoch** | klein (Enabler) |
| 6 | Beleuchtung | 4,5 min | **hoch** | ●●● hoch | negativ (Qualität) |
| 7 | Material | 3,5 min | mittel | ●● mittel | keiner |
| 8 | Shader laden | 3 min | niedrig | ●● mittel | keiner (nur DX) |

**Kernbeobachtung:** Die vier Rankings korrelieren kaum. Was man lernen sollte,
was schnell macht, was kompliziert zu verdrahten ist und was lange zu erklären
ist, sind vier verschiedene Reihenfolgen.

### 1.1 Präsentationszeit

Schätzung für ein Publikum, das den GL11-Vorläufer kennt und den wgpu-Umstieg
verstehen soll. Pace: Code zeigen, Kernunterschied erklären, Rückfragen abfangen.

| # | Stufe | Kern (li+re) | +Modern-GL | Begründung |
|---|---|---|---|---|
| 1 | Render-Loop | 5 min | +1,5 | Inversion of Control + `request_redraw`-Re-Arming ist subtil |
| 2 | Clear | 2,5 min | +0,5 | Kürzeste Stufe, 2 Zeilen vs. Descriptor |
| 3 | Geometrie | 4,5 min | +2 | „Größter Sprung", aber intuitiv; Modern-GL (VAO/VBO) lohnt eigene Zeit |
| 4 | Transformation | 6,5 min | +2 | Tiefste Stufe: Matrix-Reihenfolge, `GL_MODELVIEW`-Pointe |
| 5 | Daten→Shader | 4 min | +1,5 | „woher kommt `gl_ModelViewProjectionMatrix`?" eingängig |
| 6 | Beleuchtung | 4,5 min | +1 | Gouraud vs. Phong + `reflect` vs. Halfway |
| 7 | Material | 3,5 min | +1,5 | Globaler State schnell; „uniform control flow" subtil |
| 8 | Shader laden | 3 min | +1 | Info-Log vs. naga; „wgpu nutzt naga eh intern" |

- **Nur Kern (links+rechts): ~33,5 min** + Intro ~2–3 min ≈ **~36 min**
- **Mit Modern-GL-Spalte: ~44,5 min** + Intro ≈ **~47 min**

**Bei Zeitmangel kürzen:**
- Stufe 2 + 8 zusammenziehen als „die zwei, wo sich wenig ändert" (~−2 min).
- Modern-GL-Spalte nur bei **Stufe 3 und 4** aufklappen (größter Aha „das ist
  schon modernes GL, nicht erst wgpu"); bei 1/2/6/8 weglassen (~47 → ~40 min).
- Stufe 4 **nicht** kürzen (konzeptueller Kern); eher Stufe 6 straffen
  (Detailmathe weg) und Stufe 7 auf State-vs-Flag reduzieren.

### 1.2 Lernwert (rein pädagogisch)

Wie wichtig ist das Konzept für **Computergrafik als Fach** — zeitlose,
API-unabhängige Grundlagen vs. Plattform-/Tooling-Detail?

| # | Stufe | Lern-Relevanz | CG-Konzept dahinter |
|---|---|---|---|
| 4 | Transformation | **hoch** | Koordinatenräume, MVP-Kette, homogene Koord., nicht-kommutative Komposition |
| 6 | Beleuchtung | **hoch** | Reflexionsmodell, Phong/Blinn-Phong, N·L·V·H, Gouraud vs. Phong |
| 3 | Geometrie | **hoch** | Mesh/Vertex-Repräsentation, retained vs. immediate, CPU→GPU-Datenmodell |
| 5 | Daten→Shader | mittel | Konzept (programmierbare Pipeline) hoch, aber Framing (Bind-Group-Mechanik) tooling-lastig |
| 7 | Material | mittel | Texture-Mapping real hoch, aber als State-vs-Flag gerahmt |
| 2 | Clear | niedrig–mittel | Framebuffer/Tiefenpuffer; angrenzend Z-Buffering (versteckt) |
| 1 | Render-Loop | niedrig | Kaum CG — OS-/Windowing-Plumbing |
| 8 | Shader laden | niedrig | Entwickler-Tooling, null CG-Theorie |

**Rangfolge:** 4 › 6 › 3 (Kern-Trio) › 5 › 7 › 2 › 1 › 8
**Herz des CG-Lernens:** Stufen **4 (Transformation), 6 (Beleuchtung),
3 (Geometrie)** — eigene Lehrbuchkapitel, kehren in jeder API wieder.

### 1.3 Komplexitäts-/Verteilungslast (Code-Lokalität)

Der Sprung von „was ich schreibe, wird direkt gezeichnet" (GL11: alles in einer
`renderLoop()`) zu „ein Objekt ist über 6–7 Dateien verteilt, die ich gleichzeitig
konsistent halten muss". Leitkonzept: *einen beleuchteten, texturierten Planeten
an korrekter Orbitalposition zeichnen.*

**Kopplungsstellen, die für dieses eine Objekt synchron sein müssen:**

| # | Was synchron sein muss | Dateien | Bruch-Symptom |
|---|---|---|---|
| 1 | Vertex-Layout ↔ Shader-`@location` | `mesh.rs:20-47` ↔ `shader.wgsl:44-51` | verzerrte Geometrie, kein Compilerfehler |
| 2 | Bind-Group-Nr. 0/1/2/3 (dreifach: Pipeline-Layout, `@group`, `set_bind_group`) | `pipelines.rs:58-63` ↔ `shader.wgsl` ↔ `celestial_body.rs:212-214` + `scene.rs:134` | Validierungsfehler *oder* stilles Falschbinden |
| 3 | Uniform-Struct doppelt, byte-genau (Rust-Padding ↔ WGSL-Alignment) | `celestial_body.rs`, `camera.rs`, `scene_properties.rs` ↔ `shader.wgsl:9-40` | **stiller Datenmüll**, kein Fehler |
| 4 | Layout entfernt vom Verbraucher | `scene.rs:16-30` → `celestial_body.rs:73,114` + `pipelines.rs:61` | Pipeline-Erstellung schlägt fehl, Ursache 3 Dateien weg |
| 5 | Position = Produkt aus 3 Modulen (Orbit + hierarch. Akkumulation + VSOP87 + Eltern-vor-Kind) | `celestial_body.rs:139-153` + `scene.rs` + `orbital.rs` | Mond umkreist falschen Punkt |
| 6 | Selbe Uniform, andere Gruppennr. je Pass: `@group(2)` Haupt-Shader vs. `@group(1)` Normals | `shader.wgsl:22` vs. `normals.wgsl:12` | „group 2" ist nicht global wahr |
| 7 | Latenter Normalen-Bug (Normale==Position, bricht bei non-uniform Scale) | `mesh.rs:145-149` + `shader.wgsl:69-70` + `celestial_body.rs:25` | künftiges non-uniform Scale → falsche Beleuchtung, Ursache in anderer Datei |

**Beitrag je Stufe zur Verteilung:** Stufe **5 (Bind Groups) sehr hoch** (Kern
der Last: Dreifach-Kopplung + doppelte Structs + Layout in Fremddatei), 4 und 6
hoch, 3 mittel, 1/2 gering.

**Fazit:** Der größte kognitive Sprung ist **Stufe 4 + 5 zusammen** — der Moment,
in dem *ein Objekt aufhört, einen Ort im Code zu haben*. Der eigentliche Preis
des modernen Ansatzes ist **Nicht-Lokalität**, nicht Verbosität: Die
gefährlichsten Fehler (#3, #6, #7) werfen keinen lokalen Compilerfehler, sondern
zeigen sich als stiller visueller Müll mit Ursache in einer *anderen* Datei.
`naga`-Validierung und der `BodyId`-Newtype verwandeln einen Teil davon zurück
in prüfbare, lokale Fehler — Padding und Gruppennummern bleiben ungeschützt.

### 1.4 Performance-Hebel & Progressionspfad (rein technisch)

| # | Stufe | Hebel | Art | Begründung |
|---|---|---|---|---|
| 1 | Render-Loop | klein | Latenz | Event-getrieben spart Idle-CPU, kein Durchsatz |
| 2 | Clear | keiner | — | Nur auf Tile-GPUs (Mobile/WebGL) spart `LoadOp` Bandbreite |
| 3 | **Geometrie: Immediate→VBO** | **sehr groß** | Durchsatz | Der eine echte Sprung. Skaliert linear mit Vertex-Zahl. ~10×–1000× |
| 4 | Transformation | keiner (Runtime) | nur Wartbarkeit | Identische Mathe, gleiche Anzahl Matrixmult. |
| 5 | Bind Groups | klein, indirekt | Enabler | Voraussetzung für Instancing/gebündelte Uniform-Updates |
| 6 | Beleuchtung: Vertex→Fragment | negativ (teurer) | nur Qualität | Beleuchtung pro Pixel statt pro Vertex |
| 7 | Material | keiner | nur Struktur | Funktions-Äquivalenz |
| 8 | Shader laden | keiner | nur DX | Läuft einmalig beim Laden |

**Kernaussage:** Von 8 Stufen hat **genau eine** großen Laufzeit-Hebel — Stufe 3.
Alles andere ist Qualität (6), Wartbarkeit (4, 7) oder DX (8).

**Progressionspfad je Projekttyp** (für Modernisierung eines alten Immediate-Mode-Projekts):

- **(i) Lern-/Demo, wenig Geometrie:** `3 → 5 → 4 → 6` — Reihenfolge folgt Abhängigkeitskette, nicht Speed.
- **(ii) Viele Objekte / hohe Vertex-Zahl:** `3 → 5 → (Instancing) → 1` — Stufe 3 überlebenswichtig, Bind Groups ermöglichen Draw-Call-Bündelung.
- **(iii) Web/WASM:** `3 → 1 → 2 → 5` — VBO im WebGL2 ohnehin Pflicht; Event-Loop früh (Browser gibt `requestAnimationFrame` vor); `LoadOp::Clear` zahlt sich auf Mobile aus.
- **(iv) Langlebig, Wartbarkeit:** `3 → 4 → 8 → 5 → 7` — nach Pflicht-VBO die Wartbarkeits-Hebel trotz null Runtime-Gewinn.

**Übergreifend:** Stufe 3 steht immer zuerst (einziger großer Perf-Hebel *und*
harte Voraussetzung). Der Rest kippt je nach Projektziel fast vollständig.

### Die drei aufschlussreichen Divergenzen

- **Stufe 4:** maximaler Lernwert, **null Modernisierungsgewinn** — Stack→`glam`
  ist kosmetisch, dieselbe Mathe. Man lernt viel, das Projekt gewinnt an Laufzeit nichts.
- **Stufe 5:** höchste Verteilungs-Last, aber nur mittlerer Lernwert und kleiner
  Perf-Hebel — der meiste *Verdrahtungsschmerz* für das am wenigsten fundamentale Konzept.
- **Stufe 6:** hoher Lernwert, **negativer** Perf-Hebel — per-Fragment ist teurer,
  nicht schneller. „Modern" ≠ „schneller".

---

## Teil 2 — Konzepte & Diagnose (Research-Runde 2)

### 2.1 Munzner Nested Model — Diagnose (der Rahmen für alles Weitere)

Analyse mit Tamara Munzners *Nested Model for Visualization Design and Validation*.

**Die vier verschachtelten Ebenen für dieses Artefakt:**

1. **Domain situation** — mehrere Rollen mit divergierenden Aufgaben: Autor
   (analogischer Transfer GL11→wgpu), Lerngruppe (nachvollziehen/konsumieren),
   latent: jemand der *entscheidet was zu modernisieren* ist. Kern-Domänenproblem:
   Brücke von *bekanntem* zu *fremdem* mentalem Modell — eine **Transfer-/
   Verständnis-Aufgabe (present/consume)**, keine Nachschlage-Aufgabe.
2. **Data/task abstraction** — Daten = facettierte Tabelle (8 Stufen × 3 Spalten ×
   Code/Prosa/Badges + die 4 abgeleiteten Achsen). Tasks: (1) **compare** *innerhalb*
   einer Stufe (zentral kodiert), (2) **summarize** *über* Stufen (nur schwach
   gestützt), (3) present, (4) lookup, (5) browse, (6) **derive/decide**
   (Progressionspfad, strukturell nicht gestützt).
3. **Encoding & interaction idiom** — Position=Implementierungsfamilie,
   vertikale Ordnung=Pipeline, Farbe redundant, Badges (fill vs. outline, schwacher
   Pop-out), Collapsible=reduce/elide. **Für compare-within-stage exzellent, für
   summarize-across-stages schwach.**
4. **Algorithm** — randständig (statisches HTML), korrekt nicht überinvestiert.

**Der zentrale Befund — Ebene-2-Riss, maskiert durch Ebene-3-Politur:**
Das Artefakt *sieht fertig und schön aus* (starkes Encoding), und genau das
verdeckt, dass es sich **nicht entschieden hat, ob es ein *Vergleichs*- oder ein
*Übersichts-/Entscheidungs*-Werkzeug ist.** Dass die 4 Analyse-Achsen (Teil 1)
erst *berechnet* werden mussten, ist der direkte Beweis: die Abstraktion für den
across-stage-Summary-Task ist im Artefakt nicht kodiert.

**Threats to validity:**

| Ebene | Threat | Konkret |
|---|---|---|
| 1 wrong problem | falsche Aufgabe/Rolle | „present" und „decide" wollen verschiedene Artefakte; under-specified |
| **2 wrong abstraction** | **das Falsche zeigen** | **zentraler Riss:** Layout committet auf *compare*, jüngste Dynamik zielt auf *summarize* |
| 3 wrong encoding | wirkt nicht | 3 Spalten nahe Load-Grenze; Spec/Stil schwacher Pop-out; Collapsible ohne Gedächtnis |
| 4 slow algorithm | — | praktisch keins; Risiko = Über-Engineering |

**Munzners Empfehlung (handlungsleitend):** **Zuerst die Task-Abstraktion
schärfen, bevor mehr Encoding/Interaktion draufkommt.** Die Ebenen sind
verschachtelt — kein noch so eleganter Toggle repariert eine unklare Ebene 2.
Konkret:
1. Primären vs. sekundären Task + Rolle festlegen (billig validierbar: Autor fragen).
2. Falls beide Tasks gewollt → **zwei koordinierte Views**: across-stage-Matrix
   („overview first") + bestehende Spalten als Drill-down („details on demand").
   NICHT mehr Facetten in dieselben Spalten stopfen.
3. Toggle-/Interaktions-Ideen danach filtern, welchen Task sie bedienen.
4. Vor dem Bauen: „Wofür ist das und wer liest es?" klären.

**Messlatte für alle Ideen unten:** Welchen abstrahierten Task bedient die Idee —
und ist der Task überhaupt in scope?

### 2.2 Darstellung & globale Toggles (bedient v.a. den Summary-Task)

Zwei kollidierende Lese-Aufgaben: **tief lesen** (eine Stufe, vertikal zusammen)
vs. **vergleichen** (eine Facette über alle Stufen, als kompakte Matrix).
Kernentscheidung: Facetten NICHT in jeden Panel-Header stopfen → sie leben in
einer **Matrix-/Overview-Sicht**, Code+Callout in der **Detail-Sicht**.

**Empfohlenes Layout — aufklappbare Facetten-Matrix:**
- Rückgrat = 8-Zeilen-Matrix (Zeilen=Stufen, Spalten=Facetten), je Zeile auf
  Titel + Facetten-Marken kollabiert.
- Klick auf Zeile → volles Stufen-Detail (3 Code-Spalten + Callout) klappt
  *darunter* auf; Facetten-Marken bleiben am Zeilenkopf sichtbar.
- Overview *ist* die Navigation → kein Overview↔Detail-Switch.
- (Aufwändigste Umstrukturierung: Stufen von statischen `<section>` zu
  Matrix-Accordion-Zeilen.)

**Globale Toggles** (je eine Body-Klasse → alle 8 Stufen konsistent):
- Modern-GL-Spalte global auf/zu (native `<details>` als Per-Stufe-Override behalten)
- Spec/Stil-Badges, Kernunterschied-Callouts, Facetten, Code-Kommentare, Quell-Tags je on/off
- Spalten einzeln ausblendbar; Facetten-Fokus (eine Achse zur Zeit, Rest gedimmt)

**Presets** (der eigentliche Load-Reducer — eine Entscheidung statt zwölf):
Vortrag / Lernen / Nur-Code / Übersicht-Analyse / Alles. Zustand in URL-Hash →
teilbarer Link.

**Default:** schlank — GL11+wgpu sichtbar, Modern-GL kollabiert, Callouts an,
Badges an (subtil), **Facetten AUS** hinter einem „Facetten-Übersicht"-Toggle.

### 2.3 Interaktion / Explorable (bedient v.a. present/consume + Verständnis)

Alles als self-contained HTML + inline JS + Canvas-2D/WebGL2 machbar (CSP-konform;
WebGPU meiden). Priorisiert nach Aufwand/Nutzen:

1. **MVP-Matrixketten-Playground (Stufe 4)** — Slider für T/R/S + Projektion,
   live 4×4-Matrizen + Quad; Reihenfolge S·R vs. R·S umschaltbar → *sieht*
   Nicht-Kommutativität. Lernwichtigste + präsentationsaufwändigste Stufe → hier
   zahlt Interaktivität am meisten. (mittel)
2. **Draw-Call-Zähler Immediate vs. Buffer (Stufe 3)** — Vertex-Zahl-Slider lässt
   den Immediate-Wert explodieren → macht den einzigen großen Perf-Hebel
   selbst-erzeugbar. (mittel)
3. **Kopplungs-Highlighter (Stufe 5)** — Hover auf `@group(2)` hebt Pipeline-Layout,
   `set_bind_group(2)` *und* die `@group(1)`-Abweichung in `normals.wgsl` hervor →
   macht die Nicht-Lokalitäts-These erlebbar. (mittel)
4. **FlaecheSchwingt live (Stufe 4/1)** — die schwingende Fläche aus dem Buch live
   in ~40 Zeilen Canvas-2D, Pause/Zeit-Scrubber. Geringster Aufwand, direktes
   Buch-Beispiel, hoher Delight. (klein–mittel)

Weitere: Blinn-Phong-Playground + Gouraud/Phong-Toggle (Stufe 6), reflect vs.
Halfway-Vektordiagramm, UV-Kugel-Tessellierungs-Slider, Progressive
Code-Disclosure (Ausschnitt→volle Funktion→Datei), Deep-Link zur echten Quelle,
geführte „Spiele-die-Pipeline-durch"-Tour.

**Muster:** nicht *ein* Hero-Widget, sondern kleine Inline-Demos je Stufe genau
dort, wo der Payoff am größten ist. Minimaler erster Schritt: FlaecheSchwingt
(billig, etabliert das Muster), dann MVP-Playground + Draw-Call-Zähler.

### 2.4 Weitere Analyse-Facetten (über die 4 aus Teil 1 hinaus)

Priorisiert nach Mehrwert/Aufwand:

1. **Time-to-first-pixel / Voraussetzungs-DAG** *(billig)* — Wie viele Stufen bis
   zum ersten Pixel? GL11: Stufe 3 allein zeigt ein Dreieck. wgpu: 1→2→3→5→8
   gemeinsam korrekt nötig. Erklärt den „Onboarding-Cliff" moderner APIs und
   liefert die begründete Vortragsreihenfolge gratis.
2. **Mentale Analogien (ein Satz/Stufe)** *(billig)* — z.B. „Immediate Mode = jedes
   Wort einzeln durchtelefonieren; VBO = Dokument einmal einschicken, dann nur
   Bestellnummer nennen". Höchster Hebel für den Vortrag.
3. **Fehlerklassen-Verschiebung (compile/runtime/silent)** *(billig)* — Bündelt die
   Kopplungsfallen zu einer Achse: „modern" beseitigt Fehler nicht, es *verschiebt*
   sie — teils in prüfbare (naga), teils in stille (Padding).
4. **Rollenverteilung Anton-Kern vs. Claude-Boilerplate** *(billig)* — pro Stufe
   markieren (aus CLAUDE.md); zeigt, welche Stufen die eigentliche Lernsubstanz tragen.
5. **Lehrmaterial-Anker** *(teil-billig)* — Folie + Buchkapitel + Datei:Zeile je Stufe.
   **Befund:** die gelesenen Folien (S. 29–45) enden vor den Shader-Stufen — Stufen
   5/6/8 haben ihren Anker im Buch (Kap. 2/5/6), nicht in diesem Foliensatz. Exakte
   Foliennummern für 5/6/8 gegen S. 46–54 zu prüfen ist der teure Teil.

Vorläufige Anker-Tabelle (aus Kontext abgeleitet, **nicht** durch erneutes
PDF-Lesen verifiziert):

| Stufe | Folien (S. 29–45) | Buch (Dino, 3. Aufl.) | Code |
|---|---|---|---|
| 1 Render-Loop | S. 33–35 | Kap. 1 | `lib.rs`, `kapitel01/` |
| 2 Clear | S. 30, 35, 44 | Kap. 1 | `POGL.java` |
| 3 Geometrie | S. 29, 44 | Kap. 1 | `mesh.rs` |
| 4 Transformation | S. 38–42 | Kap. 1 | `camera.rs`, `celestial_body.rs` |
| 5 Bind Groups | **nicht in S. 29–45** | Kap. 2 | `shader.wgsl`, `pipelines.rs` |
| 6 Beleuchtung | **nicht in S. 29–45** | **Kap. 5/6** | `torus*.vs/fs` |
| 7 Material | S. 30, 35 | Kap. 7 | `shader.wgsl` |
| 8 Shader laden | **nicht in S. 29–45** | Kap. 2 | `ShaderUtilities.java` |

Nachrangig: Übertragbarkeit auf Vulkan/Metal/D3D12, LOC-Verhältnis, historische
Zeitleiste, API-Volatilität.

---

## Teil 3 — Was davon ist schon im Artefakt?

| Element | Im Artefakt? |
|---|---|
| 3-Spalten-Gegenüberstellung, 8 Stufen | ✅ ja |
| Modern-OpenGL-Mittelspalte (einklappbar) | ✅ ja |
| Spec-/Stil-Badges | ✅ ja |
| Ehrlichkeits-Disclaimer | ✅ ja |
| **Die 4 Analyse-Achsen (Teil 1)** | ❌ nein — nur hier |
| **Darstellungs-/Toggle-Konzept (2.2)** | ❌ nein — Konzept |
| **Interaktions-/Explorable-Ideen (2.3)** | ❌ nein — Ideen |
| **Munzner-Diagnose (2.1)** | ❌ nein — Rahmen |
| **Weitere Facetten (2.4)** | ❌ nein — Ideen |

## Offener Entscheidungspunkt (vor jeder Weiterentwicklung)

Nach Munzner (2.1): **Zuerst festlegen, was das Artefakt sein soll** —
reines Vergleichs-Tool (aktuelle Stärke) *oder* Vergleich + Übersicht mit zwei
koordinierten Views. Erst danach entscheiden, welche der Konzepte/Ideen aus
Teil 2 überhaupt in scope sind. Für den **Vortrag** ist zusätzlich die
Präsentationszeit (1.1) und die DAG-basierte Reihenfolge (2.4 #1) direkt relevant.
