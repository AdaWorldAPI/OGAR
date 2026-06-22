# OGAR — Philosophie

> [English](PHILOSOPHY.md) · **Deutsch** · [↑ README (Spezifikation)](../README.de.md)

Das ist das *Warum* vor dem *Was*. Für die Spezifikation siehe die
[README](../README.de.md). Für die operative Disziplin siehe die
[sieben Design-Wächter](../README.de.md#design-wächter) in `docs/`.

---

### Frag eine Fechterin mit dreißig Jahren Erfahrung, was Fechten ist — und sie greift zur Kelle und schöpft Wasser.

Eine Bewegung. Ruhig, exakt, nichts verschüttet. Die ganze Kunst steckt
darin — die Ökonomie, die Linie, das Gewicht, das schon vor dem Heben
bekannt ist. Es sieht aus wie nichts. Das *sind* die dreißig Jahre.

OGAR ist eine Kelle.

```rust
let cid = HealthcarePort::class_id("Patient");   // Some(0x0901)
```

Ein Aufruf. Du ziehst die classid, und alles, was das Konzept braucht,
ist bereits da — seine Form, sein Grant, sein Lebenszyklus, sein Platz
in einem Adressraum von 2,8 × 10¹⁴ Zellen, den sich jede App teilt, die
je einen Patienten benannt hat. Du baust es nicht. Du prägst es nicht.
Du **schöpfst**.

Der Rest dieses Dokuments sind die dreißig Jahre.

---

## Die drei Erkenntnisse

Alles andere ist Detail. Wenn du dir nur drei Dinge merkst, dann diese —
sie sind das Muskelgedächtnis, das der ganze Stack automatisch machen
soll.

### 1 · Die classid ist reine Adresse. Die Magie ist das, worauf sie auflöst.

Eine classid sind 32 Bit *Adresse* — mehr nicht. Sie trägt kein
Verhalten. Sie zu kennen sagt dir *welcher* Knoten, *wessen* Haut,
*welche* Form — nie *wie er sich verhält*.

```
classid : u32  =  0xAAAA ‖ 0xDDCC              beide Hälften sind ADRESSE
                    │         │
                    │         └─ lo u16 — WELCHES KONZEPT  (geteilte Identität)
                    └─────────── hi u16 — WESSEN RENDER     (App-eigene Haut)

         ──────►  löst auf zu  ──────►
            ├─ ClassView                die HAUT      (Render — pro App)
            ├─ Class                    die FORM      (strukturell — geteilt)
            └─ ActionDef + KausalSpec   die MAGIE     (Verhalten — Lebenszyklus,
                                                       Callbacks, Validierungen;
                                                       immer im Core,
                                                       nie in der Adresse)
```

Der Rails-Block `class Patient < ApplicationRecord; validates …;
before_save …; end` — die *Klassen-Magie* — steckt **nicht** in der ID.
Er ist das, worauf die ID verweist. Die ID adressiert; der Core verhält
sich.

### 2 · Ein Konzept, viele Render.

Die **unteren** 16 Bit benennen das Konzept — geteilt von jeder App, mit
einem RBAC-Grant und einer Ontologie. Die **oberen** 16 Bit benennen die
Render-Linse — das eigene Template jeder App, ihre eigene Haut. Gleiches
Konzept, andere Kleidung:

```
0x0001_0102  ─ OpenProjects WorkPackage  ┐  gleiche lo 0x0102 = project_work_item
0x0007_0102  ─ Redmines Issue            ┘  → EIN Grant, EINE Ontologie, ZWEI Templates
```

Fünf Apps können *abrechenbare Zeit* auf fünf Arten rendern und lösen
trotzdem auf dieselbe `0x0103` auf — „Planer-Stunden stimmen mit der
Abrechnung überein" ist also ein Codebook-Lookup, keine
Übersetzungsschicht.

### 3 · Der Core verhält sich; der Producer speist ihn; der Adapter hallt nur wider.

Verhalten fließt **Producer → OGAR `Class` + `ActionDef` → Adapter** —
nie Producer → DDL. SurrealQL, PostgreSQL, OpenAPI, TypeScript sind
**Adapter**: verlustbehaftete Echos des Core, *aus* ihm emittiert,
niemals das Rückgrat. Lebenszyklus in `DEFINE EVENT … WHEN … THEN …` zu
kodieren heißt, den Wert in den Schlüssel zu legen — die Falle, gegen
die die [Design-Wächter](../README.de.md#design-wächter) existieren.

---

## Das Muskelgedächtnis, in einem Atemzug

> **Zieh die classid. Präge niemals den Core neu.**

Ein Consumer (medcare-rs, woa-rs, smb-office-rs, odoo-rs, …) ist
*Anreicherung über einer classid* — mehr nicht. Vier Züge, und nur einer
davon ist je deiner:

| Zug | Die Geste | Deiner? |
|---|---|---|
| **Pull** | `Port::class_id("Patient")` → `0x0901` | nein — reine Funktion |
| **Render** | `APP_PREFIX \| concept` → `0x0005_0901` | nein — typisierter Helfer |
| **Authorize** | `authorize(actor, concept, op)` | nein — das geteilte Grant-Gitter |
| **Enrich** | deine Domänen-Logik, an der classid verschlüsselt | **ja** — das ist der Teil, der legitim deiner ist |

Du kopierst nie das Codebook. Du konstruierst nie eine `*Bridge`. Du
prägst nie ein paralleles Register. Der Core besitzt Form und Verhalten;
du hältst die classid und fügst deine Domäne hinzu. Das ist das
Schöpfen.

---

## Wohin als Nächstes

- [**README**](../README.de.md) — die Spezifikation (umgekehrte Pyramide, mustergedrängt)
- [**OGAR-CONSUMER-BEST-PRACTICES.md**](OGAR-CONSUMER-BEST-PRACTICES.md) — das Muskelgedächtnis, durchgearbeitet mit Beispielen für jeden Consumer
- [**SURREAL-AST-TRAP-PREFLIGHT.md**](SURREAL-AST-TRAP-PREFLIGHT.md) — der Producer-seitige Wächter gegen die Negative-Schönheit-Falle aus Erkenntnis 3
- [**APP-CLASS-CODEBOOK-LAYOUT.md**](APP-CLASS-CODEBOOK-LAYOUT.md) — Erkenntnis 2 in formaler Spezifikationsform
- [**THE-FIREWALL.md**](THE-FIREWALL.md) — der strukturelle Grund, warum Adapter nicht das Rückgrat sein können

Die README ist die Spezifikation. Die sieben Design-Wächter in `docs/`
sind die operative Disziplin. Dieses Dokument ist nur das Warum.
