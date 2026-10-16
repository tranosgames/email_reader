---

description: "Task list for 001 — Visionneuse forensic EML/MSG"
---

# Tasks: Visionneuse forensic de courriels EML/MSG

**Input**: Design documents from `/specs/001-email-forensic-viewer/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/, quickstart.md

**Tests**: **inclus et obligatoires** — Constitution V (test-first sur corpus). Chaque test est écrit et doit ÉCHOUER avant l'implémentation correspondante.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (US1…US6)

## Path Conventions

Workspace Cargo : `crates/mf-*/src/`, app : `apps/desktop/`, tests transverses : `tests/`, corpus : `tests/corpus/`.

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Project initialization and basic structure

- [x] T001 Créer le workspace Cargo `Cargo.toml` (édition 2024, `resolver = "3"`, MSRV 1.85, `[workspace.dependencies]` épinglées selon plan.md) et les crates vides `crates/mf-core`, `crates/mf-eml`, `crates/mf-msg`, `crates/mf-analysis`, `crates/mf-report`, `crates/mf-cli`
- [ ] T002 [P] Ajouter `#![forbid(unsafe_code)]` et `#![deny(clippy::unwrap_used, clippy::panic)]` dans chaque `crates/*/src/lib.rs`
- [ ] T003 [P] Configurer `rustfmt.toml`, `clippy.toml`, `deny.toml` (licences permissives uniquement, advisories)
- [ ] T004 [P] Créer `.github/workflows/ci.yml` : fmt, clippy `-D warnings`, test, `cargo deny check`, matrice Linux/Windows/macOS
- [x] T005 [P] Initialiser l'app `apps/desktop` (Tauri 2.12 + Vite + TypeScript + Tailwind) avec `src/main.ts`, `FileHandler.ts`, `MessageHandler.ts`, `UIManager.ts` vides, modèle msg-reader
- [ ] T006 [P] Initialiser `fuzz/` avec `cargo fuzz init` (cibles vides `eml_parse`, `msg_parse`, `rtf_decompress`, `auth_results`)
- [ ] T007 Constituer le corpus initial `tests/corpus/` : `eml/phishing-basic.eml`, `eml/html-tracking-pixel.eml`, `eml/dkim-signed.eml`, `eml/multi-received.eml`, `msg/attachment.msg`, `msg/unicode.msg`, `msg/ascii.msg` (jeu de test MIT de msg_parser), `nested/msg-in-msg.msg`, `nested/eml-in-eml.eml`, `malformed/*` (tronqué, OLE corrompu, MIME sans fin) + `tests/corpus/README.md` (origine, licence, empreinte de chaque fichier)

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core infrastructure that MUST be complete before ANY user story can be implemented

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [ ] T008 Définir le modèle unifié (Evidence, Message, Header, Address, DecodedValue, DateValue, Attachment, Finding, SourceRef, Ioc, Hop, AuthResult, DkimCheck, MapiProperty) avec `serde` dans `crates/mf-core/src/model.rs` (data-model.md)
- [ ] T009 [P] Définir `Limits` et ses défauts dans `crates/mf-core/src/limits.rs`
- [ ] T010 [P] Définir les erreurs (`thiserror`) `ParseIssue`, `EngineError` dans `crates/mf-core/src/error.rs`
- [ ] T011 [P] Test : lecture d'un fichier en lecture seule + SHA-256/MD5 en un passage, contrôle contre vecteurs connus et `sha256sum`, fichier inchangé (mtime/contenu) dans `crates/mf-core/tests/hashing.rs`
- [ ] T012 Implémenter `hashing.rs` + `evidence.rs` (ouverture `read(true)` seule, `Arc<[u8]>`, détection de format par signature FR-002) dans `crates/mf-core/src/`
- [ ] T013 [P] Test : journal d'audit append-only (`seq` croissant, horodatage UTC, aucune suppression possible via l'API) dans `crates/mf-core/tests/audit.rs`
- [ ] T014 Implémenter `AuditLog` dans `crates/mf-core/src/audit.rs`
- [ ] T015 Définir le trait `EvidenceSource` (ouvrir une preuve → itérateur de `Message` + accès flux bruts) et le registre de parseurs dans `crates/mf-core/src/source.rs`
- [ ] T016 Implémenter l'orchestrateur d'ingestion (parallélisme `rayon`, isolement des erreurs FR-006, doublons FR-005, événements de progression) dans `crates/mf-core/src/ingest.rs`
- [ ] T017 [P] Créer le squelette CLI `mf` (clap, sous-commandes du contrat, `--json`, codes de sortie) dans `crates/mf-cli/src/main.rs` ; implémenter `mf hash`
- [ ] T018 [P] Mettre en place le harnais de snapshots corpus (`insta`, un snapshot JSON par fichier) dans `tests/integration/corpus.rs`

