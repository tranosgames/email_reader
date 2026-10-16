# Feature Specification: Visionneuse forensic de courriels EML/MSG

**Feature Branch**: `001-email-forensic-viewer`

**Created**: 2026-10-14

**Status**: Draft

**Input**: User description: "Je souhaite créer un outil pour m'aider dans un cas de forensics sur de l'analyse d'email. Je souhaite analyser des fichiers eml, msg. Et potentiellement dans le futur manipuler des fichiers pst. Se baser sur le design du projet msg-reader (glisser-déposer, liste de messages, panneau de lecture, épinglage, tri par date, raccourcis clavier)."

## User Scenarios & Testing *(mandatory)*

> Acteur principal : **l'analyste forensic / SOC / réponse à incident** qui reçoit un ou plusieurs
> courriels suspects (phishing, BEC, fuite de données) sous forme de fichiers `.eml` ou `.msg`
> et doit établir des faits défendables : d'où vient le message, est-il authentique, que contient-il,
> quels indicateurs en extraire.

## Clarifications

### Session 2026-10-14

- Q: Comment l'analyste accède-t-il aux en-têtes depuis la lecture d'un message ? → A: Un bouton « Afficher les en-têtes » dans le panneau de lecture déplie/replie l'intégralité des en-têtes bruts du message, sans quitter la vue de lecture (FR-035).
- Q: La liste des fichiers contenus dans le mail doit-elle être visible sans changer d'onglet ? → A: Oui, la liste des pièces jointes (y compris inline et messages embarqués) est toujours affichée dans le panneau de lecture de chaque message (FR-036).
- Q: Les URL doivent-elles être listées pour chaque email dès le MVP ? → A: Oui, une liste de toutes les URL présentes dans l'email (en-têtes, corps texte, corps HTML) est affichée dans le panneau de lecture, par message, dès le MVP (FR-037).

- Q: Pourquoi le survol d'un fichier contenu affiche-t-il le MD5 ? → A: Le MD5 était placé en infobulle du SHA-256, ce qui le rendait invisible sans survol et prêtait à confusion. Les deux empreintes sont désormais affichées en clair, sans infobulle (FR-036).
- Q: Peut-on récupérer un fichier contenu ? → A: Oui, chaque fichier a un bouton « Télécharger » ; une boîte de confirmation rappelle nom, type, taille, SHA-256 et le risque, et rien n'est écrit sans validation (FR-038).

- Q: Que signifie « 1 ressource(s) distante(s) bloquée(s) » et où se trouve-t-elle ? → A: Le corps HTML demandait à charger un élément depuis Internet (souvent un pixel de suivi) ; il est bloqué car le charger signalerait à l'expéditeur l'ouverture du message (IP, heure, logiciel). Le bandeau devient une liste détaillée : pour chaque ressource, son URL, son type, l'élément et l'attribut HTML concernés, sa ligne dans le code HTML, un repère visible à son emplacement dans le rendu et l'accès direct à la ligne dans une vue « Source HTML » (FR-039).

### User Story 1 - Ingérer et lire des preuves sans les altérer (Priority: P1) 🎯 MVP

L'analyste glisse un ou plusieurs fichiers `.eml` / `.msg` (ou un dossier) dans la fenêtre, comme
dans msg-reader. Chaque fichier est immédiatement haché, puis apparaît dans une liste latérale
(expéditeur, objet, date, indicateurs « PJ » / « alerte »). En sélectionnant un message, il voit un
panneau de lecture avec l'en-tête lisible (De, À, Cc, Cci, Objet, Date), le corps rendu de façon
sûre (HTML neutralisé, images distantes bloquées), un bouton **« Afficher les en-têtes »** qui
déplie tous les en-têtes bruts, la **liste des fichiers contenus** dans le mail et la **liste de
toutes les URL** présentes dans le mail.

