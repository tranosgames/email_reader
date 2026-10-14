# MailForensics

Outil d'analyse forensic de courriels **EML** et **MSG** (PST/OST prévu), en **Rust**, avec une
interface de bureau inspirée de [msg-reader](https://github.com/Rasalas/msg-reader) :
glisser-déposer, liste des messages, panneau de lecture, épinglage, tri — enrichie de vues
d'enquête (en-têtes et chaîne de relais, SPF/DKIM/DMARC, pièces jointes et imbrication, IOC,
propriétés MAPI brutes, rapport et journal d'audit).

Le projet est structuré avec [spec-kit](https://github.com/github/spec-kit) (Spec-Driven Development).

## Où en est-on

| Étape spec-kit | Feature 001 — EML/MSG | Feature 002 — PST |
|---|---|---|
| `/speckit-constitution` | ✅ `.specify/memory/constitution.md` v1.0.0 | (partagée) |
| `/speckit-specify` | ✅ `spec.md` + checklist | ✅ `spec.md` (brouillon) |
| `/speckit-clarify` | optionnel — voir « Décisions à confirmer » | — |
| `/speckit-plan` | ✅ `plan.md`, `research.md`, `data-model.md`, `contracts/`, `quickstart.md` | à faire |
| `/speckit-tasks` | ✅ `tasks.md` (90 tâches, 9 phases) | à faire |
| `/speckit-analyze` → `/speckit-implement` → `/speckit-converge` | prochaine étape | — |

## Arborescence

```text
.specify/                 # spec-kit : constitution, templates, scripts
.claude/skills/speckit-*  # commandes spec-kit pour Claude Code
specs/
├── 001-email-forensic-viewer/   # feature active (.specify/feature.json)
└── 002-pst-support/
spikes/crate-validation/  # prototype Phase 0 : valide mail-parser, cfb, msg_parser, outlook-pst
```

## Pile technique (voir `specs/001-email-forensic-viewer/research.md`)

- EML : `mail-parser` · MSG : `cfb` + couche MAPI maison (oracle de test : `msg_parser`) · PST : `outlook-pst` (Microsoft, lecture seule)
- UI : Tauri 2 + TypeScript vanilla + Tailwind (même découpage que msg-reader : `FileHandler`, `MessageHandler`, `UIManager`)
- CLI `mf` pour l'automatisation et la contre-vérification

## Continuer

```console
uv tool install specify-cli     # ou: pip install specify-cli
claude                          # dans ce dossier ; les skills /speckit-* sont déjà installés
/speckit-clarify                # facultatif
/speckit-analyze
/speckit-implement              # commence par la Phase 1 (Setup) puis le MVP (US1)
```

Prototype de validation des crates :

```console
cd spikes/crate-validation && cargo run -- samples/phishing-basic.eml chemin/vers/un.msg
```

## Décisions à confirmer

1. Vérification DKIM/S-MIME **en ligne** (DNS) exclue de la v1 pour rester hors-ligne ; seule la vérification hors-ligne du hash du corps est faite.
2. Pas d'enrichissement externe (WHOIS, VirusTotal) en v1.
3. Mono-analyste, pas de serveur.
4. Décodage TNEF (`winmail.dat`) après le MVP.
