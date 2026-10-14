//! Cœur de MailForensics : modèle unifié, lecture en lecture seule et empreintes.
#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::panic)]

pub mod model;

pub use model::*;

use md5::Md5;
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::Path;

/// Profondeur maximale d'imbrication de messages (data-model.md, Limites).
pub const MAX_NESTING: usize = 10;
/// Taille maximale d'une preuve chargée en mémoire.
pub const MAX_EVIDENCE_SIZE: u64 = 2 * 1024 * 1024 * 1024;

#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    #[error("E/S sur {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("preuve trop volumineuse ({size} o > {max} o)")]
    TooLarge { size: u64, max: u64 },
    #[error("format non reconnu")]
    UnknownFormat,
    #[error("erreur de parsing : {0}")]
    Parse(String),
}

pub fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    bytes
        .iter()
        .fold(String::with_capacity(bytes.len() * 2), |mut s, b| {
            let _ = write!(s, "{b:02x}");
            s
        })
}

/// SHA-256 et MD5 d'un contenu, en hexadécimal minuscule.
pub fn hashes(data: &[u8]) -> (String, String) {
    (hex(&Sha256::digest(data)), hex(&Md5::digest(data)))
}

/// Détection du format par signature, jamais par extension (FR-002).
pub fn detect_format(data: &[u8]) -> Format {
    const OLE: [u8; 8] = [0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];
    if data.starts_with(&OLE) {
        return Format::Msg; // confirmé par la présence de __properties_version1.0 lors du parsing
    }
    if data.starts_with(b"!BDN") {
        return Format::Pst;
    }
    // EML : un en-tête « Nom: valeur » plausible dans les premiers octets.
    let head = &data[..data.len().min(4096)];
    let text = String::from_utf8_lossy(head);
    let looks_like_header = text.lines().take(20).any(|l| {
        l.split_once(':').is_some_and(|(n, _)| {
            !n.is_empty() && n.bytes().all(|b| b.is_ascii_graphic() && b != b':')
        })
    });
    if looks_like_header {
        Format::Eml
    } else {
        Format::Unknown
    }
}

/// Lit une preuve **en lecture seule**, une seule fois, et calcule ses empreintes
/// avant tout parsing (Constitution I). Les octets retournés sont ceux qui ont été hachés.
pub fn read_evidence(path: &Path) -> Result<(Evidence, Vec<u8>), EngineError> {
    let io = |source| EngineError::Io {
        path: path.display().to_string(),
        source,
    };
    let mut file = std::fs::OpenOptions::new()
        .read(true)
        .open(path)
        .map_err(io)?;
    let size = file.metadata().map_err(io)?.len();
    if size > MAX_EVIDENCE_SIZE {
        return Err(EngineError::TooLarge {
            size,
            max: MAX_EVIDENCE_SIZE,
        });
    }
    let mut data = Vec::with_capacity(usize::try_from(size).unwrap_or(0));
    file.read_to_end(&mut data).map_err(io)?;
    let evidence = evidence_from_bytes(&path.display().to_string(), &data);
    Ok((evidence, data))
}

/// Identification d'une preuve dont les octets ont déjà été lus (version web : le navigateur
/// fournit le contenu). `path` est l'étiquette affichée (chemin relatif côté web).
pub fn evidence_from_bytes(path: &str, data: &[u8]) -> Evidence {
    let (sha256, md5) = hashes(data);
    Evidence {
        path: path.to_string(),
        size: data.len() as u64,
        sha256,
        md5,
        format: detect_format(data),
        declared_extension: Path::new(path)
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase()),
    }
}

/// Nom de fichier sûr pour l'enregistrement d'une pièce jointe (FR-038) : sans chemin, sans
/// caractères de contrôle ni de direction (U+202E « RLO » qui maquille `exe.pdf` en `fdp.exe`),
/// sans nom réservé Windows. Le nom d'origine reste affiché tel quel dans l'interface.
pub fn safe_file_name(name: &str) -> String {
    let base = name.rsplit(['/', '\\']).next().unwrap_or(name);
    let cleaned: String = base
        .chars()
        .filter(|c| {
            !c.is_control()
                && !matches!(c, '\u{200E}' | '\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}')
        })
        .map(|c| if matches!(c, '<' | '>' | ':' | '"' | '|' | '?' | '*') { '_' } else { c })
        .collect();
    let trimmed = cleaned.trim().trim_matches('.').trim();
    let mut out: String = trimmed.chars().take(200).collect();
    if out.is_empty() {
        out = "piece-jointe".to_string();
    }
    let stem = out.split('.').next().unwrap_or("").to_ascii_uppercase();
    let reserved = ["CON", "PRN", "AUX", "NUL"].contains(&stem.as_str())
        || (stem.len() == 4
            && (stem.starts_with("COM") || stem.starts_with("LPT"))
            && stem.as_bytes()[3].is_ascii_digit());
    if reserved {
        out.insert(0, '_');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hashes_known_vectors() {
        let (s, m) = hashes(b"abc");
        assert_eq!(
            s,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(m, "900150983cd24fb0d6963f7d28e17f72");
    }

    #[test]
    fn safe_file_names() {
        assert_eq!(safe_file_name("../../etc/passwd"), "passwd");
        assert_eq!(safe_file_name("C:\\Windows\\evil.exe"), "evil.exe");
        assert_eq!(safe_file_name("facture\u{202E}fdp.exe"), "facturefdp.exe");
        assert_eq!(safe_file_name("a\tb\r\n.txt"), "ab.txt");
        assert_eq!(safe_file_name("CON.txt"), "_CON.txt");
        assert_eq!(safe_file_name("..."), "piece-jointe");
        assert_eq!(safe_file_name("rapport: 2026?.pdf"), "rapport_ 2026_.pdf");
    }

    #[test]
    fn detects_by_signature() {
        assert_eq!(
            detect_format(b"From: a@b.c\r\nSubject: x\r\n\r\nhi"),
            Format::Eml
        );
        assert_eq!(
            detect_format(&[0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1, 0]),
            Format::Msg
        );
        assert_eq!(detect_format(b"!BDN...."), Format::Pst);
        assert_eq!(detect_format(&[0u8, 1, 2, 3]), Format::Unknown);
    }
}
