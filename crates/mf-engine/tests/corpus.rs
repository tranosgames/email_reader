//! Tests sur le corpus de référence (Constitution V) : T023, T091, T093, SC-007.

use mf_core::{Disposition, UrlLocation};
use std::path::{Path, PathBuf};

fn corpus(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/corpus")
        .join(rel)
}

fn all_corpus_files() -> Vec<PathBuf> {
    let mut out = Vec::new();
    for dir in ["eml", "msg", "nested", "malformed"] {
        for e in std::fs::read_dir(corpus(dir)).expect("dossier corpus") {
            out.push(e.expect("entrée").path());
        }
    }
    out.sort();
    out
}

/// SC-001 / FR-004 : aucune preuve n'est modifiée par l'ouverture et l'analyse.
#[test]
fn evidence_is_never_modified() {
    for f in all_corpus_files() {
        let before = std::fs::read(&f).expect("lecture");
        let mtime = std::fs::metadata(&f)
            .and_then(|m| m.modified())
            .expect("mtime");
        let res = mf_engine::open(&f);
        if let Ok(a) = &res {
            assert_eq!(
                a.evidence.sha256,
                mf_core::hashes(&before).0,
                "{}",
                f.display()
            );
        }
        assert_eq!(
            std::fs::read(&f).expect("relecture"),
            before,
            "{}",
            f.display()
        );
        assert_eq!(
            std::fs::metadata(&f)
                .and_then(|m| m.modified())
                .expect("mtime"),
            mtime
        );
    }
}

/// FR-006 / FR-034 : une entrée malformée produit une erreur, jamais un plantage.
#[test]
fn malformed_inputs_are_errors() {
    assert!(mf_engine::open(&corpus("malformed/not-an-email.bin")).is_err());
    assert!(mf_engine::open(&corpus("malformed/truncated.msg")).is_err());
}

/// T091 / FR-037 : liste d'URL dédoublonnée, ordonnée, avec emplacements.
#[test]
fn url_list_per_email() {
    let a = mf_engine::open(&corpus("eml/html-links-inline.eml")).expect("ouverture");
    let urls: Vec<&str> = a.message.urls.iter().map(|u| u.url.as_str()).collect();
    assert_eq!(
        urls,
        [
            "https://mailer.example/unsub?u=42",
            "https://banque-exemple.example.login-verif.example/releve",
            "www.aide-banque.example/faq",
            "http://198.51.100.23/r?id=42&t=1",
            "https://track.mailer.example/open.gif?u=42",
            "www.banque-exemple.example",
        ]
    );
    let releve = &a.message.urls[1];
    assert_eq!(releve.occurrences, 3);
    assert_eq!(
        releve.locations,
        vec![
            UrlLocation::BodyText,
            UrlLocation::HtmlAttribute {
                attribute: "href".into()
            }
        ]
    );
    assert_eq!(
        a.message.urls[3].defanged,
        "hxxp[://]198[.]51[.]100[.]23/r?id=42&t=1"
    );
}

/// FR-008 / FR-009 : rendu sûr — script retiré, pixel distant bloqué, image cid: intégrée.
#[test]
fn html_is_safe() {
    let a = mf_engine::open(&corpus("eml/html-links-inline.eml")).expect("ouverture");
    let safe = a.message.body_html_safe.as_deref().expect("html sûr");
    assert!(!safe.contains("<script"));
    assert!(!safe.contains("track.mailer.example"));
    assert!(!safe.contains("href="));
    assert!(safe.contains("data:image/png;base64,"));
    assert_eq!(a.message.blocked_remote_resources, 1);
}

/// T093 / FR-036 : fichiers contenus, EML.
#[test]
fn attachments_eml() {
    let a = mf_engine::open(&corpus("eml/phishing-basic.eml")).expect("ouverture");
    let pj = &a.message.attachments;
    assert_eq!(pj.len(), 1);
    assert_eq!(pj[0].filename, "facture.pdf.exe");
    assert_eq!(pj[0].size, 32);
    assert_eq!(pj[0].sha256, mf_core::hashes(&pj[0].data).0);
    assert_eq!(pj[0].disposition, Disposition::Attachment);

    let b = mf_engine::open(&corpus("eml/html-links-inline.eml")).expect("ouverture");
    assert_eq!(b.message.attachments[0].filename, "logo.png");
    assert_eq!(b.message.attachments[0].disposition, Disposition::Inline);
}

/// FR-018 : message embarqué avec ses propres fichiers et URL.
#[test]
fn nested_eml_has_own_files_and_urls() {
    let a = mf_engine::open(&corpus("nested/eml-in-eml.eml")).expect("ouverture");
    assert_eq!(
        a.message.urls[0].url,
        "https://tickets.soc.example/INC-1234"
    );
    let inner = a.message.attachments[0]
        .embedded
        .as_deref()
        .expect("message embarqué");
    assert_eq!(inner.subject.as_deref(), Some("Urgent"));
    assert_eq!(inner.attachments[0].filename, "facture.zip");
    assert_eq!(
        inner
            .urls
            .iter()
            .map(|u| u.url.as_str())
            .collect::<Vec<_>>(),
        ["http://203.0.113.7/login"]
    );
    assert!(mf_engine::descend(&a.message, &[1]).is_some());
}

