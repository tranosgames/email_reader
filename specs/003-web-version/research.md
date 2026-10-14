# Research — 003 Version web locale

## W1 — Exécuter le moteur dans le navigateur

- **Decision** : compiler `mf-engine` (et donc `mf-eml`, `mf-msg`, `mf-analysis`) en WebAssembly
  (`wasm32-unknown-unknown`) via une crate `mf-wasm` (`wasm-bindgen` 0.2.129, cible `--target web`).
- **Rationale** : toutes les dépendances sont du Rust pur sans E/S système (`mail-parser`, `cfb`,
  `ammonia`/`html5ever`, `regex`, `linkify`, `sha2`, `md-5`, `encoding_rs`) ; le seul point
  dépendant du système, la lecture de fichier, est isolé dans `mf_core::read_evidence`. On ajoute
  `mf_engine::open_bytes` : la CLI, l'app et le web exécutent alors **le même code** (FR-W05).
- **Alternatives considered** : réécrire le parsing en JavaScript (msg-reader utilise
  `@kenjiuno/msgreader`) — rejeté : deux moteurs divergents, perte de la concordance garantie par
  les tests ; Pyodide/serveur — rejetés (poids, ou envoi des preuves).

## W2 — Réactivité

- **Decision** : le module WebAssembly tourne dans un **Web Worker** module ; le fil principal ne
  fait que l'affichage. Les octets sont transférés (`Transferable`) sans copie.
- **Rationale** : un MSG de plusieurs dizaines de Mo ne doit pas figer l'onglet (edge case spec).

## W3 — Garantie « zéro réseau »

- **Decision** : CSP injectée dans `index.html` au build web :
  `default-src 'none'; script-src 'self' 'wasm-unsafe-eval'; worker-src 'self'; connect-src 'self';
  style-src 'self' 'unsafe-inline'; img-src 'self' data:; font-src 'self'; frame-src 'self';
  base-uri 'none'; form-action 'none'`. `connect-src 'self'` sert uniquement au chargement du
  `.wasm` ; aucun code applicatif n'appelle `fetch` ensuite. Le corps HTML garde sa propre CSP
  `default-src 'none'` dans l'iframe sandbox (feature 001, R4).
- **Vérification** : test e2e Playwright qui enregistre toutes les requêtes et échoue si une
  requête part après le chargement ou hors de l'origine.

## W4 — Partage de l'interface

- **Decision** : une seule base de code d'interface (`apps/desktop/src`) ; `ipc.ts` choisit le
  backend au démarrage (Tauri si `__TAURI_INTERNALS__`, sinon WebAssembly). `FileHandler` utilise
  l'API de glisser-déposer HTML5 (`webkitGetAsEntry` pour les dossiers) et `<input type=file>`
  (`webkitdirectory` pour un dossier) côté web. Le mode « démo » JSON de la feature 001 devient
  inutile et est retiré.
- **Build** : `npm run build:web` → `dist-web/` (chemins relatifs, `base: "./"`), publiable tel quel.

## W5 — Déploiement

- **Decision** : workflow GitHub Actions `pages.yml` (comme msg-reader) : build wasm + build web →
  `actions/deploy-pages`.
