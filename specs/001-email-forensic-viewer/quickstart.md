# Quickstart — 001 Visionneuse forensic EML/MSG

Guide de validation de bout en bout : il sert à la fois de démarrage développeur et de recette
(dernière tâche de `tasks.md`).

## Prérequis

- Rust stable ≥ 1.85 (`rustup default stable`), `cargo install tauri-cli --version "^2" cargo-deny cargo-fuzz`
- Node.js ≥ 20 (frontend Vite)
- Linux : `libwebkit2gtk-4.1-dev libsoup-3.0-dev` ; Windows : WebView2 ; macOS : Xcode CLT

## Construire et tester le moteur

```console
cargo build --workspace
cargo test --workspace                 # unitaires + corpus (snapshots insta)
cargo clippy --workspace -- -D warnings
cargo deny check
```

## Scénario 1 — Intégrité (US1, SC-001)

```console
sha256sum tests/corpus/eml/phishing-basic.eml > /tmp/avant
cargo run -p mf-cli -- hash tests/corpus/eml/phishing-basic.eml
cargo run -p mf-cli -- report tests/corpus/eml/phishing-basic.eml --out /tmp/r --format both
sha256sum -c /tmp/avant               # doit afficher OK
```

L'empreinte affichée par `mf hash` doit être identique à `sha256sum`.

## Scénario 2 — En-têtes et constats (US2)

```console
cargo run -p mf-cli -- headers tests/corpus/eml/phishing-basic.eml
cargo run -p mf-cli -- findings tests/corpus/eml/phishing-basic.eml
```

Attendu : 2 sauts `Received` en ordre chronologique (198.51.100.4 puis 203.0.113.7), constats
`reply_to_domain_mismatch` (medium) et `message_id_foreign_domain` absent (même domaine).

## Scénario 3 — MSG, PJ et imbrication (US3)

```console
cargo run -p mf-cli -- attachments tests/corpus/msg/attachment.msg --tree
cargo run -p mf-cli -- props tests/corpus/msg/unicode.msg --tag 0x007D001F
cargo run -p mf-cli -- extract tests/corpus/eml/phishing-basic.eml --out /tmp/pj --neutralize
```

Attendu : arbre des PJ avec empreintes ; propriété des en-têtes de transport commençant par
`Return-path:` ; `/tmp/pj/001_facture.pdf.exe_` + `manifest.json`, alertes `DoubleExtension`
et `MimeMismatch`.

## Scénario 4 — Application de bureau (US1–US6)

```console
cd apps/desktop && npm install && cargo tauri dev
```

1. Glisser `tests/corpus/` entier dans la fenêtre → liste triée par date, compteur d'erreurs pour les fichiers `malformed/`.
2. Ouvrir `html-tracking-pixel.eml` → bandeau « ressources distantes bloquées », aucune requête dans l'onglet réseau des devtools.
3. Onglets En-têtes / Pièces jointes / Indicateurs / Brut : vérifier les valeurs des scénarios 2–3.
4. Épingler (touche `P`), ajouter une note, exporter le rapport → le journal d'audit contient Ingest, Pin, Note, HashVerify, ExportReport.
5. Modifier une copie d'une preuve sur disque, rouvrir le dossier d'enquête → statut « empreinte divergente », export bloqué.

## Scénario 5 — Robustesse (SC-003)

```console
cargo fuzz run eml_parse -- -max_total_time=600
cargo fuzz run msg_parse -- -max_total_time=600
```

Aucun crash ; les cas trouvés sont ajoutés à `tests/corpus/malformed/`.
