# Quickstart — 003 Version web

```console
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.129 --locked
cd apps/desktop
npm install
npm run dev:web            # compile le moteur en wasm puis sert http://localhost:5173
npm run build:web          # → dist-web/, à publier sur n'importe quel serveur statique
npx vite preview --outDir dist-web
```

Validation :

1. Ouvrir la page, déposer `tests/corpus/` entier → mêmes messages, empreintes, en-têtes, fichiers et URL que l'application de bureau (et que `mf show --json`).
2. Couper le réseau (devtools › Offline), déposer à nouveau un fichier → fonctionne.
3. Onglet Réseau : aucune requête après le chargement initial.
4. `npm run test:web` (Playwright) : e2e + concordance CLI + zéro réseau.