**Checkpoint**: Foundation ready - user story implementation can now begin in parallel

---

## Phase 3: User Story 1 - Ingérer et lire des preuves sans les altérer (Priority: P1) 🎯 MVP

**Goal**: déposer des EML/MSG, les voir listés avec empreintes, lire un message en rendu sûr.

**Independent Test**: quickstart scénarios 1 et 4.1–4.2 ; `sha256sum -c` OK après session.

### Tests for User Story 1 ⚠️

> **NOTE: Write these tests FIRST, ensure they FAIL before implementation**

- [ ] T019 [P] [US1] Test EML → Message (objet RFC 2047, adresses, date brute + UTC, corps texte/HTML, PJ) sur `tests/corpus/eml/*` dans `crates/mf-eml/tests/convert.rs`
- [ ] T020 [P] [US1] Test MSG : propriétés fixes (`__properties_version1.0`, en-têtes 32/24/8 o) et variables (`001F`, `001E` + page de code, `0102`) dans `crates/mf-msg/tests/props.rs`
- [ ] T021 [P] [US1] Test MSG → Message : expéditeur SMTP (0x5D01) **et** DN X.500, destinataires To/Cc/Bcc, `PR_CLIENT_SUBMIT_TIME`, corps texte/HTML ; concordance avec l'oracle `msg_parser` (SC-007) dans `crates/mf-msg/tests/convert.rs`
- [ ] T022 [P] [US1] Test sanitisation HTML : scripts, `on*`, `javascript:`, formulaires, iframes supprimés ; images/CSS distants retirés et comptés ; `cid:` → `data:` dans `crates/mf-analysis/tests/html.rs`
- [ ] T023 [P] [US1] Test d'intégrité transverse : ingestion + analyse + export du corpus complet, empreintes et mtime inchangés (SC-001) dans `tests/integration/integrity.rs`
- [ ] T024 [P] [US1] Test : un fichier malformé n'interrompt pas un lot et produit `Failed(reason)` ; doublon détecté (FR-005/006) dans `crates/mf-core/tests/ingest.rs`

### Implementation for User Story 1

