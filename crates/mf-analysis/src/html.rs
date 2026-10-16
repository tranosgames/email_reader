//! Rendu HTML sûr (FR-008, FR-009) et inventaire des ressources distantes bloquées (FR-039).
//!
//! Première couche de défense ; la seconde est l'iframe `sandbox` + CSP `default-src 'none'`
//! côté interface (research.md R4).
//!
//! Aucune ressource distante n'est jamais chargée. Chacune est inventoriée (URL, type, élément,
//! attribut, ligne du HTML brut) ; quand l'élément reste visible après sanitisation, sa valeur est
//! remplacée par un **repère** inerte que l'interface sait retrouver pour montrer l'emplacement :
//! - attributs d'image (`src`, `background`, `poster`) : GIF transparent 1×1 en `data:` suivi de
//!   `#mf-blocked-K` (le fragment est ignoré par le navigateur) ;
//! - attribut `style` : chaque `url(…)` distant devient `none` et la déclaration
//!   `--mf-blocked: K` est ajoutée.

use base64::Engine;
use mf_core::{BlockedKind, BlockedResource};
use regex::Regex;
use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex};

/// GIF transparent 1×1 servant de repère (aucune requête, aucun contenu).
const MARKER_GIF: &str =
    "data:image/gif;base64,R0lGODlhAQABAIAAAAAAAP///yH5BAEAAAAALAAAAAABAAEAAAIBRAA7";
/// Préfixe du fragment de repère, recherché par l'interface.
pub const MARKER_FRAGMENT: &str = "#mf-blocked-";
/// Propriété CSS de repère, recherchée par l'interface.
pub const MARKER_CSS_PROPERTY: &str = "--mf-blocked";

/// Résultat du nettoyage.
pub struct SafeHtml {
    pub html: String,
    /// Ressources distantes bloquées, dans l'ordre du HTML, numérotées à partir de 1.
    pub blocked: Vec<BlockedResource>,
}

static CSS_URL: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r#"(?i)url\(\s*(?:"([^"]*)"|'([^']*)'|([^)\s]*))\s*\)"#).ok());
static CSS_IMPORT: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r#"(?i)@import\s+(?:url\()?\s*["']?([^"')\s;]+)"#).ok());
/// Balises supprimées par la sanitisation mais qui auraient déclenché un chargement.
static REMOVED_TAG: LazyLock<Option<Regex>> = LazyLock::new(|| {
    Regex::new(r"(?is)<(script|link|iframe|frame|video|audio|source|track|embed|object|input|body)\b([^>]*)>").ok()
});
static STYLE_BLOCK: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"(?is)<style\b[^>]*>(.*?)</style\s*>").ok());
static ATTR: LazyLock<Option<Regex>> = LazyLock::new(|| {
    Regex::new(r#"(?is)\b([a-z-]+)\s*=\s*(?:"([^"]*)"|'([^']*)'|([^\s>]+))"#).ok()
});

/// Vrai si l'URL déclencherait une requête réseau (schéma réseau ou URL relative au protocole).
fn is_remote(url: &str) -> bool {
    let u = url.trim().to_ascii_lowercase();
    u.starts_with("http:")
        || u.starts_with("https:")
        || u.starts_with("//")
        || u.starts_with("ftp:")
}

fn decode_attr(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
}

/// Ressource trouvée, avant numérotation.
#[derive(Clone)]
struct Found {
    url: String,
    kind: BlockedKind,
    element: String,
    attribute: String,
    offset: Option<usize>,
    marker: Option<u32>,
}

/// Recherche de l'URL dans le HTML brut, en avançant (même ordre que le document).
struct Locator<'h> {
    html: &'h str,
    cursor: usize,
}

impl Locator<'_> {
    fn find(&mut self, url: &str) -> Option<usize> {
        let candidates = [url.to_string(), url.replace('&', "&amp;")];
        for c in &candidates {
            if c.is_empty() {
                continue;
            }
            if let Some(i) = self
                .html
                .get(self.cursor..)
                .and_then(|rest| rest.find(c.as_str()))
            {
                self.cursor += i + c.len();
                return Some(self.cursor - c.len());
            }
        }
        candidates.iter().find_map(|c| self.html.find(c.as_str()))
    }
}