**Why this priority**: Sans ingestion fiable et lecture sûre, aucune autre analyse n'est possible ;
c'est aussi l'usage le plus fréquent (« ouvrir le .msg que l'utilisateur a signalé »).

**Independent Test**: Déposer le corpus de référence ; vérifier que chaque message s'affiche, que
les empreintes affichées correspondent à un calcul indépendant (`sha256sum`) et que les fichiers
sont inchangés après fermeture (empreinte et date de modification identiques).

**Acceptance Scenarios**:

1. **Given** l'application ouverte et vide, **When** l'analyste dépose 3 fichiers (2 `.eml`, 1 `.msg`), **Then** les 3 messages apparaissent dans la liste triée par date décroissante, chacun avec son SHA-256 et son MD5 consultables.
2. **Given** un message HTML contenant une image distante et un script, **When** il est affiché, **Then** aucune requête réseau n'est émise, le script n'est pas exécuté et un bandeau indique « 1 ressource distante bloquée ».
3. **Given** un fichier déjà chargé, **When** l'analyste le dépose à nouveau (même empreinte), **Then** il n'est pas dupliqué et un avertissement « doublon de la preuve X » s'affiche.
4. **Given** un fichier corrompu ou qui n'est pas un courriel, **When** il est déposé, **Then** il apparaît dans la liste avec un statut d'erreur explicite et les autres fichiers sont traités normalement.
5. **Given** un message affiché, **When** l'analyste l'épingle (raccourci clavier ou bouton), **Then** il reste en tête de liste pendant la session et l'épinglage est journalisé.
6. **Given** un message affiché avec les en-têtes repliés, **When** l'analyste clique sur « Afficher les en-têtes » (ou touche `H`), **Then** tous les en-têtes bruts apparaissent dans leur ordre d'origine avec leur nombre, et le bouton devient « Masquer les en-têtes » ; l'état est conservé quand on passe au message suivant.
10. **Given** un email HTML avec un pixel de suivi, une image de fond et une feuille de style distantes, **When** il est affiché, **Then** le bandeau « 3 ressources distantes bloquées » se déplie en une liste (URL, type, élément, ligne) ; « Voir dans le rendu » amène au repère numéroté et « Voir dans la source » à la ligne du code HTML, sans aucune requête réseau.
9. **Given** une pièce jointe `facture.pdf.exe`, **When** l'analyste clique « Télécharger », **Then** une confirmation affiche nom, taille, SHA-256 et un avertissement ; « Annuler » n'écrit rien ; « OK » enregistre un fichier dont le SHA-256 est identique à celui affiché.
7. **Given** un message contenant 2 pièces jointes et une image inline, **When** il est affiché, **Then** la liste « Fichiers contenus (3) » montre pour chacun nom, type, taille, empreinte SHA-256 et la mention inline/attaché ; un message vide de PJ affiche « Aucun fichier ».
8. **Given** un message dont le corps HTML contient 3 liens dont un en double et le corps texte une URL supplémentaire, **When** il est affiché, **Then** la liste « URL (3) » montre chaque URL unique une seule fois avec son ou ses emplacements (HTML, texte, en-tête) ; aucune URL n'est cliquable ni ouverte, elles peuvent être copiées.

---

### User Story 2 - Analyser les en-têtes et l'authenticité (Priority: P2)

Pour un message sélectionné, l'analyste ouvre l'onglet « En-têtes » : il voit tous les en-têtes
bruts dans leur ordre d'origine, la **chaîne de relais** reconstituée (`Received`, du plus ancien
au plus récent, avec hôtes, IP, horodatages normalisés UTC et délais entre sauts), les résultats
d'authentification déclarés (SPF, DKIM, DMARC, ARC via `Authentication-Results` / `Received-SPF`)
et une liste de **constats** automatiques (ex. : `Reply-To` ≠ `From`, domaine du `Return-Path`
différent, `Message-ID` d'un domaine tiers, saut horaire incohérent, date dans le futur).

**Why this priority**: C'est le cœur de l'analyse de phishing/usurpation ; c'est ce qu'un simple
lecteur comme msg-reader ne fournit pas.

**Independent Test**: Sur des cas de corpus à vérité connue (usurpation, relais légitimes,
message forgé), vérifier que la chaîne de relais et chaque constat attendu apparaissent, avec la
référence au champ source.

**Acceptance Scenarios**:

1. **Given** un EML avec 4 en-têtes `Received`, **When** l'onglet En-têtes est ouvert, **Then** 4 sauts sont listés dans l'ordre chronologique avec IP, hôte, horodatage UTC et délai, et chaque saut renvoie à l'offset de l'en-tête brut.
2. **Given** un message dont `Reply-To` pointe vers un autre domaine que `From`, **When** il est analysé, **Then** un constat de sévérité « moyenne » est affiché, citant les deux champs.
3. **Given** un MSG Outlook, **When** il est analysé, **Then** les en-têtes de transport d'origine (propriété MAPI des en-têtes Internet) sont affichés ; s'ils sont absents (message interne/brouillon), un constat « en-têtes de transport absents » est affiché.
4. **Given** un message signé DKIM, **When** il est analysé, **Then** l'outil indique le domaine signataire, le sélecteur, et si l'empreinte du corps (`bh=`) recalculée hors-ligne correspond ou non.

---

### User Story 3 - Examiner pièces jointes et messages imbriqués (Priority: P2)

L'analyste consulte pour chaque pièce jointe : nom déclaré, type déclaré, **type réel détecté**
par signature, taille, SHA-256/MD5, statut inline/attaché, et alertes (double extension, extension
exécutable, archive chiffrée, document à macros, nom avec caractères de contrôle/RTL). Les messages
joints (`.eml`/`.msg` embarqués, `message/rfc822`) sont ouverts **récursivement** comme des
messages à part entière, avec un fil d'Ariane de provenance. Il peut exporter une ou plusieurs
pièces jointes vers un dossier, avec neutralisation optionnelle du nom.

**Why this priority**: Les charges malveillantes passent par les PJ ; leurs empreintes sont les
premiers IOC transmis aux équipes CTI/EDR.

**Independent Test**: Sur un MSG contenant un MSG qui contient un PDF, vérifier que les 3 niveaux
sont navigables, que les empreintes correspondent à celles du fichier extrait, et que l'export
n'écrit que dans le dossier choisi.

**Acceptance Scenarios**:

1. **Given** une PJ nommée `facture.pdf.exe` déclarée `application/pdf`, **When** elle est listée, **Then** les alertes « double extension » et « type déclaré ≠ type réel (exécutable PE) » sont affichées.
2. **Given** un message embarqué sur 3 niveaux, **When** l'analyste navigue, **Then** chaque niveau s'affiche avec le chemin `preuve › PJ n › …` et ses propres en-têtes.
3. **Given** une sélection de 2 PJ, **When** l'analyste exporte avec neutralisation, **Then** les fichiers sont écrits sous des noms suffixés (`.exe_`), un manifeste d'empreintes est généré et l'action est journalisée.
4. **Given** une image inline référencée par `cid:`, **When** le corps est affiché, **Then** l'image s'affiche dans le corps et figure aussi dans la liste des PJ marquée « inline ».

---

### User Story 4 - Extraire les indicateurs de compromission (Priority: P3)

L'onglet « Indicateurs » liste les URL, domaines, adresses IP et adresses courriel trouvés dans les
en-têtes, le corps texte, le corps HTML et les noms de PJ, dédoublonnés, avec leur(s)
emplacement(s). Les liens dont le texte affiché diffère de la cible, les domaines en punycode ou à
homoglyphes, et les URL vers des IP brutes sont signalés. L'analyste peut exporter les IOC (CSV,
JSON, ou format STIX-like simple) et les copier « défangés » (`hxxp://exemple[.]com`).

