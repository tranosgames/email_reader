# Feature Specification: Lecture forensic des boîtes PST/OST

**Feature Branch**: `002-pst-support`

**Created**: 2026-10-14

**Status**: Draft — à planifier après livraison de `001-email-forensic-viewer`

**Input**: User description: "Potentiellement dans le futur manipuler des fichiers PST."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Ouvrir une boîte PST et parcourir ses dossiers (Priority: P1)

L'analyste dépose un fichier `.pst` (ou `.ost`) de plusieurs Go. L'outil le hache, affiche
l'arborescence des dossiers (Boîte de réception, Éléments envoyés, Éléments supprimés, dossiers
personnalisés, dossiers de recherche) avec le nombre d'éléments, puis liste les messages d'un
dossier sélectionné dans la même interface que pour les EML/MSG. Chaque message s'ouvre avec
toutes les vues existantes (en-têtes, PJ, IOC, propriétés brutes).

**Why this priority**: Les PST sont la forme habituelle des boîtes saisies lors d'une enquête ;
sans navigation, aucune autre capacité PST n'a de valeur.

**Independent Test**: Ouvrir un PST de corpus ; le nombre de messages par dossier correspond à un
outil de référence ; le fichier est inchangé après la session.

**Acceptance Scenarios**:

1. **Given** un PST Unicode de 5 Go, **When** il est déposé, **Then** l'arborescence s'affiche avec une progression et l'interface reste utilisable pendant l'indexation.
2. **Given** un message d'un dossier PST, **When** il est ouvert, **Then** il est présenté avec les mêmes onglets qu'un MSG et sa provenance indique `boite.pst › Boîte de réception › …`.
3. **Given** un PST ANSI (ancien format) ou un OST, **When** il est déposé, **Then** il est lu ou, à défaut, un message explicite indique le format non pris en charge.

---

### User Story 2 - Rechercher et trier à l'échelle d'une boîte (Priority: P2)

L'analyste recherche dans toute la boîte (expéditeur, destinataire, objet, corps, noms de PJ,
période) et obtient une liste exploitable en quelques secondes, avec filtres et chronologie.

**Why this priority**: Une boîte contient des dizaines de milliers de messages ; la recherche
est indispensable pour trouver les éléments pertinents.

**Independent Test**: Sur un PST de 50 000 messages, une recherche retourne exactement les messages attendus d'une liste annotée.

**Acceptance Scenarios**:

1. **Given** un PST indexé, **When** l'analyste filtre « PJ exécutables, 2026-09 », **Then** la liste est produite en moins de 5 secondes.

---

### User Story 3 - Exporter des éléments d'un PST (Priority: P2)

L'analyste exporte une sélection de messages en `.eml` ou `.msg` individuels (avec manifeste
d'empreintes et provenance), ainsi que leurs PJ, pour les transmettre ou les analyser à part.

**Why this priority**: Permet de produire des pièces ciblées pour le rapport sans remettre la boîte entière.

**Independent Test**: Exporter 10 messages, les recharger dans l'outil : contenu identique (champs clés, empreintes des PJ).

**Acceptance Scenarios**:

1. **Given** une sélection de 10 messages, **When** ils sont exportés, **Then** 10 fichiers sont créés avec un manifeste listant provenance et empreintes, et le PST source reste inchangé.

---

### User Story 4 - Éléments supprimés et récupération (Priority: P3)

L'analyste voit les éléments du dossier « Éléments récupérables » / dossiers supprimés encore
référencés, et les éléments orphelins détectables, clairement marqués comme récupérés.

**Why this priority**: Forte valeur probante, mais techniquement plus incertaine.

**Independent Test**: Sur un PST de corpus où des messages ont été supprimés de façon connue, les éléments récupérables sont listés et marqués.

**Acceptance Scenarios**:

1. **Given** un PST contenant des éléments supprimés non purgés, **When** l'analyste ouvre la vue « Récupérés », **Then** ils apparaissent avec la mention de leur origine et ne sont jamais mélangés aux éléments actifs.

---

### Edge Cases

- PST protégé par mot de passe (le mot de passe PST n'est pas un chiffrement fort) : signalé, contenu lu si possible, sinon message explicite.
- PST chiffré par permutation/cyclique : déchiffrement transparent, mention dans les métadonnées.
- PST corrompu ou tronqué : lecture partielle, dossiers illisibles marqués, aucune « réparation » écrite sur la preuve.
- PST > 50 Go : pas de chargement intégral en mémoire.
- Éléments non courriel (rendez-vous, contacts, tâches) : listés avec leur classe de message, affichage minimal des propriétés.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: Le système DOIT ouvrir les fichiers PST Unicode en lecture seule stricte, sans jamais déclencher de réparation ni d'écriture.
- **FR-002**: Le système DOIT hacher le PST (SHA-256, MD5) à l'ingestion avec progression, et le revérifier avant tout export.
- **FR-003**: Le système DOIT afficher l'arborescence complète des dossiers avec compteurs.
- **FR-004**: Le système DOIT présenter chaque élément via le modèle unifié existant, avec toutes les vues de la feature 001.
- **FR-005**: Le système DOIT indiquer pour chaque message sa provenance (chemin de dossier, identifiant interne de nœud).
- **FR-006**: Le système DOIT permettre la recherche et le filtrage sur toute la boîte.
- **FR-007**: Le système DOIT exporter une sélection de messages en EML ou MSG avec manifeste d'empreintes.
- **FR-008**: Le système DOIT lister séparément les éléments supprimés récupérables, marqués comme tels.
- **FR-009**: Le système DOIT signaler les formats non pris en charge (ANSI, OST selon support) plutôt que d'échouer silencieusement.
- **FR-010**: Toutes les capacités DOIVENT être disponibles en CLI (`mf pst …`) avec sortie JSON.

### Key Entities

- **Store** : la boîte PST (preuve), format, chiffrement, statistiques.
- **Folder** : dossier, chemin, type, compteurs, parent.
- **Item** : élément (courriel ou autre classe), rattaché à un dossier, converti en Message du modèle unifié.
- **Index de boîte** : index de recherche persistant hors de la preuve.

## Success Criteria *(mandatory)*

- **SC-001**: Empreinte du PST identique avant/après ouverture, indexation et export (100 % du corpus).
- **SC-002**: Arborescence affichée en moins de 10 secondes pour un PST de 5 Go ; indexation complète d'un PST de 50 000 messages en moins de 10 minutes.
- **SC-003**: Nombre de messages par dossier identique à un outil de référence indépendant sur 100 % du corpus.
- **SC-004**: Les messages exportés puis rechargés sont identiques sur les champs clés et empreintes de PJ.

## Assumptions

- Socle technique pressenti : crate `outlook-pst` (Microsoft, MIT, lecture seule, MS-PST) — voir `specs/001-email-forensic-viewer/research.md` R12 ; à confirmer au `/speckit-plan` de cette feature, notamment pour ANSI/OST et la récupération d'éléments supprimés.
- Un index de recherche persistant (stocké dans le dossier d'enquête, jamais à côté de la preuve) devient justifié à cette échelle (Constitution VII → Complexity Tracking du plan 002).
- La modification de PST (écriture) reste hors périmètre : contraire à la Constitution I.
