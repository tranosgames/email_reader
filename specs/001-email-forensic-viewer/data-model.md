# Data Model — 001 Visionneuse forensic EML/MSG

Modèle unifié, **indépendant du format source** (Constitution III). Il vit dans `crates/mf-core`
et est sérialisable en JSON (`serde`). Les identifiants sont des `Uuid` v7 sauf mention contraire.
Tous les horodatages produits par l'outil sont en UTC (RFC 3339) ; les dates issues d'une preuve
conservent leur valeur brute en plus de la valeur normalisée.

## Vue d'ensemble

```text
Case 1──* Evidence 1──1 Message (racine)
                          Message 1──* Header
                          Message 1──* Hop
                          Message 1──* AuthResult
                          Message 1──* Attachment 0..1──1 Message (embarqué, récursif)
                          Message 1──* Finding ──* SourceRef
                          Message 1──* Ioc ──* SourceRef
Case 1──* AuditEntry      Case 1──* Annotation (note/étiquette/épingle par MessageId)
```

## Evidence

| Champ | Type | Règles |
|---|---|---|
| `id` | Uuid | |
| `path` | PathBuf | chemin absolu tel que fourni ; jamais modifié |
| `size` | u64 | |
| `fs_created`, `fs_modified`, `fs_accessed` | Option<DateTime<Utc>> | métadonnées système lues, pas écrites |
| `sha256` | `[u8; 32]` (hex en JSON) | calculé avant parsing (FR-003) |
| `md5` | `[u8; 16]` | idem |
| `format` | enum `Eml \| Msg \| Pst \| Unknown` | par signature (FR-002) : `D0 CF 11 E0` → OLE puis présence `__properties_version1.0` → Msg ; `!BDN` → Pst ; sinon heuristique RFC 5322 |
| `declared_extension` | String | pour constat « extension ≠ format » |
| `status` | `EvidenceStatus` | voir transitions |
| `root_message` | Option<MessageId> | |
| `errors` | Vec<ParseIssue> | non bloquants |

**`EvidenceStatus`** :

```text
Hashing ──► Parsing ──► Ready
   │           └──────► Failed(reason)        (fichier conservé dans la liste, FR-006)
   └──────────────────► Failed(io)
Ready ──(revérification avant export / réouverture)──► Ready | HashMismatch   (export bloqué, FR-027)
```

`Duplicate(of: EvidenceId)` est retourné à l'ingestion au lieu de créer l'objet (FR-005).

## Message

| Champ | Type | Notes |
|---|---|---|
| `id` | MessageId (Uuid) | |
| `evidence_id` | EvidenceId | |
| `provenance` | Vec<ProvenanceStep> | `[]` pour la racine ; sinon `[Attachment{index, name}, …]` (FR-018) |
| `source_kind` | `Eml \| Msg \| PstItem` | |
| `subject` | DecodedValue | |
| `from`, `sender`, `reply_to`, `return_path` | Option<Address> / Vec<Address> | |
| `to`, `cc`, `bcc` | Vec<Address> | MSG : depuis la table des destinataires (type 1/2/3) |
| `date` | DateValue | `Date:` (EML) / `PR_CLIENT_SUBMIT_TIME` (MSG) |
| `msg_times` | Option<MsgTimes> | MSG : `delivery`, `creation`, `last_modification` |
| `message_id`, `in_reply_to`, `references` | Option<String> / Vec<String> | |
| `headers` | Vec<Header> | ordre d'origine (FR-011) |
| `bodies` | Bodies | `text`, `html` (brut), `html_safe` (sanitisé), `rtf` (décompressé), `body_source` |
| `attachments` | Vec<Attachment> | |
| `hops` | Vec<Hop> | ordre chronologique (FR-012) |
| `auth` | Vec<AuthResult> | |
| `dkim` | Vec<DkimCheck> | |
| `findings` | Vec<Finding> | |
| `iocs` | Vec<Ioc> | |
| `mapi` | Option<Vec<MapiProperty>> | MSG / PST uniquement (FR-031) |
| `raw_size` | u64 | |
| `blocked_remote_resources` | u32 | FR-008 |

**`DecodedValue`** : `{ raw: Bytes, decoded: String, charset: Option<String>, issues: Vec<ParseIssue> }` — la valeur brute est toujours conservée (Constitution I).

**`DateValue`** : `{ raw: String, utc: Option<DateTime<Utc>>, original_offset_minutes: Option<i16> }`.

**`Address`** : `{ display_name: Option<String>, address: Option<String>, x500_dn: Option<String>, address_type: Option<String> /* SMTP, EX */ }`. Pour un MSG, `address` = adresse SMTP résolue (0x5D01, 0x39FE…), `x500_dn` = adresse EX d'origine.

## Header

| Champ | Type |
|---|---|
| `index` | u32 (position dans la liste) |
| `name` | String |
| `raw` | Bytes (ligne(s) pliée(s) incluses) |
| `decoded` | String |
| `offset` | Option<ByteRange> — dans la preuve (EML) ou dans la propriété 0x007D (MSG) |

## Hop

`{ index, from_host, from_helo, from_ip: Option<IpAddr>, by_host, with_proto, id, for_addr, timestamp: DateValue, delay_from_previous: Option<Duration>, header_ref: SourceRef }`.

