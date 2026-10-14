//! Rendu HTML sûr (FR-008, FR-009) : sanitisation + blocage des ressources distantes.
//!
//! Première couche de défense ; la seconde est l'iframe `sandbox` + CSP `default-src 'none'`
//! côté interface (research.md R4).

use base64::Engine;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

/// Résultat du nettoyage.
pub struct SafeHtml {
    pub html: String,
    /// Images, CSS, polices… distants supprimés.
    pub blocked_remote: u32,
}

/// `cid_images` : Content-ID (sans chevrons) → (type MIME, octets) des images inline.
pub fn sanitize(html: &str, cid_images: &HashMap<String, (String, Vec<u8>)>) -> SafeHtml {
    let blocked = Arc::new(AtomicU32::new(0));
    let counter = Arc::clone(&blocked);
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

    let mut b = ammonia::Builder::default();
    b.add_url_schemes(["data", "cid"])
        .add_generic_attributes([
            "style", "align", "valign", "width", "height", "bgcolor", "color",
        ])
        .link_rel(None)
        .strip_comments(true)
        .attribute_filter(move |element, attribute, value| {
            let v = value.trim();
            let lower = v.to_ascii_lowercase();
            match (element, attribute) {
                // Aucun lien n'est navigable dans l'outil : la cible figure dans la liste des URL.
                (_, "href") => None,
                (_, "src") | (_, "background") | (_, "srcset") | (_, "poster") => {
                    if let Some(cid) = lower.strip_prefix("cid:") {
                        data_uris.get(cid).cloned().map(Into::into)
                    } else if lower.starts_with("data:image/") {
                        Some(value.into())
                    } else {
                        counter.fetch_add(1, Ordering::Relaxed);
                        None
                    }
                }
                (_, "style")
                    if lower.contains("url(")
                        || lower.contains("expression(")
                        || lower.contains("@import") =>
                {
                    counter.fetch_add(1, Ordering::Relaxed);
                    None
                }
                _ => Some(value.into()),
            }
        });
    let html = b.clean(html).to_string();
    SafeHtml {
        html,
        blocked_remote: blocked.load(Ordering::Relaxed),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removes_scripts_remote_images_and_links() {
        let out = sanitize(
            r#"<p onclick="x()">Hi <script>alert(1)</script><img src="http://t.example/p.gif">
               <a href="http://evil.example">clic</a><div style="background:url(http://x/y)">z</div></p>"#,
            &HashMap::new(),
        );
        assert!(!out.html.contains("script"));
        assert!(!out.html.contains("onclick"));
        assert!(!out.html.contains("http://"));
        assert_eq!(out.blocked_remote, 2);
        assert!(out.html.contains("clic"));
    }

    #[test]
    fn inlines_cid_images() {
        let mut cids = HashMap::new();
        cids.insert(
            "logo@x".to_string(),
            ("image/png".to_string(), vec![1, 2, 3]),
        );
        let out = sanitize(r#"<img src="cid:LOGO@x">"#, &cids);
        assert!(
            out.html.contains("data:image/png;base64,AQID"),
            "{}",
            out.html
        );
        assert_eq!(out.blocked_remote, 0);
    }
}