**Why this priority**: Accélère la diffusion vers le SOC, mais repose sur US1–US3.

**Independent Test**: Sur un message de corpus annoté, comparer la liste des IOC extraits à la
liste attendue (rappel 100 %, aucun faux positif sur les cas annotés).

**Acceptance Scenarios**:

1. **Given** un lien `<a href="http://203.0.113.7/login">www.banque.fr</a>`, **When** les IOC sont extraits, **Then** l'URL, l'IP et le constat « texte du lien ≠ cible » apparaissent.
2. **Given** la sélection de plusieurs messages, **When** l'analyste exporte les IOC, **Then** un fichier unique dédoublonné est produit, chaque IOC listant les messages sources.

---

### User Story 5 - Rapport, notes et journal d'audit (Priority: P3)

L'analyste ajoute des notes et des étiquettes à un message, puis exporte un **rapport d'analyse**
(lisible HTML imprimable en PDF + JSON machine) comprenant : identification de la preuve
(empreintes, taille, horodatages), en-têtes clés, chaîne de relais, constats, PJ et IOC, notes, et
le journal d'audit de la session (qui/quand/quoi). Il peut sauvegarder et rouvrir un « dossier
d'enquête » qui référence les preuves par chemin et empreinte, sans les copier.

**Why this priority**: Indispensable pour la chaîne de traçabilité et la remise au client/juge,
mais n'apporte de valeur qu'une fois l'analyse faite.

