//! Conversion d'un courriel RFC 5322 (EML) vers le modèle unifié, via `mail-parser`.
#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::panic)]

use mail_parser::{HeaderValue, MessageParser, MimeHeaders, PartType};
use mf_core::{
    Address, Attachment, Disposition, EngineError, Header, MAX_NESTING, Message, hashes,
};

/// Parse un EML. `depth` = niveau d'imbrication (0 pour la preuve elle-même).
pub fn parse(raw: &[u8], depth: usize) -> Result<Message, EngineError> {
    let parsed = MessageParser::default()
        .parse(raw)
        .ok_or_else(|| EngineError::Parse("aucun message RFC 5322 reconnu".into()))?;
    Ok(convert(&parsed, raw, depth))
}

/// Parse un bloc d'en-têtes seul (ex. propriété MSG 0x007D) en conservant ordre et offsets.
pub fn parse_header_block(text: &str) -> Vec<Header> {
    let mut buf = text.as_bytes().to_vec();
    buf.extend_from_slice(b"\r\n\r\n");
    MessageParser::default()
        .parse(&buf)
        .map(|m| collect_headers(&m, &buf))
        .unwrap_or_default()
}

fn unfold(s: &str) -> String {
    s.split(['\r', '\n'])
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

fn collect_headers(msg: &mail_parser::Message<'_>, raw: &[u8]) -> Vec<Header> {
    msg.headers()
        .iter()
        .enumerate()
        .map(|(i, h)| {
            let field = h.offset_field() as usize;
            let (start, end) = (h.offset_start() as usize, h.offset_end() as usize);
            let slice = |a: usize, b: usize| {
                raw.get(a..b)
                    .map(|s| String::from_utf8_lossy(s).into_owned())
            };
            let raw_value = slice(start, end).unwrap_or_default();
            let value = match h.value() {
                HeaderValue::Text(t) => t.to_string(),
                _ => unfold(&raw_value),
            };
            Header {
                index: i as u32,
                // Nom tel qu'écrit dans la preuve (mail-parser normalise la casse des noms connus).
                name: slice(field, start)
                    .and_then(|n| n.split(':').next().map(|n| n.trim().to_string()))
                    .filter(|n| !n.is_empty())
                    .unwrap_or_else(|| h.name().to_string()),
                raw: slice(field, end).unwrap_or_default().trim_end().to_string(),
                value,
                offset: Some((field as u64, end as u64)),
            }
        })
        .collect()
}

fn addresses(a: Option<&mail_parser::Address<'_>>) -> Vec<Address> {
    a.map(|a| {
        a.iter()
            .map(|x| Address {
                display_name: x.name().map(str::to_string),
                address: x.address().map(str::to_string),
                x500_dn: None,
            })
            .collect()
    })
    .unwrap_or_default()
}

fn convert(msg: &mail_parser::Message<'_>, raw: &[u8], depth: usize) -> Message {
    let mut out = Message {
        subject: msg.subject().map(str::to_string),
        from: addresses(msg.from()),
        sender: addresses(msg.sender()).into_iter().next(),
        reply_to: addresses(msg.reply_to()),
        to: addresses(msg.to()),
        cc: addresses(msg.cc()),
        bcc: addresses(msg.bcc()),
        date_raw: msg.header_raw("Date").map(|d| d.trim().to_string()),
        date_utc: msg.date().and_then(|d| {
            chrono::DateTime::from_timestamp(d.to_timestamp(), 0).map(|t| t.to_rfc3339())
        }),
        message_id: msg.message_id().map(str::to_string),
        headers: collect_headers(msg, raw),
        body_text: msg.text_bodies().find_map(|p| match &p.body {
            PartType::Text(t) => Some(t.to_string()),
            _ => None,
        }),
        body_html: msg.html_bodies().find_map(|p| match &p.body {
            PartType::Html(t) => Some(t.to_string()),
            _ => None,
        }),
        ..Default::default()
    };
    if out.date_raw.is_some() && out.date_utc.is_none() {
        out.issues.push("en-tête Date non interprétable".into());
    }

    for (i, part) in msg.attachments().enumerate() {
        let declared_mime = part.content_type().map(|ct| match ct.subtype() {
            Some(sub) => format!("{}/{}", ct.ctype(), sub),
            None => ct.ctype().to_string(),
        });
        let disposition = match part.content_disposition() {
            Some(cd) if cd.is_inline() => Disposition::Inline,
            Some(cd) if cd.is_attachment() => Disposition::Attachment,
            _ if part.content_id().is_some() => Disposition::Inline,
            _ => Disposition::Unknown,
        };
        let (data, embedded) = match &part.body {
            PartType::Message(inner) => {
                let bytes = inner.raw_message().to_vec();
                let emb = if depth + 1 < MAX_NESTING {
                    Some(Box::new(convert(inner, inner.raw_message(), depth + 1)))
                } else {
                    out.issues
                        .push(format!("imbrication tronquée à {MAX_NESTING} niveaux"));
                    None
                };
                (bytes, emb)
            }
            _ => (part.contents().to_vec(), None),
        };
        let (sha256, md5) = hashes(&data);
        let default_name = if embedded.is_some() {
            "message.eml"
        } else {
            "(sans nom)"
        };
        out.attachments.push(Attachment {
            index: i as u32 + 1,
            filename: part
                .attachment_name()
                .map(str::to_string)
                .or_else(|| {
                    embedded
                        .as_ref()
                        .and_then(|m| m.subject.clone())
                        .map(|s| format!("{s}.eml"))
                })
                .unwrap_or_else(|| default_name.to_string()),
            declared_mime,
            size: data.len() as u64,
            sha256,
            md5,
            disposition,
            content_id: part
                .content_id()
                .map(|c| c.trim_matches(['<', '>']).to_string()),
            embedded,
            data,
        });
    }
    out
}
