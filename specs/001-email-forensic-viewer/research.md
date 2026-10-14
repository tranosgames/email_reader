# Research — 001 Visionneuse forensic EML/MSG

**Date** : 2026-10-14 · **Méthode** : revue des crates (crates.io, dépôts) + prototype compilé
(`cargo run` sur un EML synthétique de phishing et deux MSG réels issus du jeu de test de `msg_parser`).

## R0 — Rust ou Go ?

- **Decision** : **Rust** (stable 1.97, édition 2024). Go n'est pas nécessaire.
- **Rationale** : chaque format visé a une implémentation Rust maintenue et permissive :
  EML → `mail-parser`, MSG → `cfb` (+ `msg_parser` en oracle), PST → `outlook-pst` (Microsoft).
  Le projet de référence msg-reader utilise lui-même Tauri (dossier `src-tauri`), donc un
  backend Rust est dans la continuité directe de son design.
- **Alternatives considered** : Go (`emersion/go-message` pour EML, `richardlehane/mscfb` pour
  MSG, pas d'équivalent de qualité à `outlook-pst` ; UI via Wails). Rejeté : couverture PST
  inférieure et aucun gain sur EML/MSG.

**Preuve (prototype)** :

| Test | Résultat |
|---|---|
| EML synthétique (RFC 2047 UTF-8 base64, 2 `Received`, PJ `facture.pdf.exe`) | Objet décodé `Urgent : mot de passe expiré`, 2 `Received` restitués **avec offsets d'octets**, PJ hachée SHA-256 |
| `attachment.msg` (Exchange) via `cfb` brut | 179 entrées OLE, 85 propriétés variables à la racine, objet lu en `0x0037001F` |
| `unicode.msg` via `cfb` brut | En-têtes de transport lus en `0x007D001F` (5 910 o, commence par `Return-path:`) |
| Même fichiers via `msg_parser` | Objet et PJ concordants **mais** l'expéditeur de `attachment.msg` est le DN X.500 Exchange (`/O=EXCHANGELABS/…`), pas l'adresse SMTP |
| `outlook-pst::open_store` | API disponible et compilée (pas d'échantillon PST à ce stade → feature 002) |

## R1 — Parsing EML

- **Decision** : `mail-parser` 0.11 (Stalwart Labs, Apache-2.0/MIT), feature `full_encoding`.
- **Rationale** : robuste (utilisé par le serveur Stalwart), zéro-copie, décode RFC 2047/2231,
  parse `Received` en structure typée (`HeaderValue::Received`), conserve l'ordre et les offsets
  des en-têtes (`offset_field`, `offset_start`, `offset_end`) → permet de citer l'octet source
  d'un constat (Constitution II). Gère `message/rfc822` imbriqué.
- **Alternatives considered** : `mailparse` (moins complet sur les encodages, pas d'offsets
  d'en-têtes), `lettre` (orienté envoi).

## R2 — Parsing MSG

- **Decision** : couche MAPI maison (`mf-msg`) au-dessus de `cfb` 0.15 (MIT) ; `msg_parser` 0.3
  uniquement comme **oracle** dans les tests de concordance (Constitution V, SC-007).
- **Rationale** : l'analyse forensic exige l'accès à *toutes* les propriétés MAPI brutes
  (explorateur de propriétés FR-031) et à des propriétés que `msg_parser` ne remonte pas
  correctement : `PR_SENDER_SMTP_ADDRESS` 0x5D01, `PR_SENT_REPRESENTING_SMTP_ADDRESS` 0x5D02,
  `PR_TRANSPORT_MESSAGE_HEADERS` 0x007D, `PR_CLIENT_SUBMIT_TIME` 0x0039,
  `PR_MESSAGE_DELIVERY_TIME` 0x0E06, `PR_CREATION_TIME` 0x3007, `PR_LAST_MODIFICATION_TIME` 0x3008,
  `PR_INTERNET_MESSAGE_ID` 0x1035, `PR_INTERNET_CPID` 0x3FDE, `PR_MESSAGE_FLAGS` 0x0E07,
  propriétés nommées (`__nameid_version1.0`). La structure MS-OXMSG est simple et bien documentée :
  - `__properties_version1.0` : propriétés de taille fixe (en-tête 32 o racine / 24 o embarqué / 8 o recip-attach, puis entrées de 16 o) ;
  - `__substg1.0_TTTTPPPP` : propriétés variables (`001F` UTF-16LE, `001E` ANSI + page de code, `0102` binaire, `000D` objet/stockage) ;
  - `__recip_version1.0_#XXXXXXXX`, `__attach_version1.0_#XXXXXXXX` ;
  - message embarqué : stockage `__substg1.0_3701000D` (attach method 5).
  Estimation : ~1 500 lignes, entièrement couvertes par le corpus.
- **Alternatives considered** : `msg_parser` seul (rejeté : perte d'information forensic, OLE
  reader propre moins éprouvé que `cfb` — 72 M de téléchargements, utilisé par `calamine`).

## R3 — Corps RTF des MSG

- **Decision** : `compressed-rtf` 1.0 pour la décompression `PR_RTF_COMPRESSED` (MS-OXRTFCP) ;
  dé-encapsulation HTML (MS-OXRTFEX, balises `\fromhtml1` / `\*\htmltag`) en maison ;
  `rtf-parser` 0.4 pour extraire un texte brut de secours.
- **Rationale** : beaucoup de MSG Outlook n'ont qu'un corps RTF contenant du HTML encapsulé.

## R4 — Rendu HTML sûr

- **Decision** : sanitisation côté Rust avec `ammonia` 4 (liste blanche, suppression scripts,
  handlers `on*`, `<form>`, `<iframe>`, `<object>`, URL `javascript:`), réécriture des `cid:` en
  `data:` URIs, **puis** affichage dans une `<iframe sandbox>` (sans `allow-scripts`, sans
  `allow-same-origin`) avec CSP `default-src 'none'; img-src data:; style-src 'unsafe-inline'`.
  Les ressources distantes sont retirées et comptées (FR-008). La CSP globale Tauri interdit
  toute connexion sortante (`connect-src ipc: http://ipc.localhost`).
- **Rationale** : défense en profondeur — même si la sanitisation rate un vecteur, le sandbox et
  la CSP empêchent exécution et fuite réseau (pixels de suivi, Constitution IV/VI).
- **Alternatives considered** : sanitisation JS (DOMPurify) côté webview — rejeté comme couche
  unique car la logique doit vivre dans le moteur testable (Constitution III) ; on peut l'ajouter
  en seconde couche.

## R5 — Authentification (SPF/DKIM/DMARC/ARC)

- **Decision** :
  - Résultats déclarés : parsing des en-têtes `Authentication-Results` (RFC 8601),
    `Received-SPF`, `ARC-*` en maison (grammaire simple) ;
  - DKIM : parsing des balises `DKIM-Signature` via `mail-auth` 0.13 (Stalwart) ; **recalcul
    hors-ligne de `bh=`** (canonicalisation `simple`/`relaxed` du corps, RFC 6376 §3.4.3–3.4.4,
    `l=` respecté) en maison, testé avec les vecteurs de la RFC.
  - Vérification cryptographique complète (clé publique DNS) : **hors v1** (Constitution VI),
    l'architecture laisse un point d'extension opt-in (`mail-auth` sait la faire avec un résolveur).
- **Rationale** : `bh=` valide ⇒ le corps n'a pas été altéré depuis la signature ; information
  forte sans réseau.

## R6 — Pièces jointes : type réel et alertes

- **Decision** : `infer` 0.22 pour la détection par signature ; détection maison de : OLE avec
  stockage `Macros`/`_VBA_PROJECT_CUR` (via `cfb`), OOXML avec `vbaProject.bin` (via `zip` 9),
  ZIP chiffré (bit 0 du flag général), double extension, caractères U+202E/U+200E et contrôles.
- **TNEF (`winmail.dat`)** : la crate `tnef` (0.1.1, 2019) est jugée non maintenue ; décodeur
  MS-OXTNEF maison **après le MVP** ; en attendant : détection + export brut (spec, Assumptions).

## R7 — Extraction d'IOC

- **Decision** : `linkify` 0.11 (URL/adresses dans le texte), parcours DOM `html5ever` (fourni par
  `ammonia`) pour `href`/`src`/`action`, regex IPv4/IPv6 validées par `std::net`, `idna` 1.1 pour
  punycode, `unicode-security` pour les confusables/scripts mixtes ; défangage maison.

## R8 — Hachage et intégrité

- **Decision** : `sha2` 0.11 + `md-5` 0.11 calculés en un seul passage de lecture en flux
  (fichiers ouverts en lecture seule, `O_RDONLY` / `FILE_SHARE_READ`, sans `mmap` en écriture).
  Le contenu lu est conservé en mémoire (`Arc<[u8]>`) pour que le parsing travaille sur **les
  mêmes octets** que ceux hachés (pas de relecture TOCTOU).
- **Rationale** : Constitution I ; MD5 conservé car encore demandé par de nombreux outils/rapports.

## R9 — Interface (design msg-reader)

- **Decision** : Tauri 2.12 + frontend Vite/TypeScript **vanilla** + Tailwind, en reprenant le
  découpage de msg-reader : `FileHandler` (glisser-déposer), `MessageHandler` (état), `UIManager`
  (rendu), `main` (orchestration). Ajout d'onglets forensic : Lecture · En-têtes · Pièces jointes ·
  Indicateurs · Brut/Propriétés · Notes.
- **Rationale** : continuité du design demandé, frontend léger sans framework (Constitution VII),
  toute la logique côté Rust ; Tauri fournit l'association de fichiers `.eml`/`.msg` comme
  msg-reader et des binaires Windows/macOS/Linux.
- **Alternatives considered** : Svelte/React (gain marginal pour 6 vues), egui natif (perte du
  rendu HTML), web pur comme la version GitHub Pages de msg-reader (impossible de garantir
  hors-ligne/lecture seule et de hacher de gros fichiers efficacement).

## R10 — Recherche, persistance, rapport

- **Decision** : index en mémoire (normalisation minuscule + recherche par sous-chaîne sur
  champs pré-extraits) ; dossier d'enquête en JSON (`*.mfcase.json`) ; rapport HTML généré par
  gabarit (`askama`) + JSON (`serde_json`), triés de façon déterministe (SC-005).
- **Alternatives considered** : `tantivy`/SQLite — rejetés tant que la cible reste ≤ quelques
  milliers de messages (Constitution VII) ; réévalués pour PST (feature 002).

## R11 — Qualité & chaîne d'approvisionnement

- **Decision** : `insta` (snapshots JSON du corpus), `cargo-fuzz` (1 cible par parseur),
  `proptest` pour les décodeurs, `cargo-deny` (licences + advisories), `clippy -D warnings`,
  `#![forbid(unsafe_code)]` dans les crates du projet.

## R12 — Préparation PST (feature 002)

- **Constat** : `outlook-pst` 1.2 (Microsoft, MIT, MS-PST) est lecture seule — idéal en forensic —
  mais expose des `Rc<dyn Store>` (non `Send`). Conséquence de conception dès la v1 : le moteur
  expose une **trait `EvidenceSource`** et chaque source lourde tourne sur un thread de travail
  dédié qui communique par messages ; l'UI ne voit que le modèle unifié.

## Sources

- msg-reader : https://github.com/Rasalas/msg-reader
- spec-kit : https://github.com/github/spec-kit
- mail-parser : https://crates.io/crates/mail-parser · cfb : https://crates.io/crates/cfb
- msg_parser : https://crates.io/crates/msg_parser · outlook-pst : https://github.com/microsoft/outlook-pst-rs
- MS-OXMSG, MS-OXRTFCP, MS-OXRTFEX, MS-OXTNEF, MS-PST (Microsoft Open Specifications) ; RFC 5322, 2045–2049, 2047, 6376, 7208, 7489, 8601, 8617
