# Contrat IPC — commandes Tauri (frontend ↔ moteur)

Couche mince (`apps/desktop/src-tauri/src/commands.rs`) qui délègue à `mf-core` (Constitution III).
Toutes les commandes sont `async`, renvoient `Result<T, IpcError>` sérialisé en JSON, et ne
transportent **jamais** de contenu binaire volumineux non demandé (PJ chargées à la demande).

`IpcError = { code: "io" | "parse" | "not_found" | "hash_mismatch" | "limit" | "invalid", message: string, evidence_id?: string }`

## Ingestion

| Commande | Entrée | Sortie | Notes |
|---|---|---|---|
| `ingest_paths` | `{ paths: string[], recursive: bool }` | `IngestTicket { job_id }` | Lance un job ; progression par événements |
| `list_messages` | `{ filter?: Filter, sort?: Sort }` | `MessageSummary[]` | Liste latérale (style msg-reader) |
| `get_message` | `{ message_id }` | `MessageView` | En-tête lisible, corps sûr, PJ (métadonnées), compteur ressources bloquées |
| `get_evidence` | `{ evidence_id }` | `Evidence` | Empreintes, statut, erreurs |

**Événements émis** (`app.emit`) :

- `ingest://progress` `{ job_id, done, total, current_path }`
- `ingest://item` `{ job_id, result: { kind: "ready", evidence_id, message_id } | { kind: "duplicate", of } | { kind: "failed", path, error } }`
- `ingest://done` `{ job_id, ready, duplicates, failed }`

## Analyse

| Commande | Entrée | Sortie |
|---|---|---|
| `get_headers` | `{ message_id }` | `{ headers: Header[], hops: Hop[], auth: AuthResult[], dkim: DkimCheck[] }` |
| `get_findings` | `{ message_id }` | `Finding[]` |
| `get_attachments` | `{ message_id }` | `Attachment[]` (sans `data`) |
| `get_iocs` | `{ message_ids: string[] }` | `Ioc[]` |
| `get_mapi_properties` | `{ message_id }` | `MapiProperty[]` |
| `get_raw` | `{ message_id, target: "source" \| { stream: string } \| { attachment: number }, offset: number, length: number }` | `{ bytes_b64, total_len }` (fenêtré, pour la vue hex) |
| `search` | `{ query: string, filter?: Filter }` | `MessageId[]` |
| `timeline` | `{ filter?: Filter }` | `{ message_id, utc, label }[]` |

`Filter = { pinned?, has_attachments?, min_severity?, sender_domain?, from_utc?, to_utc?, tags? }`
`Sort = { key: "date" | "sender" | "subject" | "severity", dir: "asc" | "desc" }`

## Annotations & audit

| Commande | Entrée | Sortie |
|---|---|---|
| `set_pin` | `{ message_id, pinned }` | `()` (journalisé) |
| `add_note` / `add_tag` / `remove_tag` | `{ message_id, text }` | `Annotation` |
| `get_audit_log` | `{}` | `AuditEntry[]` |

## Export (toujours précédé d'une revérification d'empreinte — FR-027)

| Commande | Entrée | Sortie |
|---|---|---|
| `export_attachments` | `{ message_id, indices: number[], dest_dir, neutralize: bool }` | `Manifest` |
| `export_iocs` | `{ message_ids, format: "csv" \| "json", defang: bool, dest_file }` | `{ count }` |
| `export_report` | `{ message_ids, format: "html" \| "json" \| "both", dest }` | `{ files: string[] }` |
| `save_case` / `open_case` | `{ path }` | `Case` / `{ case, reverification: { evidence_id, status }[] }` |

`dest_dir` / `dest_file` proviennent exclusivement du dialogue natif (`tauri-plugin-dialog`) ;
le backend refuse toute destination située dans le répertoire d'une preuve chargée.

## Capacités Tauri (`capabilities/default.json`)

Autorisées : `core:event`, `dialog:allow-open`, `dialog:allow-save`, commandes ci-dessus.
**Refusées** : `shell:*`, `http:*`, `fs:*` côté frontend (tout accès fichier passe par le moteur),
`opener:*` (aucune URL ni PJ ne s'ouvre depuis l'app).
