//! Liste des URL d'un message (FR-037) : en-têtes, corps texte, attributs/texte/CSS du HTML.

use linkify::{LinkFinder, LinkKind};
use mf_core::{Message, UrlEntry, UrlLocation};
use regex::Regex;
use std::sync::LazyLock;

static ATTR: LazyLock<Option<Regex>> = LazyLock::new(|| {
    Regex::new(r#"(?is)\b(href|src|action|formaction|background|poster|data|cite|longdesc|srcset)\s*=\s*(?:"([^"]*)"|'([^']*)'|([^\s>]+))"#).ok()
});
static CSS_URL: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r#"(?i)url\(\s*['"]?([^'")\s]+)['"]?\s*\)"#).ok());
static TAG: LazyLock<Option<Regex>> = LazyLock::new(|| Regex::new(r"(?s)<[^>]*>").ok());
static WWW: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r#"(?i)\bwww\.[a-z0-9-]+(\.[a-z0-9-]+)+[^\s<>"'`]*"#).ok());
static SCHEME: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"(?i)^[a-z][a-z0-9+.-]{1,15}:").ok());

/// Schémas ignorés : références internes au message, pas des destinations.
const IGNORED_SCHEMES: [&str; 3] = ["cid:", "data:", "about:"];

#[derive(Default)]
struct Collector {
    entries: Vec<UrlEntry>,
}

impl Collector {
    fn add(&mut self, raw: &str, loc: UrlLocation) {
        let url = raw
            .trim()
            .trim_end_matches(['.', ',', ';', ':', ')', ']', '>', '\'', '"', '!', '?']);
        if url.len() < 4 {
            return;
        }
        let lower = url.to_ascii_lowercase();
        if IGNORED_SCHEMES.iter().any(|s| lower.starts_with(s)) {
            return;
        }
        let is_url = lower.starts_with("www.") || SCHEME.as_ref().is_some_and(|r| r.is_match(url));
        if !is_url {
            return; // chemin relatif ou ancre : pas une destination exploitable
        }
        if let Some(e) = self.entries.iter_mut().find(|e| e.url == url) {
            e.occurrences += 1;
            if !e.locations.contains(&loc) {
                e.locations.push(loc);
            }
            return;
        }
        self.entries.push(UrlEntry {
            url: url.to_string(),
            defanged: defang(url),
            host: host(url),
            locations: vec![loc],
            occurrences: 1,
        });
    }

    fn scan_text(&mut self, text: &str, loc: &UrlLocation) {
        let mut finder = LinkFinder::new();
        finder.kinds(&[LinkKind::Url]);
        for l in finder.links(text) {
            self.add(l.as_str(), loc.clone());
        }
        if let Some(www) = WWW.as_ref() {
            for m in www.find_iter(text) {
                // Évite le doublon « www.x » quand « http://www.x » vient d'être capturé.
                let start = m.start();
                let preceded_by_scheme = text[..start].ends_with("://");
                if !preceded_by_scheme {
                    self.add(m.as_str(), loc.clone());
                }
            }
        }
    }
}

/// Extrait les URL uniques d'un message, dans l'ordre de première apparition.
pub fn extract(msg: &Message) -> Vec<UrlEntry> {
    let mut c = Collector::default();
    for h in &msg.headers {
        c.scan_text(
            &h.value,
            &UrlLocation::Header {
                name: h.name.clone(),
            },
        );
    }
    if let Some(text) = &msg.body_text {
        c.scan_text(text, &UrlLocation::BodyText);
    }
    if let Some(html) = &msg.body_html {
        if let Some(attr) = ATTR.as_ref() {
            for cap in attr.captures_iter(html) {
                let name = cap.get(1).map_or("", |m| m.as_str()).to_ascii_lowercase();
                let value = cap
                    .get(2)
                    .or(cap.get(3))
                    .or(cap.get(4))
                    .map_or("", |m| m.as_str());
                let value = decode_entities(value);
                let loc = UrlLocation::HtmlAttribute {
                    attribute: name.clone(),
                };
                if name == "srcset" {
                    for part in value.split(',') {
                        if let Some(u) = part.split_whitespace().next() {
                            c.add(u, loc.clone());
                        }
                    }
                } else {
                    c.add(&value, loc);
                }
            }
        }
        if let Some(css) = CSS_URL.as_ref() {
            for cap in css.captures_iter(html) {
                if let Some(u) = cap.get(1) {
                    c.add(&decode_entities(u.as_str()), UrlLocation::HtmlCss);
                }
            }
        }
        if let Some(tag) = TAG.as_ref() {
            let text = decode_entities(&tag.replace_all(html, " "));
            c.scan_text(&text, &UrlLocation::HtmlText);
        }
    }
    c.entries
}