**Independent Test**: Exporter le rapport deux fois sur la même preuve : contenu identique hors
horodatage d'export ; rouvrir le dossier : preuves revérifiées par empreinte, notes restaurées.

**Acceptance Scenarios**:

1. **Given** une preuve analysée et annotée, **When** le rapport est exporté, **Then** il contient les empreintes revérifiées au moment de l'export et le journal d'audit complet.
2. **Given** un dossier d'enquête rouvert alors qu'une preuve a été modifiée sur disque, **When** il se charge, **Then** la preuve est marquée « empreinte divergente » et toute exportation la concernant est bloquée.

---

### User Story 6 - Rechercher, filtrer, chronologie et vue brute (Priority: P3)

L'analyste qui a chargé des dizaines de messages recherche en texte libre (objet, corps,
adresses, noms de PJ), filtre (épinglés, avec PJ, avec alertes, par expéditeur/domaine, par
période) et voit une **chronologie** des messages. Pour un message donné, il peut afficher la
source brute (EML) ou l'explorateur de propriétés (MSG : identifiant, type, valeur décodée et
brute de chaque propriété) et une vue hexadécimale de n'importe quel flux.

**Why this priority**: Gagne du temps sur les gros lots et permet la contre-vérification experte,
mais n'est pas requis pour traiter un message isolé.

**Independent Test**: Charger 200 messages de corpus ; une recherche d'un mot présent dans 3
corps retourne exactement ces 3 messages ; l'explorateur MSG liste les mêmes propriétés qu'un
outil tiers de référence.

**Acceptance Scenarios**:

1. **Given** 200 messages chargés, **When** l'analyste filtre « avec alertes » + domaine expéditeur, **Then** la liste se met à jour en moins d'une seconde.
2. **Given** un MSG, **When** l'explorateur de propriétés est ouvert, **Then** chaque propriété affiche son identifiant hexadécimal, son nom canonique s'il est connu, son type et sa valeur.

---

### Edge Cases

