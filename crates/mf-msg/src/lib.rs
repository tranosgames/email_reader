//! Lecture des messages Outlook `.msg` (MS-OXMSG) au-dessus de `cfb` (MS-CFB).
//!
//! Couche MAPI volontairement maison (plan.md › Complexity Tracking) : elle donne accès aux
//! propriétés brutes, à l'adresse SMTP réelle de l'expéditeur et aux en-têtes de transport.
#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::panic)]

mod props;

use mf_core::{Address, Attachment, Disposition, EngineError, MAX_NESTING, Message, hashes};
use props::Node;
use std::collections::BTreeMap;
use std::io::Read;

/// Nombre maximal d'entrées OLE acceptées (data-model.md › Limites).
const MAX_OLE_ENTRIES: usize = 100_000;

/// Tous les flux du fichier composé, indexés par chemin (`/a/b/__substg1.0_0037001F`).
pub(crate) type Streams = BTreeMap<String, Vec<u8>>;

/// Parse un `.msg`.
pub fn parse(raw: &[u8]) -> Result<Message, EngineError> {
    let streams = read_streams(raw)?;
    if !streams.contains_key("/__properties_version1.0") {
        return Err(EngineError::Parse(
            "fichier OLE sans __properties_version1.0 : ce n'est pas un message Outlook".into(),
        ));
    }
    Ok(convert(&Node::root(&streams), 0))
}

fn read_streams(raw: &[u8]) -> Result<Streams, EngineError> {
    let mut comp = cfb::CompoundFile::open(std::io::Cursor::new(raw))
        .map_err(|e| EngineError::Parse(format!("conteneur OLE illisible : {e}")))?;
    let paths: Vec<_> = comp
        .walk()
        .filter(|e| e.is_stream())
        .map(|e| e.path().to_path_buf())
        .take(MAX_OLE_ENTRIES + 1)
        .collect();
    if paths.len() > MAX_OLE_ENTRIES {
        return Err(EngineError::Parse(format!(
            "plus de {MAX_OLE_ENTRIES} entrées OLE"
        )));
    }
    let mut out = Streams::new();
    for p in paths {
        let mut buf = Vec::new();
        comp.open_stream(&p)
            .and_then(|mut s| s.read_to_end(&mut buf))
            .map_err(|e| EngineError::Parse(format!("flux {} illisible : {e}", p.display())))?;
        out.insert(p.to_string_lossy().replace('\\', "/"), buf);
    }
    Ok(out)
}

// Identifiants de propriétés MAPI utilisés (MS-OXPROPS).
const PR_SUBJECT: u16 = 0x0037;
const PR_CLIENT_SUBMIT_TIME: u16 = 0x0039;
const PR_SENT_REPRESENTING_NAME: u16 = 0x0042;
const PR_SENT_REPRESENTING_ADDRTYPE: u16 = 0x0064;
const PR_SENT_REPRESENTING_EMAIL: u16 = 0x0065;
const PR_TRANSPORT_MESSAGE_HEADERS: u16 = 0x007D;
const PR_SENDER_NAME: u16 = 0x0C1A;
const PR_SENDER_ADDRTYPE: u16 = 0x0C1E;
const PR_SENDER_EMAIL: u16 = 0x0C1F;
const PR_RECIPIENT_TYPE: u16 = 0x0C15;
const PR_MESSAGE_DELIVERY_TIME: u16 = 0x0E06;
const PR_BODY: u16 = 0x1000;
const PR_HTML: u16 = 0x1013;
const PR_RTF_COMPRESSED: u16 = 0x1009;
const PR_INTERNET_MESSAGE_ID: u16 = 0x1035;
const PR_DISPLAY_NAME: u16 = 0x3001;
const PR_ADDRTYPE: u16 = 0x3002;
const PR_EMAIL_ADDRESS: u16 = 0x3003;
const PR_CREATION_TIME: u16 = 0x3007;
const PR_ATTACH_DATA: u16 = 0x3701;
const PR_ATTACH_FILENAME: u16 = 0x3704;
const PR_ATTACH_METHOD: u16 = 0x3705;
const PR_ATTACH_LONG_FILENAME: u16 = 0x3707;
const PR_ATTACH_MIME_TAG: u16 = 0x370E;
const PR_ATTACH_CONTENT_ID: u16 = 0x3712;
const PR_SMTP_ADDRESS: u16 = 0x39FE;
const PR_SENDER_SMTP_ADDRESS: u16 = 0x5D01;
const PR_SENT_REPRESENTING_SMTP_ADDRESS: u16 = 0x5D02;
const PR_ATTACHMENT_HIDDEN: u16 = 0x7FFE;

