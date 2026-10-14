# Contrat — module WebAssembly `mf_wasm`

Généré par `wasm-bindgen --target web` dans `apps/desktop/src/wasm/` (non versionné).

| Fonction | Signature | Retour |
|---|---|---|
| `analyze` | `(file_name: string, bytes: Uint8Array) => string` | JSON `{ evidence: Evidence, message: Message }` — mêmes types que `mf show --json` / `get_message` (data-model 001) |
| `engine_version` | `() => string` | version du moteur (`mf-wasm` = version workspace) |

Erreurs : exception JS (`Error.message` = message `EngineError` en français, ex. « format non reconnu »).

`evidence.path` vaut le chemin relatif du fichier dans le dépôt (`dossier/sous/x.msg`) : le
navigateur n'expose pas le chemin absolu.

## Protocole du Web Worker (`engine.worker.ts`)

- Entrée : `{ id: number, name: string, bytes: ArrayBuffer }` (transféré)
- Sortie : `{ id, ok: true, view: MessageView }` ou `{ id, ok: false, error: string }`
- Message initial `{ ready: true, version }` quand le module est chargé.