- Fichier `.msg` en encodage ANSI (non Unicode) avec page de code déclarée : décodage selon la page de code, valeur brute disponible.
- Corps présent uniquement en RTF compressé (avec ou sans HTML encapsulé) : décompression et affichage ; si HTML encapsulé, rendu HTML sûr.
- Pièce jointe `winmail.dat` (TNEF) : détectée et signalée ; contenu listé si décodable, sinon exporté tel quel.
- Messages S/MIME (`.p7m`, `.p7s`) ou PGP : signature détectée, certificats extraits et affichés ; contenu chiffré signalé « non déchiffrable sans clé ».
- En-têtes mal formés, encodages RFC 2047 invalides, jeux de caractères inconnus : valeur brute conservée, avertissement affiché, pas d'échec global.
- Dates absentes, invalides, sans fuseau ou dans le futur : affichées brutes + constat.
- Imbrication profonde (> 10 niveaux) ou boucle de messages embarqués : arrêt à une limite configurable, constat « imbrication tronquée ».
- Très gros fichiers (pièce jointe de 100 Mo, MSG de 500 Mo) : l'interface reste réactive, progression affichée.
- Nom de pièce jointe avec caractère de contrôle de direction (U+202E) ou chemin (`../`) : affiché échappé, neutralisé à l'export.
- Fichier `.eml` qui est en réalité un `.msg` (ou l'inverse) : détection par signature, pas par extension.
- Fichier en lecture seule, sur partage réseau ou support WORM : fonctionnement normal (aucune écriture requise).

## Requirements *(mandatory)*

### Functional Requirements

**Ingestion & intégrité**

- **FR-001**: Le système DOIT accepter des fichiers `.eml` et `.msg` par glisser-déposer, sélection de fichiers et sélection de dossier (récursive), ainsi qu'à l'ouverture via l'association de fichiers du système.
- **FR-002**: Le système DOIT détecter le format par signature de contenu, indépendamment de l'extension.
- **FR-003**: Le système DOIT calculer SHA-256 et MD5 de chaque preuve avant tout parsing et les afficher.
- **FR-004**: Le système NE DOIT JAMAIS écrire, renommer, verrouiller ou modifier une preuve ni son répertoire.
- **FR-005**: Le système DOIT détecter les doublons par empreinte et ne charger qu'une instance.
- **FR-006**: Le système DOIT isoler les erreurs : un fichier illisible n'interrompt pas le traitement des autres et affiche un message d'erreur exploitable.

**Lecture**

- **FR-007**: Le système DOIT afficher, pour chaque message : expéditeur (nom + adresse SMTP), destinataires À/Cc/Cci, objet, date (originale et UTC), et le corps en texte, HTML ou RTF selon disponibilité, avec sélection de la version affichée.
- **FR-008**: Le système DOIT rendre le HTML sans exécution de script, sans requête réseau, sans formulaire actif et sans navigation, et indiquer le nombre de ressources distantes bloquées.
- **FR-009**: Le système DOIT afficher les images inline référencées par `cid:` à partir des pièces jointes du message.
- **FR-010**: Le système DOIT reproduire l'ergonomie de msg-reader : liste latérale triable (date, expéditeur, objet), panneau de lecture, épinglage, raccourcis clavier (navigation ↑/↓, épingler, changer d'onglet), thèmes clair/sombre.

- **FR-035**: Le panneau de lecture DOIT comporter un bouton (et un raccourci clavier) « Afficher / Masquer les en-têtes » qui déplie tous les en-têtes bruts du message dans leur ordre d'origine ; repliés par défaut, état mémorisé pendant la session.
- **FR-036**: Le panneau de lecture DOIT afficher pour chaque message la liste de tous les fichiers contenus (pièces jointes, images inline, messages embarqués) avec nom, type, taille, SHA-256 et MD5 visibles sans survol, ou « Aucun fichier ».
- **FR-039**: Pour chaque ressource distante bloquée (images, images de fond, feuilles de style, scripts, cadres, médias, objets), le système DOIT indiquer son URL, son type, l'élément et l'attribut HTML d'origine, sa ligne dans le corps HTML et si elle apparaît dans le rendu ; un repère numéroté DOIT marquer son emplacement dans le rendu, et une vue « Source HTML » numérotée DOIT permettre d'atteindre la ligne. Les URL concernées sont signalées « chargement bloqué » dans la liste des URL. Aucune de ces ressources n'est jamais chargée.
- **FR-038**: Chaque fichier contenu DOIT offrir un bouton « Télécharger » qui ouvre une boîte de confirmation (nom, type, taille, SHA-256, avertissement sur le contenu potentiellement malveillant) ; sans confirmation rien n'est écrit. Le fichier enregistré DOIT être octet pour octet celui contenu dans le message (empreinte revérifiée), sous un nom assaini (sans chemin ni caractères de contrôle). Un message Outlook embarqué sans fichier d'origine (objet MAPI) n'est pas téléchargeable et le bouton l'indique.
- **FR-037**: Le panneau de lecture DOIT afficher pour chaque message la liste dédoublonnée de toutes les URL présentes (en-têtes, corps texte, attributs du corps HTML) avec leur emplacement ; les URL ne sont jamais ouvertes par l'outil et sont copiables (brutes ou défangées).