/// Construit une adresse en séparant SMTP et DN Exchange X.500.
fn address(
    name: Option<String>,
    email: Option<String>,
    addrtype: Option<String>,
    smtp: Option<String>,
) -> Address {
    let is_ex = addrtype
        .as_deref()
        .is_some_and(|t| t.eq_ignore_ascii_case("EX"))
        || email
            .as_deref()
            .is_some_and(|e| e.starts_with("/O=") || e.starts_with("/o="));
    let (address, x500_dn) = if is_ex {
        (smtp, email)
    } else {
        (smtp.or(email), None)
    };
    Address {
        display_name: name,
        address,
        x500_dn,
    }
}

fn convert(node: &Node<'_>, depth: usize) -> Message {
    let mut out = Message {
        subject: node.string(PR_SUBJECT),
        ..Default::default()
    };

    // En-têtes de transport Internet (FR-011, FR-035).
    let transport = node.string(PR_TRANSPORT_MESSAGE_HEADERS);
    match &transport {
        Some(text) => out.headers = mf_eml::parse_header_block(text),
        None => out.issues.push(
            "en-têtes de transport absents (PR_TRANSPORT_MESSAGE_HEADERS 0x007D) : message interne ou brouillon".into(),
        ),
    }
    let header = |name: &str| {
        out.headers
            .iter()
            .find(|h| h.name.eq_ignore_ascii_case(name))
            .map(|h| h.value.clone())
    };

    // Expéditeur : « au nom de » (sent representing) = From ; sender = compte émetteur.
    let sender = address(
        node.string(PR_SENDER_NAME),
        node.string(PR_SENDER_EMAIL),
        node.string(PR_SENDER_ADDRTYPE),
        node.string(PR_SENDER_SMTP_ADDRESS),
    );
    let representing = address(
        node.string(PR_SENT_REPRESENTING_NAME),
        node.string(PR_SENT_REPRESENTING_EMAIL),
        node.string(PR_SENT_REPRESENTING_ADDRTYPE),
        node.string(PR_SENT_REPRESENTING_SMTP_ADDRESS),
    );
    let has = |a: &Address| a.address.is_some() || a.x500_dn.is_some() || a.display_name.is_some();
    if has(&representing) {
        if has(&sender) && sender != representing {
            out.sender = Some(sender);
        }
        out.from.push(representing);
    } else if has(&sender) {
        out.from.push(sender);
    }

    // Destinataires.
    for recip in node.children("__recip_version1.0_#") {
        let r = address(
            recip.string(PR_DISPLAY_NAME),
            recip.string(PR_EMAIL_ADDRESS),
            recip.string(PR_ADDRTYPE),
            recip.string(PR_SMTP_ADDRESS),
        );
        match recip.fixed_u32(PR_RECIPIENT_TYPE).map(|t| t & 0x0F) {
            Some(2) => out.cc.push(r),
            Some(3) => out.bcc.push(r),
            _ => out.to.push(r),
        }
    }

    // Dates.
    out.date_raw = header("Date");
    let (date_prop, date_name) = [
        (PR_CLIENT_SUBMIT_TIME, "PR_CLIENT_SUBMIT_TIME"),
        (PR_MESSAGE_DELIVERY_TIME, "PR_MESSAGE_DELIVERY_TIME"),
        (PR_CREATION_TIME, "PR_CREATION_TIME"),
    ]
    .into_iter()
    .find_map(|(id, n)| node.fixed_time(id).map(|t| (t, n)))
    .map_or((None, None), |(t, n)| (Some(t), Some(n)));
    out.date_utc = date_prop.map(|t| t.to_rfc3339());
    if out.date_raw.is_none() {
        out.date_raw = date_name
            .zip(out.date_utc.clone())
            .map(|(n, t)| format!("{n} = {t}"));
    }
    out.message_id = node
        .string(PR_INTERNET_MESSAGE_ID)
        .or_else(|| header("Message-ID"));
    if out.reply_to.is_empty() {
        if let Some(rt) = header("Reply-To") {
            out.reply_to.push(Address {
                address: Some(rt),
                ..Default::default()
            });
        }
    }

    // Corps.
    out.body_text = node.string(PR_BODY);
    out.body_html = node
        .binary(PR_HTML)
        .map(|b| node.decode_internet(b))
        .or_else(|| node.string(PR_HTML));
    if out.body_text.is_none()
        && out.body_html.is_none()
        && node.binary(PR_RTF_COMPRESSED).is_some()
    {
        out.issues
            .push("corps présent uniquement en RTF compressé (rendu RTF : tâche T030)".into());
    }

    // Pièces jointes (FR-016, FR-036) et messages embarqués (FR-018).
    let html_lower = out
        .body_html
        .as_deref()
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();
    let referenced = |cid: Option<&str>| {
        cid.is_some_and(|c| html_lower.contains(&format!("cid:{}", c.to_ascii_lowercase())))
    };
    for (i, att) in node
        .children("__attach_version1.0_#")
        .into_iter()
        .enumerate()
    {
        let index = i as u32 + 1;
        let method = att.fixed_u32(PR_ATTACH_METHOD);
        let content_id = att.string(PR_ATTACH_CONTENT_ID);
        let hidden = att.fixed_bool(PR_ATTACHMENT_HIDDEN).unwrap_or(false);
        let mut filename = att
            .string(PR_ATTACH_LONG_FILENAME)
            .or_else(|| att.string(PR_ATTACH_FILENAME))
            .or_else(|| att.string(PR_DISPLAY_NAME));
        let embedded_node = att.object(PR_ATTACH_DATA);
        let (data, size, sha256, md5, embedded) = match embedded_node {
            Some(sub) => {
                // Pas d'octets « fichier » pour un message embarqué : empreinte calculée sur
                // l'ensemble canonique de ses flux (chemin relatif + longueur + contenu, triés).
                let canon = sub.canonical_bytes();
                let (s, m) = hashes(&canon);
                let size = sub.total_len();
                let emb = if depth + 1 < MAX_NESTING {
                    Some(Box::new(convert(&sub, depth + 1)))
                } else {
                    out.issues
                        .push(format!("imbrication tronquée à {MAX_NESTING} niveaux"));
                    None
                };
                if filename.is_none() {
                    filename = emb
                        .as_ref()
                        .and_then(|m| m.subject.clone())
                        .map(|s| format!("{s}.msg"));
                }
                (Vec::new(), size, s, m, emb)
            }
            None => {
                let data = att
                    .binary(PR_ATTACH_DATA)
                    .map(<[u8]>::to_vec)
                    .unwrap_or_default();
                let (s, m) = hashes(&data);
                (data.clone(), data.len() as u64, s, m, None)
            }
        };
        if method == Some(5) && embedded.is_none() && depth + 1 < MAX_NESTING {
            out.issues.push(format!(
                "PJ {index} : méthode 5 (message embarqué) sans stockage 0x3701000D"
            ));
        }
        out.attachments.push(Attachment {
            index,
            filename: filename.unwrap_or_else(|| "(sans nom)".into()),
            declared_mime: att.string(PR_ATTACH_MIME_TAG),
            size,
            sha256,
            md5,
            // Outlook pose souvent un Content-ID sur des PJ ordinaires : « inline » seulement si
            // la PJ est masquée ou réellement référencée par le corps HTML (cid:…).
            disposition: if hidden || referenced(content_id.as_deref()) {
                Disposition::Inline
            } else {
                Disposition::Attachment
            },
            content_id,
            embedded,
            data,
        });
    }
    out
}