/// MSG : PJ objet (méthode 5) et fichier .msg joint sont tous deux ouverts récursivement.
#[test]
fn nested_msg() {
    let a = mf_engine::open(&corpus("msg/test_email.msg")).expect("ouverture");
    let pj = &a.message.attachments;
    assert_eq!(pj.len(), 3);
    assert!(pj[0].embedded.is_some(), "objet MAPI embarqué");
    assert!(pj[1].embedded.is_none());
    let joined = pj[2].embedded.as_deref().expect(".msg joint ouvert");
    assert_eq!(joined.subject.as_deref(), Some("Test Email"));
    assert_eq!(joined.attachments.len(), 2);
}

/// SC-007 : concordance avec un parseur indépendant (oracle `msg_parser`).
#[test]
fn msg_concordance_with_oracle() {
    for name in [
        "ascii.msg",
        "unicode.msg",
        "attachment.msg",
        "test_email.msg",
    ] {
        let path = corpus(&format!("msg/{name}"));
        let ours = mf_engine::open(&path).expect("ouverture").message;
        let oracle = msg_parser::Outlook::from_path(&path).expect("oracle");
        assert_eq!(
            ours.subject.as_deref().unwrap_or(""),
            oracle.subject,
            "{name} : objet"
        );
        assert_eq!(
            ours.attachments.len(),
            oracle.attachments.len(),
            "{name} : nombre de PJ"
        );
        assert_eq!(ours.to.len(), oracle.to.len(), "{name} : destinataires À");
        assert_eq!(ours.cc.len(), oracle.cc.len(), "{name} : Cc");
        for (o, r) in ours.attachments.iter().zip(&oracle.attachments) {
            if o.embedded.is_none() || !o.data.is_empty() {
                assert_eq!(
                    o.size as usize,
                    r.payload_bytes.len(),
                    "{name} : taille de {}",
                    o.filename
                );
            }
        }
    }
}

/// FR-035 : en-têtes dans l'ordre d'origine, avec offsets pointant sur la source.
#[test]
fn headers_in_original_order_with_offsets() {
    let path = corpus("eml/phishing-basic.eml");
    let raw = std::fs::read(&path).expect("lecture");
    let a = mf_engine::open(&path).expect("ouverture");
    let names: Vec<&str> = a.message.headers.iter().map(|h| h.name.as_str()).collect();
    assert_eq!(&names[..4], ["Received", "Received", "From", "Reply-To"]);
    let h = &a.message.headers[2];
    let (s, e) = h.offset.expect("offset");
    assert_eq!(
        String::from_utf8_lossy(&raw[s as usize..e as usize]).trim_end(),
        h.raw
    );
    assert_eq!(
        a.message.subject.as_deref(),
        Some("Urgent : mot de passe expiré")
    );
}

/// MSG : en-têtes de transport (0x007D) exposés comme en-têtes.
#[test]
fn msg_transport_headers() {
    let a = mf_engine::open(&corpus("msg/unicode.msg")).expect("ouverture");
    assert_eq!(
        a.message.headers.first().map(|h| h.name.as_str()),
        Some("Return-path")
    );
    assert!(a.message.headers.iter().any(|h| h.name == "Received"));
}

/// FR-W05 : l'ouverture depuis des octets (version web) donne exactement le même résultat.
#[test]
fn open_bytes_matches_open_path() {
    for f in all_corpus_files() {
        let from_path = mf_engine::open(&f)
            .ok()
            .map(|a| serde_json::to_string(&a.message).expect("json"));
        let bytes = std::fs::read(&f).expect("lecture");
        let from_bytes = mf_engine::open_bytes("x", bytes)
            .ok()
            .map(|a| serde_json::to_string(&a.message).expect("json"));
        assert_eq!(from_path, from_bytes, "{}", f.display());
    }
}

/// T098 / FR-038 : extraction octet pour octet, y compris dans un message embarqué.
#[test]
fn extract_attachments() {
    let a = mf_engine::open(&corpus("eml/phishing-basic.eml")).expect("ouverture");
    let x = mf_engine::extract_attachment(&a.message, &[], 1).expect("extraction");
    assert_eq!(x.file_name, "facture.pdf.exe");
    assert_eq!(x.sha256, a.message.attachments[0].sha256);
    assert_eq!(&x.data[..2], b"MZ");

    let n = mf_engine::open(&corpus("nested/eml-in-eml.eml")).expect("ouverture");
    let inner = mf_engine::extract_attachment(&n.message, &[1], 1).expect("PJ du message joint");
    assert_eq!(inner.file_name, "facture.zip");
    let whole = mf_engine::extract_attachment(&n.message, &[], 1).expect("le .eml joint lui-même");
    assert!(whole.data.starts_with(b"From:"));

    // Objet MAPI embarqué : pas de fichier d'origine.
    let m = mf_engine::open(&corpus("msg/test_email.msg")).expect("ouverture");
    assert!(mf_engine::extract_attachment(&m.message, &[], 1).is_err());
    // Fichier .msg joint (octets réels) : extractible et ré-ouvrable à l'identique.
    let joined = mf_engine::extract_attachment(&m.message, &[], 3).expect(".msg joint");
    let reopened = mf_engine::open_bytes(&joined.file_name, joined.data).expect("ré-ouverture");
    assert_eq!(reopened.message.subject.as_deref(), Some("Test Email"));
    assert!(mf_engine::extract_attachment(&m.message, &[], 99).is_err());
}