Règles : tri du plus ancien (dernier en-tête `Received`) au plus récent ; `delay_from_previous < 0`
→ constat `hop_time_regression` ; délai > 24 h → constat `hop_long_delay`.

## AuthResult / DkimCheck

- `AuthResult { mechanism: Spf|Dkim|Dmarc|Arc|Other, result: String /* pass, fail, softfail… */, authserv_id: String, properties: Map<String,String>, header_ref: SourceRef }`.
- `DkimCheck { domain, selector, algorithm, canonicalization, signed_headers: Vec<String>, body_length_limit: Option<u64>, bh_declared: String, bh_computed: Option<String>, body_hash_ok: Option<bool>, signature_verified: NotChecked /* v1 */, header_ref }`.

## Attachment

| Champ | Type | Règles |
|---|---|---|
| `index` | u32 | |
| `filename` | DecodedValue | `long_filename` prioritaire (MSG 0x3707), sinon 0x3704 |
| `declared_mime` | Option<String> | |
| `detected_mime` | Option<String> | par signature (`infer`) |
| `size` | u64 | |
| `sha256`, `md5` | hashes | sur le contenu décodé |
| `disposition` | `Inline \| Attachment \| Unknown` | |
| `content_id` | Option<String> | |
| `msg_attach_method` | Option<u32> | MSG 0x3705 |
| `embedded` | Option<MessageId> | si `message/rfc822` ou méthode 5 |
| `flags` | Vec<AttachmentFlag> | `DoubleExtension`, `RiskyExtension`, `MimeMismatch`, `BidiOrControlChars`, `EncryptedArchive`, `Macros`, `Tnef`, `SmimeSignature`, `SmimeEncrypted`, `ZeroLength` |
| `data` | non sérialisé | `Arc<[u8]>` ou référence paresseuse vers le flux source |

## Finding

`{ rule_id: &'static str, severity: Info|Low|Medium|High, title, description, refs: Vec<SourceRef> }`.

**`SourceRef`** : `{ kind: Header{index,name} | MapiProp{tag} | Body{part} | Attachment{index} | ByteRange{start,end}, excerpt: Option<String> }` — Constitution II.

Règles v1 (identifiants stables, utilisés dans les tests) :

| rule_id | Sévérité | Déclencheur |
|---|---|---|
| `reply_to_domain_mismatch` | Medium | domaine `Reply-To` ≠ domaine `From` |
| `return_path_domain_mismatch` | Low | domaine `Return-Path` ≠ `From` |
| `sender_from_mismatch` | Low | `Sender` présent et ≠ `From` |
| `display_name_contains_address` | Medium | nom affiché contient une adresse ≠ adresse réelle |
| `message_id_foreign_domain` | Low | domaine du `Message-ID` ≠ domaine `From` |
| `hop_time_regression` | Medium | horodatage d'un saut antérieur au précédent |
| `hop_long_delay` | Info | délai > 24 h |
| `date_in_future` / `date_missing` | Medium / Low | par rapport au dernier saut ou à l'horloge |
| `msg_no_transport_headers` | Info | MSG sans 0x007D |
| `auth_fail_spf` / `auth_fail_dkim` / `auth_fail_dmarc` | High | verdict déclaré `fail` |
| `dkim_body_hash_mismatch` | High | `bh` recalculé ≠ déclaré |
| `link_text_target_mismatch` | Medium | texte de lien = URL/domaine ≠ cible |
| `url_raw_ip` | Medium | URL vers IP littérale |
| `idn_homograph` | High | domaine confusable / scripts mixtes |
| `attachment_*` | selon flag | reprise des `AttachmentFlag` |
| `nesting_truncated` | Info | profondeur > limite |

## Ioc

`{ kind: Url|Domain|Ipv4|Ipv6|Email|Hash, value, defanged, locations: Vec<SourceRef>, messages: Vec<MessageId> }` — unicité sur `(kind, value normalisée)`.

## MapiProperty (MSG / PST)

`{ tag: u32, id: u16, ptype: u16, name: Option<&'static str>, named: Option<NamedPropKey>, source: FixedStream|SubStorage, value: MapiValue, raw: Bytes }`.

## Case (dossier d'enquête) — fichier `*.mfcase.json`

`{ schema_version: 1, case_id, title, created_utc, tool_version, evidence: Vec<{path, sha256, md5, size}>, annotations: Vec<Annotation>, audit: Vec<AuditEntry> }`.

- `Annotation { message_key: (evidence_sha256, provenance), kind: Note(String) | Tag(String) | Pin, created_utc }` — la clé est stable entre sessions (pas d'Uuid volatil).
- `AuditEntry { ts_utc, seq: u64, action: Ingest|Duplicate|ParseError|View|Pin|Unpin|Note|Tag|ExportAttachments|ExportIocs|ExportReport|HashVerify|HashMismatch|CaseSave|CaseOpen, target: Option<String>, sha256: Option<String>, detail: Option<String>, tool_version }` — append-only, `seq` strictement croissant.

## Limites (FR-034)

| Paramètre | Défaut |
|---|---|
| Taille max d'une preuve | 2 Gio (au-delà : refus explicite) |
| Profondeur d'imbrication | 10 |
| Parties MIME par message | 10 000 |
| En-têtes par message | 5 000 |
| Taille décompressée RTF | 64 Mio |
| Entrées OLE | 100 000 |