/// Hôte d'une URL (sans identifiants ni port), en minuscules.
pub fn host(url: &str) -> Option<String> {
    let rest = match url.find("://") {
        Some(i) => &url[i + 3..],
        None if url.to_ascii_lowercase().starts_with("www.") => url,
        None => return url.split_once('@').map(|(_, d)| d.to_ascii_lowercase()),
    };
    let authority = rest.split(['/', '?', '#']).next()?;
    let host = authority.rsplit('@').next()?;
    let host = if host.starts_with('[') {
        host.split(']').next().map(|h| format!("{h}]"))?
    } else {
        host.split(':').next()?.to_string()
    };
    (!host.is_empty()).then(|| host.to_ascii_lowercase())
}

/// Forme « défangée » non cliquable : `hxxps[://]exemple[.]com/x`.
pub fn defang(url: &str) -> String {
    let (scheme, rest) = match url.find("://") {
        Some(i) => (&url[..i], &url[i + 3..]),
        None => ("", url),
    };
    let scheme = match scheme.to_ascii_lowercase().as_str() {
        "http" => "hxxp".to_string(),
        "https" => "hxxps".to_string(),
        "ftp" => "fxp".to_string(),
        s => s.to_string(),
    };
    let (authority, path) = match rest.find(['/', '?', '#']) {
        Some(i) => (&rest[..i], &rest[i..]),
        None => (rest, ""),
    };
    let authority = authority.replace('.', "[.]").replace('@', "[@]");
    if scheme.is_empty() {
        format!("{authority}{path}")
    } else {
        format!("{scheme}[://]{authority}{path}")
    }
}

fn decode_entities(s: &str) -> String {
    if !s.contains('&') {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(i) = rest.find('&') {
        out.push_str(&rest[..i]);
        rest = &rest[i..];
        let Some(end) = rest[..rest.len().min(12)].find(';') else {
            out.push('&');
            rest = &rest[1..];
            continue;
        };
        let ent = &rest[1..end];
        let ch = match ent {
            "amp" => Some('&'),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            "nbsp" => Some(' '),
            _ if ent.starts_with("#x") || ent.starts_with("#X") => {
                u32::from_str_radix(&ent[2..], 16)
                    .ok()
                    .and_then(char::from_u32)
            }
            _ if ent.starts_with('#') => ent[1..].parse().ok().and_then(char::from_u32),
            _ => None,
        };
        match ch {
            Some(c) => {
                out.push(c);
                rest = &rest[end + 1..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use mf_core::Header;

    fn msg(html: &str, text: &str) -> Message {
        Message {
            headers: vec![Header {
                index: 0,
                name: "List-Unsubscribe".into(),
                raw: String::new(),
                value: "<https://news.example/unsub?id=1>".into(),
                offset: None,
            }],
            body_html: Some(html.into()),
            body_text: Some(text.into()),
            ..Default::default()
        }
    }

    #[test]
    fn extracts_dedups_and_locates() {
        let m = msg(
            r##"<a href="http://203.0.113.7/login?a=1&amp;b=2">www.banque.fr</a>
               <a href="http://203.0.113.7/login?a=1&amp;b=2">encore</a>
               <img src="cid:logo"><div style="background:url('https://cdn.example/bg.png')"></div>
               <a href="#top">ancre</a><a href="mailto:x@y.example">m</a>"##,
            "Voir https://docs.example/page. Merci",
        );
        let urls = extract(&m);
        let list: Vec<&str> = urls.iter().map(|u| u.url.as_str()).collect();
        assert_eq!(
            list,
            [
                "https://news.example/unsub?id=1",
                "https://docs.example/page",
                "http://203.0.113.7/login?a=1&b=2",
                "mailto:x@y.example",
                "https://cdn.example/bg.png",
                "www.banque.fr",
            ]
        );
        let login = &urls[2];
        assert_eq!(login.occurrences, 2);
        assert_eq!(
            login.locations,
            vec![UrlLocation::HtmlAttribute {
                attribute: "href".into()
            }]
        );
        assert_eq!(login.host.as_deref(), Some("203.0.113.7"));
        assert_eq!(login.defanged, "hxxp[://]203[.]0[.]113[.]7/login?a=1&b=2");
    }

    #[test]
    fn defang_and_host() {
        assert_eq!(
            defang("https://user@a.b.example:8443/x.y"),
            "hxxps[://]user[@]a[.]b[.]example:8443/x.y"
        );
        assert_eq!(
            host("https://user@A.Example:8443/x").as_deref(),
            Some("a.example")
        );
        assert_eq!(host("www.exemple.fr/a").as_deref(), Some("www.exemple.fr"));
    }
}