fn line_of(html: &str, offset: usize) -> u32 {
    html.get(..offset)
        .map_or(0, |s| s.bytes().filter(|&b| b == b'\n').count()) as u32
        + 1
}

fn kind_of(element: &str, attribute: &str) -> BlockedKind {
    match (element, attribute) {
        (_, "style") | (_, "background") => BlockedKind::Background,
        (_, "poster") | ("video" | "audio" | "source" | "track", _) => BlockedKind::Media,
        ("img" | "input", _) => BlockedKind::Image,
        ("script", _) => BlockedKind::Script,
        ("link" | "style", _) => BlockedKind::Stylesheet,
        ("iframe" | "frame", _) => BlockedKind::Frame,
        ("object" | "embed", _) => BlockedKind::Object,
        _ => BlockedKind::Other,
    }
}

/// Ressources des éléments que la sanitisation supprime entièrement (ils n'ont donc pas de repère).
fn removed_elements(html: &str) -> Vec<Found> {
    let mut out = Vec::new();
    if let (Some(tag_re), Some(attr_re)) = (REMOVED_TAG.as_ref(), ATTR.as_ref()) {
        for cap in tag_re.captures_iter(html) {
            let element = cap.get(1).map_or("", |m| m.as_str()).to_ascii_lowercase();
            let attrs = cap.get(2).map_or("", |m| m.as_str());
            let tag_start = cap.get(0).map_or(0, |m| m.start());
            let attrs_start = cap.get(2).map_or(tag_start, |m| m.start());
            let lower_attrs = attrs.to_ascii_lowercase();
            for a in attr_re.captures_iter(attrs) {
                let name = a.get(1).map_or("", |m| m.as_str()).to_ascii_lowercase();
                let wanted = match element.as_str() {
                    "link" => name == "href",
                    "object" => name == "data",
                    "body" => name == "background",
                    "input" => name == "src" && lower_attrs.contains("image"),
                    _ => name == "src" || name == "poster",
                };
                let Some(v) = a.get(2).or(a.get(3)).or(a.get(4)) else {
                    continue;
                };
                let url = decode_attr(v.as_str());
                if wanted && is_remote(&url) {
                    out.push(Found {
                        kind: kind_of(&element, &name),
                        url,
                        element: element.clone(),
                        attribute: name,
                        offset: Some(attrs_start + v.start()),
                        marker: None,
                    });
                }
            }
        }
    }
    if let Some(block) = STYLE_BLOCK.as_ref() {
        for cap in block.captures_iter(html) {
            let Some(css) = cap.get(1) else { continue };
            for (re, attribute) in [
                (CSS_IMPORT.as_ref(), "@import"),
                (CSS_URL.as_ref(), "url()"),
            ] {
                let Some(re) = re else { continue };
                for u in re.captures_iter(css.as_str()) {
                    let Some(m) = u.get(1).or(u.get(2)).or(u.get(3)) else {
                        continue;
                    };
                    let offset = Some(css.start() + m.start());
                    // `@import url(…)` est vu par les deux expressions : une seule entrée.
                    if is_remote(m.as_str()) && !out.iter().any(|f: &Found| f.offset == offset) {
                        out.push(Found {
                            url: m.as_str().to_string(),
                            kind: BlockedKind::Stylesheet,
                            element: "style".into(),
                            attribute: attribute.into(),
                            offset,
                            marker: None,
                        });
                    }
                }
            }
        }
    }
    out
}

