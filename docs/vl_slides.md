# Vorlesungsfolien (VL slides)

Die Vorlesungsfolien, auf denen dieses Projekt aufbaut, liegen **nicht**
in diesem (öffentlichen) Repo, da sie urheberrechtlich geschütztes
Lehrmaterial der Universität sind.

Sie liegen stattdessen in einem privaten Companion-Repo:
[`AntonHermann/astronomore-slides`](https://github.com/AntonHermann/astronomore-slides).

## Zugriff für Claude-Sessions

Das Repo ist privat und muss pro Session explizit hinzugefügt werden
(`add_repo`-Tool bzw. Claude Code Remote), bevor Claude auf die Folien
zugreifen kann. Es ist nicht automatisch Teil des Session-Kontexts.

Lokal (nativ, außerhalb der Remote-Sessions) einfach parallel zu
`astronomore/` klonen:

```sh
git clone git@github.com:AntonHermann/astronomore-slides.git ../astronomore-slides
```
