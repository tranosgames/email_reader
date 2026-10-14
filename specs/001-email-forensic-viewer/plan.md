# Implementation Plan: Visionneuse forensic de courriels EML/MSG

**Branch**: `001-email-forensic-viewer` | **Date**: 2026-10-14 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/001-email-forensic-viewer/spec.md`

**Note**: This template is filled in by the `/speckit-plan` command; its definition describes the execution workflow.

## Summary

Application de bureau d'analyse forensic de courriels `.eml` et `.msg`, reprenant l'ergonomie de
msg-reader (glisser-déposer, liste latérale, panneau de lecture, épinglage, tri, raccourcis) et y
ajoutant les vues d'enquête : en-têtes et chaîne de relais, authentification, pièces jointes et
imbrication, IOC, propriétés brutes, rapport et journal d'audit. Approche : un moteur Rust en
crates indépendantes (`mf-core`, `mf-eml`, `mf-msg`, `mf-analysis`, `mf-report`) exposé par une
CLI `mf` et par une app Tauri 2 au frontend TypeScript vanilla. Les preuves sont lues une seule
fois en lecture seule, hachées, puis parsées depuis la même mémoire. Le modèle unifié est conçu
pour accueillir PST (feature 002, `outlook-pst`) sans toucher à l'UI.

## Technical Context

**Language/Version**: Rust stable 1.97 (édition 2024, MSRV 1.85) ; TypeScript 5 (frontend)

**Primary Dependencies**: `mail-parser` 0.11 (EML), `cfb` 0.15 (OLE/MSG), `compressed-rtf` 1.0 + `rtf-parser` 0.4 (RTF), `ammonia` 4 (HTML), `mail-auth` 0.13 (DKIM/ARC parsing), `infer` 0.22, `zip` 9, `sha2` 0.11, `md-5` 0.11, `linkify` 0.11, `idna` 1.1, `unicode-security` 0.1, `serde`/`serde_json`, `askama`, `clap` 4, `thiserror` 2, `tracing`, `chrono` ; Tauri 2.12 (+ `tauri-plugin-dialog`), Vite, Tailwind ; tests : `insta`, `proptest`, `cargo-fuzz`, oracle `msg_parser` 0.3 (dev-dependency)

**Storage**: aucun SGBD ; modèle en mémoire ; dossier d'enquête JSON `*.mfcase.json` ; exports HTML/JSON/CSV vers une destination choisie par l'analyste

**Testing**: `cargo test` (unitaires + intégration sur corpus `tests/corpus/` avec snapshots `insta`), tests de concordance contre `msg_parser`, `proptest` sur les décodeurs, `cargo-fuzz` (EML, MSG, RTF, Authentication-Results), test e2e Tauri (WebDriver) sur scénario 4 du quickstart

**Target Platform**: Windows 10+, macOS 13+, Linux (webkit2gtk 4.1) — x86_64 et aarch64

**Project Type**: desktop-app + CLI + bibliothèques (workspace Cargo)

**Performance Goals**: affichage d'un message ≤ 25 Mo < 2 s ; ingestion de 500 messages < 60 s (parsing parallèle `rayon`, hachage en flux) ; filtre/recherche sur 1 000 messages < 1 s

**Constraints**: hors-ligne strict, aucune écriture sur les preuves, aucune exécution de contenu, `#![forbid(unsafe_code)]`, pas de panic sur entrée, limites data-model.md, mémoire < 2× taille cumulée des preuves chargées

**Scale/Scope**: jusqu'à quelques milliers de messages par session ; 6 vues ; ~34 exigences fonctionnelles

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principe | Vérification dans ce plan | Statut |
|---|---|---|
| I. Intégrité de la preuve | Lecture unique en lecture seule (`OpenOptions::read(true)`), hachage avant parsing sur les mêmes octets (R8) ; destination d'export refusée si dans le répertoire d'une preuve ; revérification avant export (FR-027) ; valeurs brutes conservées (`DecodedValue`, `DateValue`) | ✅ |
| II. Traçabilité | `AuditLog` append-only dans `mf-core` ; chaque `Finding` porte des `SourceRef` (offsets EML / tags MAPI) ; rapport trié de façon déterministe (schéma) | ✅ |
| III. Moteur d'abord | Toute logique dans les crates `mf-*` ; `mf-cli` et `src-tauri` sans logique métier ; trait `EvidenceSource` pour PST | ✅ |
| IV. Contenu hostile | `ammonia` + iframe `sandbox` + CSP `default-src 'none'` ; capacités Tauri sans `shell`/`http`/`fs`/`opener` ; aucune PJ ouverte ; limites + fuzzing | ✅ |
| V. Tests sur corpus | Corpus versionné + snapshots ; oracle `msg_parser` ; tests écrits avant implémentation dans `tasks.md` ; CI clippy/fmt | ✅ |
| VI. Hors-ligne & reproductible | Aucune dépendance réseau à l'exécution ; vérification DKIM DNS exclue v1 ; `Cargo.lock`, `cargo deny` ; mise à jour auto de msg-reader **non reprise** | ✅ |
| VII. Simplicité | Pas de base de données ni de moteur de recherche ; frontend sans framework | ✅ |