**En-têtes & authenticité**

- **FR-011**: Le système DOIT afficher tous les en-têtes bruts dans l'ordre d'origine ; pour un MSG, ceux de la propriété des en-têtes de transport Internet.
- **FR-012**: Le système DOIT reconstituer la chaîne `Received` (de, par, avec, IP, horodatage, délai entre sauts) en ordre chronologique.
- **FR-013**: Le système DOIT extraire et présenter les résultats SPF, DKIM, DMARC et ARC déclarés dans les en-têtes, en indiquant quel serveur les a émis.
- **FR-014**: Le système DOIT, pour chaque signature DKIM, afficher domaine, sélecteur, algorithme, en-têtes signés et vérifier hors-ligne la correspondance de l'empreinte du corps.
- **FR-015**: Le système DOIT générer des constats automatiques avec sévérité (info / faible / moyenne / élevée) et référence à la source, au minimum : `From`/`Reply-To`/`Return-Path`/`Sender` de domaines différents ; nom affiché contenant une adresse différente ; `Message-ID` d'un domaine tiers ; incohérences chronologiques de la chaîne ; date future ou absente ; en-têtes de transport absents (MSG) ; échec SPF/DKIM/DMARC déclaré.

**Pièces jointes & imbrication**

- **FR-016**: Le système DOIT lister chaque pièce jointe avec nom, type déclaré, type réel détecté, taille, SHA-256, MD5, inline/attachée, Content-ID.
- **FR-017**: Le système DOIT signaler : double extension, extension à risque, type déclaré ≠ type réel, caractères de contrôle/directionnels dans le nom, archive protégée par mot de passe, document Office avec macros.
- **FR-018**: Le système DOIT ouvrir récursivement les messages embarqués (EML dans EML, MSG dans MSG, mixtes) jusqu'à une profondeur configurable (défaut 10) et afficher le chemin de provenance.
- **FR-019**: Le système DOIT exporter une sélection de pièces jointes vers un dossier choisi, avec neutralisation de nom optionnelle, assainissement des chemins et manifeste d'empreintes.
- **FR-020**: Le système NE DOIT JAMAIS ouvrir ni exécuter une pièce jointe.

**Indicateurs**

- **FR-021**: Le système DOIT extraire URL, domaines, IP (v4/v6) et adresses courriel depuis en-têtes, corps texte, corps HTML (attributs inclus) et noms de PJ, dédoublonnés avec emplacements.
- **FR-022**: Le système DOIT signaler les liens dont le texte affiché diffère de la cible, les domaines punycode/homoglyphes et les URL vers IP brutes.
- **FR-023**: Le système DOIT exporter les IOC (CSV, JSON) pour un ou plusieurs messages et permettre la copie au format défangé.

**Rapport & audit**

- **FR-024**: Le système DOIT tenir un journal d'audit append-only (horodatage UTC, action, objet, empreinte) pour la session.
- **FR-025**: Le système DOIT permettre notes et étiquettes par message.
- **FR-026**: Le système DOIT exporter un rapport HTML (imprimable) et JSON contenant identification de la preuve, empreintes revérifiées, en-têtes clés, chaîne de relais, constats, PJ, IOC, notes et journal d'audit.
- **FR-027**: Le système DOIT revérifier l'empreinte d'une preuve avant tout export et bloquer l'export en cas de divergence.
- **FR-028**: Le système DOIT sauvegarder/rouvrir un dossier d'enquête référençant les preuves par chemin et empreinte, sans les copier.

**Recherche & vues expertes**

- **FR-029**: Le système DOIT offrir une recherche plein texte et des filtres (épinglé, PJ, alertes, expéditeur, domaine, période) sur les messages chargés.
- **FR-030**: Le système DOIT afficher une chronologie des messages chargés.
- **FR-031**: Le système DOIT afficher la source brute (EML), l'explorateur de propriétés (MSG) et une vue hexadécimale de tout flux ou partie.
- **FR-032**: Toutes les capacités d'analyse (FR-002 à FR-031 hors UI) DOIVENT être disponibles en ligne de commande avec sortie lisible et JSON.