- [ ] T025 [P] [US1] Implémenter la conversion `mail_parser::Message` → `Message` (en-têtes ordonnés + offsets, `DecodedValue`, `DateValue`) dans `crates/mf-eml/src/convert.rs` et `headers.rs`
- [ ] T026 [P] [US1] Implémenter le lecteur OLE (wrapper `cfb`, limites d'entrées) dans `crates/mf-msg/src/ole.rs`
- [ ] T027 [US1] Implémenter le décodage des propriétés MAPI fixes/variables et la table des noms canoniques dans `crates/mf-msg/src/props.rs` et `tags.rs` (dépend de T026)
- [ ] T028 [US1] Implémenter les destinataires (`__recip_version1.0_#*`, types 1/2/3, résolution SMTP/EX) dans `crates/mf-msg/src/recipients.rs`
- [ ] T029 [US1] Implémenter la conversion MSG → `Message` (corps texte/HTML, dates MAPI, `PR_INTERNET_CPID`) dans `crates/mf-msg/src/convert.rs`
- [ ] T030 [P] [US1] Implémenter le corps RTF : décompression (`compressed-rtf`), dé-encapsulation HTML MS-OXRTFEX, texte de secours (`rtf-parser`) dans `crates/mf-msg/src/rtf.rs`
- [ ] T031 [P] [US1] Implémenter la sanitisation HTML (`ammonia`, réécriture `cid:`, compteur de ressources bloquées) dans `crates/mf-analysis/src/html.rs`
- [ ] T032 [US1] Enregistrer les parseurs EML/MSG comme `EvidenceSource` dans `crates/mf-core/src/source.rs` ; implémenter `mf show`
- [ ] T033 [US1] Implémenter les commandes Tauri `ingest_paths`, `list_messages`, `get_message`, `get_evidence`, `set_pin` + événements `ingest://*` dans `apps/desktop/src-tauri/src/commands.rs` et `jobs.rs`
- [x] T034 [US1] Configurer `tauri.conf.json` (CSP `default-src 'none'`, associations `.eml`/`.msg`, pas d'updater) et `capabilities/default.json` (sans shell/http/fs/opener)
- [x] T035 [US1] Frontend `FileHandler.ts` : zone de dépôt plein écran (comme msg-reader), dialogue fichiers/dossier, ouverture par association
- [ ] T036 [US1] Frontend `MessageHandler.ts` + `UIManager.ts` : liste latérale (expéditeur, objet, date, badges PJ/alerte), tri, épingles, raccourcis ↑/↓/P, thèmes clair/sombre
- [ ] T037 [US1] Frontend `views/reading.ts` : en-tête lisible, sélecteur texte/HTML/RTF, iframe `sandbox` sans `allow-scripts`, bandeau ressources bloquées, liste des PJ (métadonnées) ; panneau empreintes de la preuve
- [ ] T038 [US1] Journaliser Ingest/Duplicate/ParseError/View/Pin dans le journal d'audit

### Ajouts clarification 2026-10-14 (FR-035, FR-036, FR-037) — dans le MVP

> Implémenté le 2026-10-14 : moteur (`mf-core`, `mf-eml`, `mf-msg`, `mf-analysis`, `mf-engine`), CLI `mf`
> (`show --headers`, `headers`, `attachments`, `urls`) et vue de lecture de l'app. Les tâches de
> socle touchées en chemin (T008, T012, T019–T026, T029, T031, T033, T036, T037) sont **partiellement**
> faites et restent ouvertes jusqu'à leur couverture complète.

- [x] T091 [P] [US1] Test liste d'URL par message : extraction depuis en-têtes, texte et attributs HTML (`href`, `src`, `action`), dédoublonnage, emplacements, défangage dans `crates/mf-analysis/tests/urls.rs`
- [x] T092 [US1] Implémenter l'extraction d'URL de base dans `crates/mf-analysis/src/urls.rs` (socle de T064, qui ajoutera domaines/IP/courriels)
- [x] T093 [P] [US1] Test liste des fichiers contenus EML et MSG (nom, type déclaré, taille, SHA-256, inline/attaché, message embarqué) dans `crates/mf-eml/tests/convert.rs` et `crates/mf-msg/tests/convert.rs`
- [x] T094 [US1] Implémenter la liste des PJ de base dans `crates/mf-eml/src/convert.rs` et `crates/mf-msg/src/attachments.rs` (socle de T055/T057)
- [x] T095 [US1] Frontend : bouton « Afficher / Masquer les en-têtes » + raccourci `H`, état mémorisé en session, dans `apps/desktop/src/views/reading.ts`
- [x] T096 [US1] Frontend : section « Fichiers contenus (n) » dans `apps/desktop/src/views/reading.ts`
- [x] T097 [US1] Frontend : section « URL (n) » avec emplacement, copie brute/défangée, jamais cliquable, dans `apps/desktop/src/views/reading.ts`

### Ajouts clarification 2026-10-14 (2) — FR-036 (MD5 visible), FR-038 (téléchargement confirmé)

- [x] T098 [P] [US1] Test : extraction d'une PJ (racine et message embarqué) identique octet pour octet, SHA-256 revérifié, nom assaini (`../`, U+202E, contrôles), objet MAPI embarqué refusé, dans `crates/mf-engine/tests/corpus.rs`
- [x] T099 [US1] `mf_engine::extract_attachment(bytes, path, index)` + `mf_core::safe_file_name` dans `crates/mf-engine/src/lib.rs`, `crates/mf-core/src/lib.rs`
- [x] T100 [P] [US1] CLI `mf extract <preuve> --index N [--path …] --out <dossier>` dans `crates/mf-cli/src/main.rs`
- [x] T101 [US1] Bureau : commande Tauri `save_attachment` (dialogue d'enregistrement natif, écriture par le moteur, refus si la destination est le dossier de la preuve) + confirmation `ask` dans `apps/desktop/src-tauri/src/main.rs`
- [x] T102 [US1] Web : fonction wasm `extract_attachment`, le worker conserve les octets de chaque preuve, confirmation `window.confirm`, téléchargement via Blob
- [x] T103 [US1] Interface : MD5 affiché en clair, bouton « Télécharger » par fichier, désactivé avec explication pour un objet MAPI, dans `apps/desktop/src/views/reading.ts`
- [x] T104 [US1] E2E web : annulation → aucun téléchargement ; confirmation → fichier téléchargé au SHA-256 attendu

### Ajouts clarification 2026-10-16 — FR-039 (détail des ressources bloquées)

- [x] T105 [P] [US1] Corpus `tests/corpus/eml/remote-resources.eml` (pixel, image de fond CSS et attribut, srcset, feuille de style, @import, script, cadre, vidéo, image cid:) + test d'inventaire attendu dans `crates/mf-engine/tests/corpus.rs`
- [x] T106 [US1] Modèle `BlockedResource` (numéro, URL, type, élément, attribut, ligne, repère) dans `crates/mf-core/src/model.rs`
- [x] T107 [US1] Inventaire dans `crates/mf-analysis/src/html.rs` : éléments conservés (repère dans le rendu) et éléments supprimés par la sanitisation (script, link, iframe, média…)
- [x] T108 [P] [US1] CLI : section « Ressources distantes bloquées » de `mf show`
- [x] T109 [US1] Interface : liste dépliable, repères numérotés dans le rendu, vue « Source HTML » numérotée, mention « chargement bloqué » dans la liste des URL
- [x] T110 [US1] E2E web : liste, repère trouvé dans le rendu, ligne atteinte dans la source, zéro requête

**Checkpoint**: User Story 1 fonctionnelle et testable seule — MVP « lecteur forensic sûr ».

---

## Phase 4: User Story 2 - Analyser les en-têtes et l'authenticité (Priority: P2)

**Goal**: en-têtes bruts, chaîne Received, SPF/DKIM/DMARC/ARC, constats sourcés.

**Independent Test**: quickstart scénario 2 ; snapshots de constats sur cas annotés.

### Tests for User Story 2 ⚠️

- [ ] T039 [P] [US2] Test chaîne `Received` (ordre chronologique, IP, délais, régression horaire, en-têtes repliés/mal formés) dans `crates/mf-analysis/tests/hops.rs`
- [ ] T040 [P] [US2] Test parsing `Authentication-Results` (RFC 8601, exemples de l'annexe B), `Received-SPF`, `ARC-Authentication-Results` dans `crates/mf-analysis/tests/auth.rs`
- [ ] T041 [P] [US2] Test DKIM : parsing des balises, recalcul `bh=` simple/relaxed avec `l=`, vecteurs RFC 6376 + `dkim-signed.eml` modifié d'un octet (→ mismatch) dans `crates/mf-analysis/tests/dkim.rs`
- [ ] T042 [P] [US2] Test des règles d'en-tête (`reply_to_domain_mismatch`, `return_path_domain_mismatch`, `sender_from_mismatch`, `display_name_contains_address`, `message_id_foreign_domain`, `date_in_future`, `date_missing`, `msg_no_transport_headers`, `auth_fail_*`) avec `SourceRef` attendues dans `crates/mf-analysis/tests/rules_headers.rs`
- [ ] T043 [P] [US2] Test MSG : en-têtes de transport 0x007D parsés en `Header` avec offsets relatifs à la propriété dans `crates/mf-msg/tests/transport_headers.rs`

### Implementation for User Story 2

- [ ] T044 [P] [US2] Implémenter `hops.rs` (à partir de `HeaderValue::Received` + repli regex) dans `crates/mf-analysis/src/hops.rs`
- [ ] T045 [P] [US2] Implémenter `auth.rs` (Authentication-Results, Received-SPF, ARC) dans `crates/mf-analysis/src/auth.rs`
- [ ] T046 [P] [US2] Implémenter `dkim.rs` (parsing via `mail-auth`, canonicalisation et `bh` maison, `signature_verified = NotChecked`) dans `crates/mf-analysis/src/dkim.rs`
- [ ] T047 [US2] Implémenter le moteur de règles (`Rule` trait, registre, sévérités) et les règles d'en-tête dans `crates/mf-analysis/src/rules/{mod.rs, headers.rs}`
- [ ] T048 [US2] Parser 0x007D en en-têtes dans `crates/mf-msg/src/convert.rs` (réutilise `mail-parser` sur le bloc d'en-têtes)
- [ ] T049 [US2] Implémenter `mf headers` et `mf findings` dans `crates/mf-cli/src/main.rs`
- [ ] T050 [US2] Commandes Tauri `get_headers`, `get_findings` + `views/headers.ts` (tableau des en-têtes bruts, frise des sauts avec délais, cartes SPF/DKIM/DMARC, liste des constats cliquables vers la source)

**Checkpoint**: US1 et US2 fonctionnent indépendamment.

---

## Phase 5: User Story 3 - Examiner pièces jointes et messages imbriqués (Priority: P2)

**Goal**: PJ avec types réels, empreintes, alertes ; imbrication récursive ; export neutralisé.

**Independent Test**: quickstart scénario 3 ; `nested/msg-in-msg.msg` navigable sur 3 niveaux.

### Tests for User Story 3 ⚠️

- [ ] T051 [P] [US3] Test PJ MSG (`__attach_version1.0_#*`, 0x3707/0x3704, 0x3705, 0x3712 Content-ID, données 0x3701) dans `crates/mf-msg/tests/attachments.rs`
- [ ] T052 [P] [US3] Test imbrication récursive EML/MSG/mixte, limite de profondeur, provenance (`PJ 2 › PJ 1`) dans `tests/integration/nested.rs`
- [ ] T053 [P] [US3] Test des alertes PJ (double extension, extension à risque, MIME ≠ réel, U+202E, ZIP chiffré, macros OLE/OOXML, TNEF, S/MIME, taille nulle) dans `crates/mf-analysis/tests/attachments.rs`
- [ ] T054 [P] [US3] Test export : neutralisation, assainissement `../` et caractères de contrôle, manifeste, refus si destination = répertoire d'une preuve, revérification d'empreinte préalable dans `crates/mf-report/tests/export.rs`

### Implementation for User Story 3

- [ ] T055 [P] [US3] Implémenter les PJ MSG + message embarqué (stockage `__substg1.0_3701000D`, en-tête de propriétés 24 o) dans `crates/mf-msg/src/attachments.rs`
- [ ] T056 [P] [US3] Implémenter `message/rfc822` récursif + limite de profondeur dans `crates/mf-eml/src/convert.rs`
- [ ] T057 [US3] Implémenter la détection de type réel et les alertes PJ (`infer`, `cfb`, `zip`) dans `crates/mf-analysis/src/attachments.rs`
- [ ] T058 [US3] Implémenter l'export de PJ + `manifest.json` (contrat cli.md) dans `crates/mf-report/src/export.rs` ; `mf attachments`, `mf extract`
- [ ] T059 [US3] Commandes Tauri `get_attachments`, `export_attachments` + `views/attachments.ts` (tableau, badges d'alerte, ouverture d'un message embarqué comme onglet avec fil d'Ariane, export via dialogue natif)
- [ ] T060 [US3] Journaliser ExportAttachments avec empreintes

**Checkpoint**: US1–US3 fonctionnent indépendamment.

---

## Phase 6: User Story 4 - Extraire les indicateurs de compromission (Priority: P3)

**Goal**: IOC dédoublonnés et sourcés, alertes de liens, export CSV/JSON défangé.

**Independent Test**: comparaison avec `tests/corpus/expected/*.iocs.json` (rappel 100 %).

### Tests for User Story 4 ⚠️

- [ ] T061 [P] [US4] Test extraction URL/domaines/IPv4/IPv6/courriels depuis en-têtes, texte, HTML (href/src/action), noms de PJ ; dédoublonnage et emplacements dans `crates/mf-analysis/tests/iocs.rs`
- [ ] T062 [P] [US4] Test règles `link_text_target_mismatch`, `url_raw_ip`, `idn_homograph` (punycode, scripts mixtes, confusables) dans `crates/mf-analysis/tests/rules_links.rs`
- [ ] T063 [P] [US4] Test défangage/réfangage aller-retour et exports CSV/JSON multi-messages dans `crates/mf-analysis/tests/defang.rs`

### Implementation for User Story 4

- [ ] T064 [P] [US4] Implémenter `iocs.rs` (`linkify`, parcours DOM html5ever, `std::net`, `idna`) dans `crates/mf-analysis/src/iocs.rs`
- [ ] T065 [P] [US4] Implémenter `defang.rs` dans `crates/mf-analysis/src/defang.rs`
- [ ] T066 [US4] Implémenter les règles de liens (`unicode-security`) dans `crates/mf-analysis/src/rules/links.rs`
- [ ] T067 [US4] Implémenter l'export IOC dans `crates/mf-report/src/export.rs` ; `mf iocs`
- [ ] T068 [US4] Commandes Tauri `get_iocs`, `export_iocs` + `views/iocs.ts` (groupés par type, copie défangée, sélection multi-messages)

**Checkpoint**: US1–US4 fonctionnent indépendamment.

---

## Phase 7: User Story 5 - Rapport, notes et journal d'audit (Priority: P3)

**Goal**: notes/étiquettes, dossier d'enquête, rapport HTML+JSON déterministe.

**Independent Test**: quickstart scénario 4.4–4.5 ; deux exports identiques hors `exported_utc`.

### Tests for User Story 5 ⚠️

- [ ] T069 [P] [US5] Test de conformité du rapport JSON au schéma `contracts/report.schema.json` (`jsonschema` en dev-dependency) dans `crates/mf-report/tests/schema.rs`
- [ ] T070 [P] [US5] Test de déterminisme : deux exports → identiques hors `exported_utc` (SC-005) dans `tests/integration/determinism.rs`
- [ ] T071 [P] [US5] Test dossier d'enquête : sauvegarde/réouverture, clé d'annotation stable, preuve modifiée → `HashMismatch` et export bloqué (FR-027/028) dans `crates/mf-core/tests/case.rs`

### Implementation for User Story 5

- [ ] T072 [P] [US5] Implémenter `Case` + annotations (notes, étiquettes, épingles) dans `crates/mf-core/src/case.rs`
- [ ] T073 [US5] Implémenter le rapport JSON trié (schéma v1) dans `crates/mf-report/src/json.rs`
- [ ] T074 [US5] Implémenter le rapport HTML imprimable (gabarit `askama`, CSS d'impression, sans ressource externe) dans `crates/mf-report/src/html.rs` et `templates/report.html`
- [ ] T075 [US5] Revérification d'empreinte avant chaque export (PJ, IOC, rapport) — point d'entrée unique dans `crates/mf-report/src/lib.rs` ; `mf report`
- [ ] T076 [US5] Commandes Tauri `add_note`, `add_tag`, `remove_tag`, `get_audit_log`, `export_report`, `save_case`, `open_case` + `views/notes.ts` et vue journal d'audit

**Checkpoint**: US1–US5 fonctionnent indépendamment.

---

## Phase 8: User Story 6 - Rechercher, filtrer, chronologie et vue brute (Priority: P3)

**Goal**: recherche/filtres, chronologie, source brute, explorateur MAPI, hexdump.

**Independent Test**: 200 messages chargés, recherche/filtres < 1 s ; propriétés MAPI concordantes.

### Tests for User Story 6 ⚠️

- [ ] T077 [P] [US6] Test recherche/filtres (épinglé, PJ, sévérité, domaine, période, étiquettes) sur un lot synthétique de 1 000 messages avec budget de temps dans `crates/mf-core/tests/search.rs`
- [ ] T078 [P] [US6] Test explorateur MAPI : liste complète des propriétés (racine, destinataires, PJ), propriétés nommées (`__nameid_version1.0`) résolues dans `crates/mf-msg/tests/named.rs`
- [ ] T079 [P] [US6] Test lecture fenêtrée des flux bruts (offset/longueur, bornes) dans `crates/mf-core/tests/raw.rs`

### Implementation for User Story 6

- [ ] T080 [P] [US6] Implémenter l'index et les filtres en mémoire dans `crates/mf-core/src/search.rs`
- [ ] T081 [P] [US6] Implémenter les propriétés nommées dans `crates/mf-msg/src/named.rs` ; `mf props`, `mf raw`
- [ ] T082 [US6] Commandes Tauri `search`, `timeline`, `get_mapi_properties`, `get_raw` + `views/timeline.ts`, `views/raw.ts` (source brute, table des propriétés, hexdump virtualisé)
- [ ] T083 [US6] Barre de recherche et panneau de filtres dans `UIManager.ts` (remplit la « Future Improvements » de msg-reader : recherche, filtres épinglés/PJ/expéditeur/objet)

**Checkpoint**: toutes les user stories sont fonctionnelles indépendamment.

---

## Phase 9: Polish & Cross-Cutting Concerns

**Purpose**: Improvements that affect multiple user stories

- [ ] T084 [P] Cibles de fuzzing réelles `eml_parse`, `msg_parse`, `rtf_decompress`, `auth_results` dans `fuzz/fuzz_targets/` + job CI nocturne ; crashs → `tests/corpus/malformed/`
- [ ] T085 [P] Benchmarks (`criterion`) ingestion 500 messages / affichage 25 Mo (SC-002) dans `crates/mf-core/benches/`
- [ ] T086 [P] Test « zéro réseau » : lancement de l'app sous proxy refusant tout, ouverture du corpus, aucune connexion tentée (SC-004) dans `apps/desktop/tests/e2e/no-network.spec.ts`
- [ ] T087 [P] Décodeur TNEF (MS-OXTNEF) pour `winmail.dat` dans `crates/mf-msg/src/tnef.rs` (post-MVP, cf. research R6)
- [ ] T088 [P] Documentation utilisateur `docs/guide-analyste.md` et `README.md` (installation, gatekeeper/SmartScreen comme msg-reader)
- [ ] T089 Revue de sécurité ciblée : hachage, audit, sanitisation, capacités Tauri (Constitution — revue obligatoire)
- [ ] T090 Exécuter l'intégralité de `quickstart.md` et consigner le résultat

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)** : aucune dépendance
- **Foundational (Phase 2)** : dépend de Phase 1 — BLOQUE toutes les user stories
- **User Stories (Phases 3–8)** : dépendent de Phase 2 ; ensuite parallélisables ou dans l'ordre P1 → P2 → P3
- **Polish (Phase 9)** : après les stories souhaitées (T087 indépendante)

### User Story Dependencies

- **US1 (P1)** : après Phase 2 — aucune dépendance sur d'autres stories
- **US2 (P2)** : après Phase 2 ; réutilise l'affichage US1 pour l'UI mais le moteur et la CLI sont testables seuls
- **US3 (P2)** : après Phase 2 ; l'export utilise la revérification (T075 de US5) → en attendant, T058 appelle directement `hashing::verify`, consolidé en T075
- **US4 (P3)** : après Phase 2 ; extraction HTML s'appuie sur T031 (US1)
- **US5 (P3)** : après Phase 2 ; le rapport agrège ce que US2–US4 ont produit si disponible (sections vides sinon)
- **US6 (P3)** : après Phase 2 ; l'explorateur MAPI réutilise T027

### Within Each User Story

- Tests écrits et ROUGES avant l'implémentation
- Modèle → parseurs → analyse → export → CLI → IPC → UI
- Story terminée et validée avant de passer à la priorité suivante (en solo)

### Parallel Opportunities

- T002–T006 en parallèle ; T009–T011, T013, T017, T018 en parallèle
- Dans US1 : T019–T024 en parallèle ; puis T025 ‖ T026 ‖ T030 ‖ T031
- `mf-eml` et `mf-msg` sont des crates distinctes : deux développeurs peuvent avancer en parallèle dès Phase 2

---

## Parallel Example: User Story 1

```bash
# Tests US1 en parallèle :
Task: "T019 Test EML → Message dans crates/mf-eml/tests/convert.rs"
Task: "T020 Test propriétés MAPI dans crates/mf-msg/tests/props.rs"
Task: "T022 Test sanitisation HTML dans crates/mf-analysis/tests/html.rs"

# Implémentations indépendantes :
Task: "T025 Conversion EML dans crates/mf-eml/src/convert.rs"
Task: "T026 Lecteur OLE dans crates/mf-msg/src/ole.rs"
Task: "T031 Sanitisation HTML dans crates/mf-analysis/src/html.rs"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Phase 1 Setup → Phase 2 Foundational
2. Phase 3 US1 → **STOP et VALIDER** (quickstart 1 + 4.1–4.2)
3. Utilisable immédiatement comme lecteur forensic sûr remplaçant msg-reader dans l'enquête

### Incremental Delivery

1. MVP (US1)
2. + US2 (en-têtes/auth) → le vrai gain forensic
3. + US3 (PJ/imbrication) → IOC de charge utile
4. + US4, US5, US6
5. Puis feature `002-pst-support`

---

## Traçabilité exigences → tâches

| Exigence | Tâches | | Exigence | Tâches |
|---|---|---|---|---|
| FR-001 | T035, T034 (associations) | | FR-018 | T052, T055, T056, T059 |
| FR-002 | T012 | | FR-019 | T054, T058 |
| FR-003 | T011, T012 | | FR-020 | T034 (pas d'`opener`), T089 |
| FR-004 | T011, T012, T023 | | FR-021 | T061, T064 |
| FR-005 | T016, T024 | | FR-022 | T062, T066 |
| FR-006 | T016, T024 | | FR-023 | T063, T065, T067, T068 |
| FR-007 | T019, T021, T025, T029, T030, T037 | | FR-024 | T013, T014, T038, T060 |
| FR-008 | T022, T031, T034, T037, T086 | | FR-025 | T072, T076 |
| FR-009 | T022, T031 | | FR-026 | T069, T073, T074 |
| FR-010 | T035, T036 | | FR-027 | T071, T075 |
| FR-011 | T043, T048, T050 | | FR-028 | T071, T072, T076 |
| FR-012 | T039, T044 | | FR-029 | T077, T080, T083 |
| FR-013 | T040, T045 | | FR-030 | T082 |
| FR-014 | T041, T046 | | FR-031 | T078, T079, T081, T082 |
| FR-015 | T042, T047 | | FR-032 | T017, T032, T049, T058, T067, T075, T081 |
| FR-016 | T051, T055, T057 | | FR-033 | T034, T086 |
| FR-017 | T053, T057 | | FR-034 | T009, T024, T084 |
| FR-035 | T095 | | FR-036 | T093, T094, T096 |
| FR-037 | T091, T092, T097 | | FR-038 | T098–T104 |
| FR-039 | T105–T110 | | | |

---

## Notes

- [P] = fichiers différents, pas de dépendance
- Chaque PR cite ses `T0xx` et `FR-0xx`
- Commit après chaque tâche ou groupe logique
- Ne jamais committer de preuve réelle d'un dossier client dans le corpus
