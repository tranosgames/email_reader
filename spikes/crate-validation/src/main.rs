// Prototype de recherche (Phase 0) : valide que l'écosystème Rust couvre EML, MSG et PST.
use mail_parser::MimeHeaders;
use sha2::{Digest, Sha256};
use std::io::Read;

fn sha256(b: &[u8]) -> String {
    let h = Sha256::digest(b);
    h.iter().map(|x| format!("{x:02x}")).collect()
}

fn eml(path: &str) {
    let raw = std::fs::read(path).unwrap();
    println!("== EML {path} sha256={}", sha256(&raw));
    let msg = mail_parser::MessageParser::default().parse(&raw).expect("parse");
    println!("  subject: {:?}", msg.subject());
    println!("  from: {:?}", msg.from().and_then(|a| a.first()).and_then(|a| a.address()));
    // En-têtes bruts, dans l'ordre, avec offsets => indispensable pour l'analyse des Received
    for h in msg.headers() {
        if h.name().eq_ignore_ascii_case("Received") {
            let s = &raw[h.offset_start() as usize..h.offset_end() as usize];
            println!("  Received @{}: {}", h.offset_field(), String::from_utf8_lossy(s).trim());
        }
    }
    for a in msg.attachments() {
        println!("  PJ: {:?} {} o sha256={}", a.attachment_name(), a.contents().len(), sha256(a.contents()));
    }
}

fn msg_raw(path: &str) {
    let raw = std::fs::read(path).unwrap();
    println!("== MSG(cfb) {path} sha256={}", sha256(&raw));
    let mut comp = cfb::CompoundFile::open(std::io::Cursor::new(raw)).unwrap();
    let entries: Vec<_> = comp.walk().map(|e| (e.path().to_path_buf(), e.is_stream(), e.len())).collect();
    let mut n = 0;
    for (p, is_stream, len) in &entries {
        let name = p.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
        if *is_stream && name.starts_with("__substg1.0_") && p.parent() == Some(std::path::Path::new("/")) {
            n += 1;
            let tag = &name[12..];
            if tag.ends_with("001F") && (tag.starts_with("0037") || tag.starts_with("007D")) {
                let mut buf = Vec::new();
                comp.open_stream(p).unwrap().read_to_end(&mut buf).unwrap();
                let u16s: Vec<u16> = buf.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
                let s = String::from_utf16_lossy(&u16s);
                println!("  prop 0x{tag} ({len} o): {}", s.lines().next().unwrap_or(""));
            }
        }
    }
    println!("  {n} propriétés variables à la racine, {} entrées OLE", entries.len());
}

fn msg_high(path: &str) {
    let o = msg_parser::Outlook::from_path(path).unwrap();
    println!("== MSG(msg_parser) subject={:?} from={:?} pj={}", o.subject, o.sender.email, o.attachments.len());
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    for a in &args {
        if a.ends_with(".eml") { eml(a) }
        else if a.ends_with(".msg") { msg_raw(a); msg_high(a) }
        else if a.ends_with(".pst") {
            match outlook_pst::open_store(a) { Ok(_) => println!("PST ouvert"), Err(e) => println!("PST: {e}") }
        }
    }
}
