---

description: "Task list for 003 — Version web locale"
---

# Tasks: Version web locale de la visionneuse

**Input**: Design documents from `/specs/003-web-version/`

**Prerequisites**: plan.md, spec.md, research.md, contracts/wasm-api.md, quickstart.md

**Tests**: inclus (Constitution V).

## Format: `[ID] [P?] [Story] Description`

---

## Phase 1: Setup

> Implémenté le 2026-10-14. Résultat de `npm run test:web` : 0 échec (concordance CLI sur les 7 fichiers du corpus, zéro requête réseau après chargement, fonctionnement hors-ligne).

- [x] W001 Ajouter la cible `wasm32-unknown-unknown` et `wasm-bindgen-cli` 0.2.129 au quickstart et à la CI
- [x] W002 [P] Créer la crate `crates/mf-wasm` (cdylib, `wasm-bindgen = "=0.2.129"`) et l'ajouter au workspace
- [x] W003 [P] Profil `[profile.wasm]` (opt-level "z", LTO, `panic = "abort"`, strip) dans `Cargo.toml`

## Phase 2: Foundational

- [x] W004 Test de parité : `open_bytes` produit le même `Message` que `open` sur tout le corpus dans `crates/mf-engine/tests/corpus.rs`
- [x] W005 `mf_core::evidence_from_bytes` + `mf_engine::open_bytes` (chemin de code commun) dans `crates/mf-core/src/lib.rs`, `crates/mf-engine/src/lib.rs`
- [x] W006 Liaison `analyze` / `engine_version` dans `crates/mf-wasm/src/lib.rs` (contracts/wasm-api.md)
- [x] W007 Script `apps/desktop/scripts/build-wasm.sh` : build wasm + `wasm-bindgen --target web` vers `src/wasm/`

**Checkpoint**: le moteur tourne dans le navigateur.

---

## Phase 3: User Story 1 - Analyser dans le navigateur (Priority: P1) 🎯 MVP

**Independent Test**: déposer le corpus dans la page ; sorties identiques à `mf show --json`.

### Tests ⚠️

- [x] W008 [P] [US1] E2E Playwright : dépôt de fichiers via `<input>`, liste, scellé, bouton En-têtes (`H`), fichiers contenus, URL, message joint, fichier illisible, dans `apps/desktop/tests/web_e2e.py`
- [x] W009 [P] [US1] E2E concordance : pour chaque fichier du corpus, empreintes, nombre d'en-têtes, noms et SHA-256 des PJ, liste d'URL identiques à `mf show --json`, dans `apps/desktop/tests/web_e2e.py`

### Implementation

- [x] W010 [US1] Web Worker `apps/desktop/src/engine.worker.ts` (chargement wasm, transfert sans copie)
- [x] W011 [US1] Backend web dans `apps/desktop/src/ipc.ts` : état des preuves, doublons par SHA-256, résumé identique au backend Tauri, limite 500 Mo (FR-W07)
- [x] W012 [US1] `FileHandler.ts` : sélection de fichiers et de dossier (`webkitdirectory`), glisser-déposer HTML5 avec parcours récursif des dossiers (`webkitGetAsEntry`), filtre `.eml`/`.msg` dans les dossiers
- [x] W013 [US1] Retirer le mode démo JSON (`scripts/make-demo.py`, branche `?demo` de `ipc.ts`)

**Checkpoint**: MVP web fonctionnel.

---

## Phase 4: User Story 2 - Confidentialité vérifiable (Priority: P1)

### Tests ⚠️

- [x] W014 [P] [US2] E2E « zéro réseau » : toutes les requêtes enregistrées ; échec si une requête part après le chargement ; passage hors-ligne puis dépôt d'un fichier, dans `apps/desktop/tests/web_e2e.py`

### Implementation

- [x] W015 [US2] CSP de la page web injectée au build (`vite.config.ts`, mode web) — research W3
- [x] W016 [US2] Mention visible « Analyse locale : les fichiers ne quittent pas ce navigateur et ne sont pas modifiés » (FR-W06) dans l'état vide et le pied de liste

---

## Phase 5: User Story 3 - Déploiement (Priority: P2)

- [x] W017 [US3] Mode Vite `web` : `base: "./"`, sortie `dist-web/`, scripts `dev:web`, `build:web`, `test:web` dans `apps/desktop/package.json`
- [x] W018 [US3] Workflow `.github/workflows/pages.yml` (build wasm + web → GitHub Pages)

---

## Phase 6: Polish

- [ ] W019 [P] Vérifier le poids de `dist-web/` < 5 Mo (SC-W04 : ✅ 2,5 Mo dont 2,1 Mo de moteur, 0,8 Mo compressé) et le temps d'affichage d'un message de 25 Mo (SC-W03 : à mesurer, pas de fichier de cette taille dans le corpus)
- [x] W020 [P] README : section « Version web »
- [x] W021 Exécuter `quickstart.md`

---

## Dependencies & Execution Order

- Phase 1 → Phase 2 (bloquante) → US1 → US2 (la CSP peut se faire en parallèle de US1) → US3 → Polish
- W010, W011, W012 dépendent de W006/W007 ; W013 après W011

## Traçabilité

| Exigence | Tâches |
|---|---|
| FR-W01 | W005, W006, W010 |
| FR-W02 | W014, W015 |
| FR-W03 | W012 |
| FR-W04 | W008, W011 |
| FR-W05 | W004, W009 |
| FR-W06 | W016 |
| FR-W07 | W011 |
| FR-W08 | W017, W018 |
