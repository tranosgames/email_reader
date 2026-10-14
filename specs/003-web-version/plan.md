# Implementation Plan: Version web locale de la visionneuse

**Branch**: `003-web-version` | **Date**: 2026-10-14 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/003-web-version/spec.md`

## Summary

Publier l'interface de la feature 001 comme page web statique dont l'analyse s'exécute dans le
navigateur. Le moteur Rust existant est compilé en WebAssembly (`mf-wasm`), chargé dans un Web
Worker ; l'interface TypeScript est partagée avec l'application Tauri et choisit son backend au
démarrage. Aucune donnée ne quitte le navigateur ; une CSP stricte et un test e2e le garantissent.

## Technical Context

**Language/Version**: Rust 1.97 (cible `wasm32-unknown-unknown`), TypeScript 5

**Primary Dependencies**: `wasm-bindgen` 0.2.129 (+ `wasm-bindgen-cli`), moteur `mf-engine` existant, Vite 7 ; tests : Playwright

**Storage**: aucun (mémoire de l'onglet)

**Testing**: `cargo test` (parité `open` / `open_bytes`), Playwright (e2e, concordance avec `mf show --json`, zéro réseau)

**Target Platform**: navigateurs récents (Chrome/Edge/Firefox/Safari) ; GitHub Pages ou tout serveur statique

**Project Type**: web statique (même dépôt, même frontend que l'app de bureau)

**Performance Goals**: message ≤ 25 Mo affiché < 3 s ; page < 5 Mo

**Constraints**: aucune requête après chargement ; aucune transmission d'octets ; limite 500 Mo par fichier

**Scale/Scope**: une page, aucune nouvelle vue

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principe | Vérification | Statut |
|---|---|---|
| I. Intégrité | Le navigateur ne peut pas écrire sur le fichier source ; empreintes calculées par le même code Rust sur les octets lus | ✅ |
| II. Traçabilité | Mêmes `SourceRef`/offsets ; étiquette de preuve = chemin relatif (documenté dans le contrat) | ✅ |
| III. Moteur d'abord | `mf-wasm` = liaison de 20 lignes vers `mf-engine::open_bytes` ; aucune logique dans le JS | ✅ |
| IV. Contenu hostile | Même sanitisation + iframe sandbox ; CSP de page `default-src 'none'` | ✅ |
| V. Tests | Test de parité Rust + e2e de concordance navigateur ↔ CLI | ✅ |
| VI. Hors-ligne (v1.1.0) | Page statique, zéro requête après chargement, vérifiée par test | ✅ |
| VII. Simplicité | Pas de serveur, pas de second frontend, suppression du mode démo JSON | ✅ |

**Re-check post-design** : ✅ (amendement constitution 1.1.0 appliqué avant cette feature).

## Project Structure

### Documentation (this feature)

```text
specs/003-web-version/
├── plan.md
├── research.md
├── quickstart.md
├── contracts/wasm-api.md
├── checklists/requirements.md
└── tasks.md
```

### Source Code (repository root)

```text
crates/
├── mf-core/src/lib.rs        # + evidence_from_bytes
├── mf-engine/src/lib.rs      # + open_bytes (chemin commun open / open_bytes)
└── mf-wasm/                  # NOUVEAU : liaison wasm-bindgen (analyze, engine_version)
apps/desktop/                 # interface partagée bureau + web
├── src/
│   ├── ipc.ts                # backend Tauri ou WebAssembly, choisi au démarrage
│   ├── engine.worker.ts      # NOUVEAU : Web Worker qui charge le module wasm
│   ├── web-engine.ts         # NOUVEAU : création du worker (remplacé par web-engine.stub.ts dans le build de bureau)
│   ├── FileHandler.ts        # + glisser-déposer HTML5 / <input type=file> / dossiers
│   └── wasm/                 # généré par wasm-bindgen (non versionné)
├── scripts/build-wasm.sh     # NOUVEAU : cargo build wasm + wasm-bindgen
├── tests/web_e2e.py          # NOUVEAU : e2e Playwright (Python)
└── vite.config.ts            # + mode « web » (base relative, dist-web/, CSP)
.github/workflows/pages.yml   # NOUVEAU : publication GitHub Pages
```

**Structure Decision**: on ne duplique pas l'interface : `apps/desktop/src` sert aux deux cibles.
Le mode « démo » (JSON pré-généré) de la feature 001 est remplacé par le vrai moteur web et
supprimé avec `scripts/make-demo.py`.

## Complexity Tracking

| Violation | Why Needed | Simpler Alternative Rejected Because |
|---|---|---|
| Web Worker en plus du fil principal | Garder l'interface réactive pendant l'analyse de gros MSG | Analyse sur le fil principal : onglet figé plusieurs secondes sur un MSG de 50 Mo |
