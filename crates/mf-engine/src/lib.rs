//! Point d'entrée unique du moteur : ouvrir une preuve, la parser et l'analyser.
//! Utilisé à l'identique par la CLI `mf` et l'application de bureau (Constitution III).
#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::panic)]

use mf_core::{EngineError, Evidence, Format, Message};
use serde::Serialize;
use std::path::Path;

/// Une preuve ouverte et analysée.
#[derive(Debug, Serialize)]
pub struct Analyzed {
    pub evidence: Evidence,
    pub message: Message,
}

/// Lit (lecture seule), hache, parse et analyse un fichier `.eml` ou `.msg`.
pub fn open(path: &Path) -> Result<Analyzed, EngineError> {
    let (evidence, bytes) = mf_core::read_evidence(path)?;
    let mut message = match evidence.format {
        Format::Eml => mf_eml::parse(&bytes, 0)?,
        Format::Msg => mf_msg::parse(&bytes)?,
        Format::Pst => return Err(EngineError::Parse("PST : prévu dans la feature 002".into())),
        Format::Unknown => return Err(EngineError::UnknownFormat),
    };
    expand_attached_messages(&mut message, 0);
    mf_analysis::analyze(&mut message);
    Ok(Analyzed { evidence, message })
}

/// Parcourt un message et ses messages embarqués selon un chemin d'indices de PJ (`[2, 1]`).
pub fn descend<'a>(root: &'a Message, path: &[u32]) -> Option<&'a Message> {
    path.iter().try_fold(root, |m, idx| {
        m.attachments
            .iter()
            .find(|a| a.index == *idx)?
            .embedded
            .as_deref()
    })
}

/// Une PJ binaire qui est elle-même un `.msg` ou un `.eml` (ex. « transférer en pièce jointe »)
/// est ouverte comme message embarqué, quel que soit le format parent (FR-018).
fn expand_attached_messages(msg: &mut Message, depth: usize) {
    if depth + 1 >= mf_core::MAX_NESTING {
        return;
    }
    for a in &mut msg.attachments {
        if a.embedded.is_none() && !a.data.is_empty() {
            let name = a.filename.to_ascii_lowercase();
            let mime = a
                .declared_mime
                .as_deref()
                .unwrap_or("")
                .to_ascii_lowercase();
            let parsed = match mf_core::detect_format(&a.data) {
                Format::Msg => mf_msg::parse(&a.data).ok(),
                Format::Eml if name.ends_with(".eml") || mime == "message/rfc822" => {
                    mf_eml::parse(&a.data, depth + 1).ok()
                }
                _ => None,
            };
            a.embedded = parsed.map(Box::new);
        }
        if let Some(inner) = a.embedded.as_deref_mut() {
            expand_attached_messages(inner, depth + 1);
        }
    }
}