**Re-check post-design (Phase 1)** : data-model, contrats et quickstart respectent les 7 principes ; une seule entrée en Complexity Tracking (couche MAPI maison).

## Project Structure

### Documentation (this feature)

```text
specs/001-email-forensic-viewer/
├── plan.md              # Ce fichier
├── research.md          # Phase 0 — choix techniques et preuves du prototype
├── data-model.md        # Phase 1 — modèle unifié, règles, transitions
├── quickstart.md        # Phase 1 — scénarios de validation
├── contracts/
│   ├── cli.md               # Contrat de la CLI `mf`
│   ├── tauri-commands.md    # Contrat IPC frontend ↔ moteur
│   └── report.schema.json   # Schéma JSON du rapport
├── checklists/
│   └── requirements.md
└── tasks.md             # Phase 2 (/speckit-tasks)
```

### Source Code (repository root)

```text
Cargo.toml                    # workspace
deny.toml
crates/
├── mf-core/                  # modèle unifié, Evidence/hachage, AuditLog, Case, limites, trait EvidenceSource
│   └── src/{model.rs, evidence.rs, hashing.rs, audit.rs, case.rs, limits.rs, source.rs, ingest.rs, search.rs}
├── mf-eml/                   # EML → Message (mail-parser), offsets d'en-têtes, message/rfc822 récursif
│   └── src/{lib.rs, convert.rs, headers.rs}
├── mf-msg/                   # MSG → Message : OLE (cfb), MAPI props fixes/variables, destinataires, PJ, embarqués, nameid, RTF
│   └── src/{lib.rs, ole.rs, props.rs, tags.rs, recipients.rs, attachments.rs, named.rs, rtf.rs, convert.rs}
├── mf-analysis/              # Received, Authentication-Results, DKIM bh, règles de constats, PJ (type réel, macros), IOC
│   └── src/{hops.rs, auth.rs, dkim.rs, rules/, attachments.rs, iocs.rs, html.rs, defang.rs}
├── mf-report/                # rapport HTML (askama) + JSON, export PJ + manifeste, export IOC
│   └── src/{lib.rs, json.rs, html.rs, export.rs}  templates/report.html
├── mf-engine/                # point d'entrée unique : ouvrir → hacher → parser → analyser ; PJ .msg/.eml ouvertes récursivement
│   └── src/lib.rs
└── mf-cli/                   # binaire `mf` (clap) — couche mince
    └── src/main.rs
apps/
└── desktop/
    ├── src-tauri/            # Tauri 2 — commands.rs (contrat IPC), jobs.rs, capabilities/, tauri.conf.json
    ├── index.html
    └── src/                  # TypeScript vanilla, découpage msg-reader
        ├── main.ts
        ├── FileHandler.ts    # glisser-déposer, dialogue fichiers/dossiers
        ├── MessageHandler.ts # état : liste, sélection, épingles, filtres
        ├── UIManager.ts      # rendu liste + panneau + onglets
        ├── views/{reading.ts, headers.ts, attachments.ts, iocs.ts, raw.ts, notes.ts, timeline.ts}
        ├── ipc.ts            # wrappers typés des commandes Tauri
        ├── dom.ts            # construction DOM sans innerHTML
        └── styles.css        # CSS natif à variables, clair/sombre selon le système, polices embarquées (hors-ligne)
tests/
├── corpus/{eml/, msg/, nested/, malformed/, expected/}
└── integration/              # tests transverses (intégrité, déterminisme, concordance oracle)
fuzz/
└── fuzz_targets/{eml_parse.rs, msg_parse.rs, rtf_decompress.rs, auth_results.rs}
```

**Structure Decision**: workspace Cargo « bibliothèques + CLI + app de bureau ». Le découpage des
crates suit les frontières de format (`mf-eml`, `mf-msg`, plus tard `mf-pst`) et de responsabilité
(`mf-analysis` ne dépend que de `mf-core`, donc fonctionne identiquement pour tout format).
Le frontend reproduit volontairement les modules de msg-reader (`FileHandler`, `MessageHandler`,
`UIManager`, `main`) pour faciliter la transposition de son design.

Graphe de dépendances : `mf-core` ← `mf-eml`, `mf-msg`, `mf-analysis` ← `mf-engine` (← `mf-report`) ← `mf-cli`, `src-tauri`. `mf-engine` a été ajouté à l'implémentation (2026-10-14) pour que la CLI et l'app partagent le même chemin d'ouverture ; Tailwind a été écarté au profit de CSS natif (moins d'outillage, Constitution VII).

## Complexity Tracking

| Violation | Why Needed | Simpler Alternative Rejected Because |
|---|---|---|
| Couche MAPI maison sur `cfb` au lieu d'utiliser `msg_parser` seul | Accès à toutes les propriétés brutes (FR-031), adresse SMTP réelle de l'expéditeur, dates MAPI, en-têtes de transport, propriétés nommées | `msg_parser` renvoie le DN X.500 Exchange comme expéditeur (vérifié par prototype) et n'expose pas l'ensemble des propriétés ; il reste utilisé comme oracle de test |
