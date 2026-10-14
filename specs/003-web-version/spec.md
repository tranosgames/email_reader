# Feature Specification: Version web locale de la visionneuse

**Feature Branch**: `003-web-version`

**Created**: 2026-10-14

**Status**: Draft

**Input**: User description: "Est-ce qu'il y a une version web au projet ? → oui, l'ajouter" — une version utilisable dans un navigateur, à la manière de la page GitHub Pages de msg-reader, sans installation.

## Clarifications

### Session 2026-10-14

- Q: Les fichiers peuvent-ils être envoyés à un serveur pour analyse ? → A: Non. L'analyse se fait entièrement dans le navigateur ; la page est statique (Constitution VI, v1.1.0).
- Q: Quelles capacités par rapport à l'application de bureau ? → A: Les mêmes vues de lecture que la feature 001 (en-têtes dépliables, fichiers contenus, URL, messages joints) ; les fonctions qui dépendent du système (association de fichiers, export vers un dossier, PST) sont hors périmètre.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Analyser des emails dans le navigateur sans installation (Priority: P1) 🎯 MVP

L'analyste ouvre la page web (hébergée, ou fichier local ouvert dans le navigateur), glisse des
fichiers `.eml` / `.msg` ou un dossier, et obtient exactement la même vue que dans l'application
de bureau : liste des messages, scellé de la preuve (SHA-256, MD5), bouton « Afficher les
en-têtes », fichiers contenus, URL, ouverture des messages joints.

**Why this priority**: C'est toute la valeur de la feature : un accès immédiat, sur un poste où
l'on ne peut rien installer.

**Independent Test**: Ouvrir la page, déposer le corpus de référence ; les empreintes, en-têtes,
fichiers et URL affichés sont identiques à ceux de `mf show --json` pour chaque fichier.

**Acceptance Scenarios**:

1. **Given** la page chargée, **When** l'analyste dépose 2 `.eml` et 1 `.msg`, **Then** les 3 messages apparaissent avec les mêmes empreintes que l'application de bureau.
2. **Given** un message affiché, **When** il appuie sur `H`, **Then** tous les en-têtes bruts se déplient comme dans l'application de bureau.
3. **Given** un dossier déposé contenant des sous-dossiers, **When** il est traité, **Then** tous les `.eml`/`.msg` qu'il contient sont chargés, les autres fichiers ignorés.
4. **Given** un fichier illisible, **When** il est déposé, **Then** il est listé en erreur et les autres sont traités.

---

### User Story 2 - Garantie de confidentialité vérifiable (Priority: P1)

L'analyste doit pouvoir affirmer (et vérifier) qu'aucune donnée de preuve n'a quitté son poste :
la page indique clairement que l'analyse est locale, et aucune requête réseau n'est émise après
le chargement, y compris à l'affichage d'un HTML contenant des ressources distantes.

**Why this priority**: Une page web inspire moins confiance qu'une application locale ; sans cette garantie, la version web n'est pas utilisable sur un dossier réel.

**Independent Test**: Charger la page, couper le réseau, déposer le corpus : tout fonctionne ; l'onglet Réseau des outils de développement ne montre aucune requête après le chargement.

**Acceptance Scenarios**:

1. **Given** la page chargée puis le réseau coupé, **When** l'analyste dépose des fichiers, **Then** l'analyse fonctionne normalement.
2. **Given** un email HTML avec pixel de suivi, **When** il est affiché, **Then** aucune requête n'est émise et le pixel est compté comme bloqué.

---

### User Story 3 - Déploiement simple (Priority: P2)

Le mainteneur publie la version web en une commande ou par CI (GitHub Pages comme msg-reader), et
peut aussi la livrer comme un dossier statique à ouvrir hors-ligne.

**Why this priority**: Utile pour la diffusion, mais la page fonctionne déjà localement.

**Independent Test**: Le dossier produit par la commande de build fonctionne servi par n'importe quel serveur statique.

**Acceptance Scenarios**:

1. **Given** une poussée sur la branche principale, **When** la CI s'exécute, **Then** la page est publiée sur GitHub Pages.

---

### Edge Cases

- Fichier très volumineux (> 500 Mo) : refus explicite avec conseil d'utiliser l'application de bureau, au lieu de saturer la mémoire de l'onglet.
- Navigateur sans WebAssembly : message clair au chargement.
- Plusieurs dépôts successifs, doublons : comportement identique à l'application de bureau (doublon ignoré).
- Analyse longue : l'interface reste réactive (analyse hors du fil principal ou progression affichée).

## Requirements *(mandatory)*

### Functional Requirements

- **FR-W01**: La page DOIT analyser les fichiers `.eml` et `.msg` entièrement dans le navigateur, avec le même moteur que l'application de bureau.
- **FR-W02**: La page NE DOIT émettre aucune requête réseau après son chargement initial, et ne DOIT transmettre aucun octet de preuve.
- **FR-W03**: La page DOIT accepter fichiers et dossiers par glisser-déposer et par sélection (fichiers, dossier).
- **FR-W04**: La page DOIT offrir les vues de la feature 001 livrées : liste, scellé (SHA-256, MD5), bouton des en-têtes (FR-035), fichiers contenus (FR-036), URL (FR-037), messages joints, raccourcis clavier.
- **FR-W05**: Les résultats (empreintes, en-têtes, fichiers, URL) DOIVENT être identiques à ceux de l'application de bureau et de la CLI pour un même fichier.
- **FR-W06**: La page DOIT indiquer à l'analyste que l'analyse est locale et que les fichiers ne sont ni envoyés ni modifiés.
- **FR-W07**: La page DOIT refuser explicitement les fichiers au-delà d'une taille limite (défaut 500 Mo).
- **FR-W08**: La page DOIT pouvoir être publiée comme site statique (GitHub Pages) et fonctionner servie depuis n'importe quel serveur statique.

### Key Entities

Mêmes entités que la feature 001 (Preuve, Message, En-tête, Pièce jointe, URL) — aucun nouveau modèle.

## Success Criteria *(mandatory)*

- **SC-W01**: Pour 100 % du corpus, les sorties de la page sont identiques à `mf show --json`.
- **SC-W02**: 0 requête réseau observée après le chargement de la page, sur l'ensemble du corpus.
- **SC-W03**: Un message de ≤ 25 Mo s'affiche en moins de 3 secondes dans un navigateur récent.
- **SC-W04**: La page complète (code + moteur + polices) pèse moins de 5 Mo.

## Assumptions

- Navigateurs cibles : versions récentes de Chrome, Edge, Firefox, Safari (WebAssembly requis).
- Pas d'export de pièces jointes en v1 web (feature 001 ne l'a pas encore livré) ; il passera par des téléchargements.
- Le PST (feature 002) n'est pas visé en web : taille des fichiers incompatible avec la mémoire d'un onglet.
- Révision de la décision R9 de la feature 001 : le web pur avait été écarté pour le hors-ligne et le hachage ; le moteur WebAssembly lève ces deux objections (calcul local, aucune requête).