**Robustesse & confidentialité**

- **FR-033**: Le système DOIT fonctionner sans aucun accès réseau et n'émettre aucune connexion réseau sans action explicite de l'analyste.
- **FR-034**: Le système DOIT résister aux entrées malformées sans plantage ni blocage, avec limites de taille, profondeur et nombre de parties.

### Key Entities *(include if feature involves data)*

- **Preuve (Evidence)** : fichier source ingéré ; chemin, taille, horodatages système, SHA-256, MD5, format détecté, statut (ok / erreur / empreinte divergente).
- **Message** : courriel normalisé indépendant du format ; adresses, objet, dates, corps (texte/HTML/RTF), en-têtes, pièces jointes, provenance (preuve + chemin d'imbrication).
- **En-tête (Header)** : nom, valeur brute, valeur décodée, position (ordre, offset) dans la source.
- **Saut de relais (Hop)** : émetteur, récepteur, IP, protocole, horodatage, délai, en-tête source.
- **Résultat d'authentification** : mécanisme (SPF/DKIM/DMARC/ARC), verdict, émetteur du verdict, détails.
- **Pièce jointe (Attachment)** : nom, types déclaré/détecté, taille, empreintes, inline, Content-ID, éventuel message embarqué.
- **Constat (Finding)** : règle, sévérité, description, références aux champs/offsets sources.
- **Indicateur (IOC)** : type, valeur, valeur défangée, emplacements, messages sources.
- **Dossier d'enquête (Case)** : preuves référencées, notes, étiquettes, épinglages, journal d'audit.
- **Entrée d'audit** : horodatage UTC, action, cible, empreinte associée, version de l'outil.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 100 % des preuves du corpus de référence conservent une empreinte identique et un contenu inchangé après ingestion, analyse et export.
- **SC-002**: Un message de ≤ 25 Mo s'affiche en moins de 2 secondes ; un lot de 500 messages est ingéré et listé en moins de 60 secondes sur un poste portable standard.
- **SC-003**: 0 plantage sur l'ensemble du corpus (y compris fichiers malformés) et après 24 h de fuzzing cumulé par parseur.
- **SC-004**: Sur les cas annotés du corpus, 100 % des constats et IOC attendus sont produits, et 0 requête réseau est observée pendant l'affichage.
- **SC-005**: Deux exports successifs du même rapport sont identiques hors horodatage d'export.
- **SC-006**: Un analyste formé identifie les indicateurs d'usurpation d'un message de test en moins de 5 minutes, contre plus de 15 minutes avec un simple lecteur de courriel.
- **SC-007**: Les valeurs extraites des MSG du corpus concordent à 100 % avec un outil de référence indépendant sur les champs objet, expéditeur, destinataires, date et empreintes des PJ.

## Assumptions

- Utilisateurs : analystes techniques à l'aise avec les notions d'en-têtes SMTP et d'IOC ; l'interface est en français avec termes techniques usuels (anglais si standard : Received, DKIM…).
- Usage mono-poste, mono-analyste en v1 ; pas de gestion multi-utilisateur ni de serveur.
- La vérification cryptographique complète DKIM/S/MIME nécessitant des clés publiques en DNS ou des autorités de certification est hors périmètre v1 (respect du hors-ligne) ; seule la vérification hors-ligne (empreinte de corps, extraction des certificats) est incluse. Une vérification en ligne opt-in pourra faire l'objet d'une feature ultérieure.
- Pas d'enrichissement externe (WHOIS, réputation, VirusTotal) en v1.
- Le décodage complet du TNEF est souhaitable mais peut être livré après le MVP.
- Le support PST/OST fait l'objet de la feature `002-pst-support` ; le modèle de données de cette feature doit l'accueillir sans changement de l'interface.
- Le design visuel s'inspire de msg-reader (Rasalas/msg-reader, licence MIT) ; aucun code n'est obligatoirement repris.
