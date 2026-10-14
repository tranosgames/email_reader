<!--
Sync Impact Report
- Version: (template) → 1.0.0
- Principes ajoutés : I. Intégrité de la preuve, II. Traçabilité, III. Moteur d'abord (library-first),
  IV. Contenu hostile par défaut, V. Tests sur corpus, VI. Hors-ligne & reproductible, VII. Simplicité
- Sections ajoutées : Contraintes techniques, Workflow de développement, Gouvernance
- Templates : plan-template.md ✅ (Constitution Check lit ce fichier à l'exécution), spec/tasks ✅ aucun changement requis
- TODO différés : aucun
-->

# MailForensics Constitution

## Core Principles

### I. Intégrité de la preuve (NON NÉGOCIABLE)

- Un fichier de preuve (EML, MSG, PST) n'est **jamais modifié**. Il est ouvert en lecture seule ;
  aucun fichier temporaire, verrou ou métadonnée n'est écrit à côté de lui.
- Chaque preuve est hachée (SHA-256 **et** MD5) **à l'ingestion**, avant tout parsing. Le hachage est
  recalculé avant tout export ; une divergence bloque l'export et est signalée.
- Tout artefact extrait (pièce jointe, corps, message embarqué) est haché individuellement et
  rattaché à sa preuve source par un chemin d'origine (ex. `preuve.msg › PJ 2 › message.msg › PJ 1`).
- Les données sont affichées **telles qu'elles sont dans la preuve** ; toute valeur dérivée
  (décodage, normalisation, conversion de fuseau) est visuellement distinguée de la valeur brute,
  et la valeur brute reste toujours accessible.

### II. Traçabilité et rapport

- Chaque action de l'analyste qui produit un résultat (ingestion, export, marquage, note) est
  journalisée dans un journal d'audit horodaté en UTC, append-only, inclus dans les exports.
- Chaque constat affiché (ex. « Reply-To diffère de From ») cite sa source : nom du champ,
  propriété MAPI ou offset d'octet dans la preuve.
- Les exports sont déterministes : la même preuve et la même version de l'outil produisent un
  rapport identique (hors horodatage d'export).

### III. Moteur d'abord (library-first)

- Toute la logique d'analyse vit dans des crates Rust indépendantes de l'interface
  (`mf-core`, `mf-eml`, `mf-msg`, puis `mf-pst`). L'interface graphique et la CLI sont des
  couches minces sans logique métier.
- Chaque crate a une API documentée et est testable seule. Une CLI (`mf`) expose chaque
  capacité du moteur avec une sortie lisible **et** JSON, pour l'automatisation et la contre-vérification.
- Le modèle de données unifié (`Evidence`, `Message`, `Attachment`, `Finding`) est indépendant du
  format source : ajouter PST ne doit pas modifier l'UI.

### IV. Contenu hostile par défaut

- Tout courriel est présumé malveillant. Le HTML est rendu **sanitisé** dans une vue isolée, sans
  script, sans requête réseau (images distantes, polices, CSS externes bloqués), sans navigation.
- Aucune pièce jointe n'est jamais exécutée ni ouverte par l'outil ; l'export se fait vers un
  dossier choisi par l'analyste, avec option de neutralisation du nom (`.exe` → `.exe_`).
- Les parseurs doivent résister aux fichiers malformés : aucun `panic` sur une entrée, limites
  explicites (taille, profondeur d'imbrication, nombre de parties), `unsafe` interdit hors
  dépendances auditées. Chaque parseur est fuzzé.

### V. Tests sur corpus (test-first)

- Un corpus de référence versionné (`tests/corpus/`) contient des EML/MSG réels ou synthétiques,
  y compris malformés, avec résultats attendus (snapshots). Toute nouvelle capacité commence par
  un cas de corpus qui échoue.
- Les extractions MSG sont contre-vérifiées par un second parseur (oracle) dans les tests.
- Pas de fusion avec des tests rouges, `clippy -D warnings` et `cargo fmt --check` obligatoires.

### VI. Hors-ligne et reproductible

- L'outil fonctionne entièrement hors-ligne. Aucune télémétrie, aucune mise à jour automatique
  silencieuse, aucune résolution DNS/WHOIS implicite. Toute fonction réseau future sera opt-in,
  explicite et journalisée.
- Les builds sont reproductibles : `Cargo.lock` versionné, versions de dépendances épinglées,
  `cargo deny` / `cargo audit` en CI.

### VII. Simplicité

- On implémente ce dont l'enquête a besoin (YAGNI). Pas de base de données tant qu'un modèle en
  mémoire + export JSON suffit. Chaque complexité supplémentaire est justifiée dans le plan
  (section Complexity Tracking).

## Contraintes techniques

- Langage : Rust stable (édition 2024). Go uniquement si une capacité indispensable n'a pas
  d'implémentation Rust viable, décision documentée dans `research.md`.
- Interface : application de bureau Tauri 2 (Windows, macOS, Linux), dans l'esprit ergonomique
  de msg-reader (glisser-déposer, liste des messages, panneau de lecture) enrichie de vues forensic.
- Formats : EML (RFC 5322/2045+) et MSG (MS-OXMSG/MS-CFB) en v1 ; PST/OST (MS-PST) en lecture seule
  dans une feature ultérieure.
- Licences des dépendances : MIT, Apache-2.0, BSD ou équivalent permissif uniquement.

## Workflow de développement

- Méthode Spec-Driven Development (spec-kit) : `constitution → specify → clarify → plan → tasks →
  analyze → implement → converge`. Aucune implémentation sans `spec.md` et `plan.md` validés.
- Chaque PR référence ses IDs de tâches (`T0xx`) et d'exigences (`FR-0xx`).
- Revue obligatoire de toute modification touchant le hachage, le journal d'audit ou la
  sanitisation HTML.

## Governance

- Cette constitution prime sur toute autre pratique du projet. Le « Constitution Check » du plan
  vérifie chaque principe ; toute violation est justifiée dans Complexity Tracking ou corrigée.
- Amendements : PR dédiée modifiant ce fichier, avec rapport d'impact et incrément de version
  (MAJOR : principe retiré/redéfini ; MINOR : principe ou section ajouté ; PATCH : clarification).

**Version**: 1.0.0 | **Ratified**: 2026-10-14 | **Last Amended**: 2026-10-14
