//! Analyse d'un message normalisé : HTML sûr et liste d'URL (constats, IOC… à venir).
#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::panic)]

pub mod html;
pub mod urls;

use mf_core::{Disposition, Message};
use std::collections::HashMap;

/// Enrichit le message et, récursivement, ses messages embarqués.
pub fn analyze(msg: &mut Message) {
    if let Some(raw_html) = &msg.body_html {
        let cids: HashMap<String, (String, Vec<u8>)> = msg
            .attachments
            .iter()
            .filter(|a| a.disposition == Disposition::Inline || a.content_id.is_some())
            .filter_map(|a| {
                let cid = a.content_id.as_ref()?;
                let mime = a
                    .declared_mime
                    .clone()
                    .filter(|m| m.starts_with("image/"))?;
                Some((cid.to_ascii_lowercase(), (mime, a.data.clone())))
            })
            .collect();
        let safe = html::sanitize(raw_html, &cids);
        msg.body_html_safe = Some(safe.html);
        msg.blocked_remote_resources = safe.blocked.len() as u32;
        msg.blocked_resources = safe.blocked;
    }
    msg.urls = urls::extract(msg);
    for a in &mut msg.attachments {
        if let Some(inner) = a.embedded.as_deref_mut() {
            analyze(inner);
        }
    }
}