/// `cid_images` : Content-ID (sans chevrons) → (type MIME, octets) des images inline.
pub fn sanitize(html: &str, cid_images: &HashMap<String, (String, Vec<u8>)>) -> SafeHtml {
    let data_uris: HashMap<String, String> = cid_images
        .iter()
        .map(|(cid, (mime, bytes))| {
            let b64 = base64::engine::general_purpose::STANDARD.encode(bytes);
            (
                cid.to_ascii_lowercase(),
                format!("data:{mime};base64,{b64}"),
            )
        })
        .collect();
    let data_uris = Arc::new(data_uris);
    let kept: Arc<Mutex<Vec<Found>>> = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&kept);

    let mut b = ammonia::Builder::default();
    b.add_url_schemes(["data", "cid"])
        .add_generic_attributes([
            "style", "align", "valign", "width", "height", "bgcolor", "color",
        ])
        .add_tag_attributes("img", ["srcset"])
        .add_tag_attributes("table", ["background"])
        .add_tag_attributes("tr", ["background"])
        .add_tag_attributes("td", ["background"])
        .add_tag_attributes("th", ["background"])
        .link_rel(None)
        .strip_comments(true)
        .attribute_filter(move |element, attribute, value| {
            let lower = value.trim().to_ascii_lowercase();
            let mut found = sink.lock().unwrap_or_else(|e| e.into_inner());
            let next_marker = found.iter().filter_map(|f| f.marker).max().unwrap_or(0) + 1;
            let mut record = |url: &str, marker: Option<u32>| {
                found.push(Found {
                    url: url.trim().to_string(),
                    kind: kind_of(element, attribute),
                    element: element.to_string(),
                    attribute: attribute.to_string(),
                    offset: None,
                    marker,
                });
            };
            match attribute {
                // Aucun lien n'est navigable dans l'outil : la cible figure dans la liste des URL.
                "href" => None,
                // Seuls <img> et les cellules de tableau sont conservés par la sanitisation ;
                // les autres porteurs (script, iframe, video…) sont inventoriés par
                // `removed_elements`, sans repère puisqu'ils disparaissent du rendu.
                "src" | "background" | "poster"
                    if !matches!(element, "img" | "table" | "tr" | "td" | "th") =>
                {
                    None
                }
                "src" | "background" | "poster" => {
                    if let Some(cid) = lower.strip_prefix("cid:") {
                        data_uris.get(cid).cloned().map(Into::into)
                    } else if lower.starts_with("data:image/") {
                        Some(value.into())
                    } else if is_remote(&lower) {
                        record(value, Some(next_marker));
                        Some(format!("{MARKER_GIF}{MARKER_FRAGMENT}{next_marker}").into())
                    } else {
                        None
                    }
                }
                "srcset" => {
                    // Chaque candidat distant est inventorié ; l'attribut est retiré (le `src`
                    // de la même image porte, lui, le repère).
                    for cand in value.split(',') {
                        if let Some(u) = cand.split_whitespace().next().filter(|u| is_remote(u)) {
                            record(u, None);
                        }
                    }
                    None
                }
                "style" if lower.contains("expression(") || lower.contains("@import") => {
                    record(value, None);
                    None
                }
                "style" if lower.contains("url(") => {
                    let re = CSS_URL.as_ref()?;
                    let mut any_remote = false;
                    let rewritten = re.replace_all(value, |c: &regex::Captures<'_>| {
                        let u = c
                            .get(1)
                            .or(c.get(2))
                            .or(c.get(3))
                            .map_or("", |m| m.as_str());
                        let ul = u.to_ascii_lowercase();
                        if let Some(uri) =
                            ul.strip_prefix("cid:").and_then(|cid| data_uris.get(cid))
                        {
                            format!("url(\"{uri}\")")
                        } else if is_remote(u) {
                            any_remote = true;
                            record(u, Some(next_marker));
                            "none".to_string()
                        } else {
                            c.get(0).map_or(String::new(), |m| m.as_str().to_string())
                        }
                    });
                    if any_remote {
                        Some(format!("{rewritten}; {MARKER_CSS_PROPERTY}: {next_marker}").into())
                    } else {
                        Some(rewritten.into_owned().into())
                    }
                }
                _ => Some(value.into()),
            }
        });
    let safe = b.clean(html).to_string();

    let mut locator = Locator { html, cursor: 0 };
    let mut all: Vec<Found> = kept
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .iter()
        .cloned()
        .map(|mut f| {
            f.offset = locator.find(&f.url);
            f
        })
        .collect();
    let removed = removed_elements(html);
    // Une ressource déjà inventoriée par le filtre (même offset) n'est pas dupliquée.
    let extra: Vec<Found> = removed
        .into_iter()
        .filter(|r| !all.iter().any(|k| k.offset == r.offset && k.url == r.url))
        .collect();
    all.extend(extra);
    all.sort_by_key(|f| (f.offset.is_none(), f.offset.unwrap_or(usize::MAX)));

    let blocked = all
        .into_iter()
        .enumerate()
        .map(|(i, f)| BlockedResource {
            number: i as u32 + 1,
            url: f.url,
            kind: f.kind,
            element: f.element,
            attribute: f.attribute,
            line: f.offset.map(|o| line_of(html, o)),
            marker: f.marker,
        })
        .collect();
    SafeHtml {
        html: safe,
        blocked,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removes_scripts_and_links_and_marks_remote_resources() {
        let out = sanitize(
            "<p onclick=\"x()\">Hi <script>alert(1)</script><img src=\"http://t.example/p.gif\" width=1 height=1>\n\
             <a href=\"http://evil.example\">clic</a><div style=\"color:red;background:url(http://x/y)\">z</div></p>",
            &HashMap::new(),
        );
        assert!(!out.html.contains("<script"));
        assert!(!out.html.contains("onclick"));
        assert!(!out.html.contains("http://"), "{}", out.html);
        assert!(out.html.contains("clic"));
        assert_eq!(out.blocked.len(), 2);
        let pixel = &out.blocked[0];
        assert_eq!(
            (pixel.number, pixel.url.as_str(), pixel.kind),
            (1, "http://t.example/p.gif", BlockedKind::Image)
        );
        assert_eq!(
            (pixel.element.as_str(), pixel.attribute.as_str(), pixel.line),
            ("img", "src", Some(1))
        );
        assert!(
            out.html
                .contains(&format!("{MARKER_FRAGMENT}{}", pixel.marker.unwrap_or(0)))
        );
        let bg = &out.blocked[1];
        assert_eq!(
            (bg.kind, bg.element.as_str(), bg.line),
            (BlockedKind::Background, "div", Some(2))
        );
        assert!(out.html.contains("--mf-blocked: 2"), "{}", out.html);
        assert!(out.html.contains("color:red"));
    }

    #[test]
    fn inventories_removed_elements() {
        let out = sanitize(
            "<link rel=stylesheet href=\"https://cdn.example/a.css\">\n<style>@import url('https://cdn.example/b.css');\
             p{background:url(https://cdn.example/c.png)}</style>\n<script src=\"https://cdn.example/x.js\"></script>\n\
             <iframe src=\"https://evil.example/f\"></iframe><p>texte</p>",
            &HashMap::new(),
        );
        let got: Vec<(&str, BlockedKind, Option<u32>, Option<u32>)> = out
            .blocked
            .iter()
            .map(|b| (b.url.as_str(), b.kind, b.line, b.marker))
            .collect();
        assert_eq!(
            got,
            [
                (
                    "https://cdn.example/a.css",
                    BlockedKind::Stylesheet,
                    Some(1),
                    None
                ),
                (
                    "https://cdn.example/b.css",
                    BlockedKind::Stylesheet,
                    Some(2),
                    None
                ),
                (
                    "https://cdn.example/c.png",
                    BlockedKind::Stylesheet,
                    Some(2),
                    None
                ),
                (
                    "https://cdn.example/x.js",
                    BlockedKind::Script,
                    Some(3),
                    None
                ),
                ("https://evil.example/f", BlockedKind::Frame, Some(4), None),
            ]
        );
        assert!(out.html.contains("texte"));
    }

    #[test]
    fn inlines_cid_images() {
        let mut cids = HashMap::new();
        cids.insert(
            "logo@x".to_string(),
            ("image/png".to_string(), vec![1, 2, 3]),
        );
        let out = sanitize(
            r#"<img src="cid:LOGO@x"><td style="background:url(cid:logo@x)">"#,
            &cids,
        );
        assert!(
            out.html.contains("data:image/png;base64,AQID"),
            "{}",
            out.html
        );
        assert!(out.blocked.is_empty());
    }
}
