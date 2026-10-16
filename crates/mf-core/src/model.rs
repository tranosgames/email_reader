//! Modèle unifié, indépendant du format source (data-model.md).

use serde::{Deserialize, Serialize};

/// Format détecté par signature (FR-002).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Format {
    Eml,
    Msg,
    Pst,
    Unknown,
}

/// Fichier de preuve ingéré (lecture seule, empreintes calculées avant parsing).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evidence {
    pub path: String,
    pub size: u64,
    pub sha256: String,
    pub md5: String,
    pub format: Format,
    pub declared_extension: Option<String>,
}

/// Une adresse, telle qu'elle figure dans la preuve.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Address {
    pub display_name: Option<String>,
    /// Adresse SMTP (pour un MSG : adresse SMTP résolue).
    pub address: Option<String>,
    /// Adresse Exchange X.500 d'origine (MSG), le cas échéant.
    pub x500_dn: Option<String>,
}

/// Un en-tête dans l'ordre d'origine (FR-011, FR-035).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Header {
    pub index: u32,
    pub name: String,
    /// Valeur brute telle qu'écrite (lignes repliées conservées).
    pub raw: String,
    /// Valeur dépliée (sans retours à la ligne de repli).
    pub value: String,
    /// Plage d'octets dans la source : la preuve (EML) ou la propriété 0x007D (MSG).
    pub offset: Option<(u64, u64)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Disposition {
    Inline,
    Attachment,
    Unknown,
}

/// Fichier contenu dans le message (FR-016, FR-036).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    pub index: u32,
    pub filename: String,
    pub declared_mime: Option<String>,
    pub size: u64,
    pub sha256: String,
    pub md5: String,
    pub disposition: Disposition,
    pub content_id: Option<String>,
    /// Message embarqué (message/rfc822 ou MSG imbriqué), analysé récursivement.
    pub embedded: Option<Box<Message>>,
    /// Faux pour un message Outlook embarqué en objet MAPI : il n'a pas de fichier d'origine (FR-038).
    pub extractable: bool,
    /// Contenu décodé ; jamais sérialisé (export à la demande uniquement).
    #[serde(skip)]
    pub data: Vec<u8>,
}

/// Où une URL a été trouvée.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum UrlLocation {
    Header { name: String },
    BodyText,
    HtmlAttribute { attribute: String },
    HtmlText,
    HtmlCss,
}

impl std::fmt::Display for UrlLocation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UrlLocation::Header { name } => write!(f, "en-tête {name}"),
            UrlLocation::BodyText => write!(f, "corps texte"),
            UrlLocation::HtmlAttribute { attribute } => write!(f, "HTML @{attribute}"),
            UrlLocation::HtmlText => write!(f, "HTML texte"),
            UrlLocation::HtmlCss => write!(f, "HTML css"),
        }
    }
}

/// URL unique d'un message (FR-037).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UrlEntry {
    pub url: String,
    pub defanged: String,
    pub host: Option<String>,
    pub locations: Vec<UrlLocation>,
    pub occurrences: u32,
}

/// Nature d'une ressource distante que le HTML demandait à charger (FR-039).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlockedKind {
    /// `<img src>`, `<img srcset>`, `<input type=image>` — dont les pixels de suivi.
    Image,
    /// `style="background: url(…)"`, attribut `background`.
    Background,
    /// `<link rel=stylesheet>`, `@import`, `url()` dans un bloc `<style>`.
    Stylesheet,
    Script,
    /// `<iframe>`, `<frame>`.
    Frame,
    /// `<video>`, `<audio>`, `<source>`, `<track>`, attribut `poster`.
    Media,
    /// `<object>`, `<embed>`.
    Object,
    Other,
}

/// Ressource distante bloquée : jamais chargée, mais localisée (FR-039).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockedResource {
    /// Numéro affiché (1, 2, …) dans l'ordre du HTML.
    pub number: u32,
    pub url: String,
    pub kind: BlockedKind,
    /// Élément HTML d'origine (`img`, `td`, `link`…).
    pub element: String,
    /// Attribut ou construction d'origine (`src`, `style`, `href`, `@import`…).
    pub attribute: String,
    /// Ligne (1 = première) dans le corps HTML brut, si retrouvée.
    pub line: Option<u32>,
    /// Repère interne présent dans le HTML rendu ; `None` si l'élément a été supprimé
    /// par la sanitisation (script, feuille de style, cadre…) et n'apparaît pas au rendu.
    pub marker: Option<u32>,
}

/// Courriel normalisé.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Message {
    pub subject: Option<String>,
    pub from: Vec<Address>,
    pub sender: Option<Address>,
    pub reply_to: Vec<Address>,
    pub to: Vec<Address>,
    pub cc: Vec<Address>,
    pub bcc: Vec<Address>,
    /// Valeur brute de la date (en-tête `Date:` ou propriété MAPI).
    pub date_raw: Option<String>,
    /// Date normalisée en UTC (RFC 3339).
    pub date_utc: Option<String>,
    pub message_id: Option<String>,
    pub headers: Vec<Header>,
    pub body_text: Option<String>,
    /// HTML brut, tel que dans la preuve (jamais rendu tel quel).
    pub body_html: Option<String>,
    /// HTML sanitisé, prêt pour une iframe sandbox (rempli par l'analyse).
    pub body_html_safe: Option<String>,
    /// Nombre de ressources distantes bloquées (= `blocked_resources.len()`).
    pub blocked_remote_resources: u32,
    /// Détail de chaque ressource distante bloquée, dans l'ordre du HTML (FR-039).
    pub blocked_resources: Vec<BlockedResource>,
    pub attachments: Vec<Attachment>,
    pub urls: Vec<UrlEntry>,
    /// Avertissements de parsing non bloquants.
    pub issues: Vec<String>,
}
