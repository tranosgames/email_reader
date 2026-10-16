//! `mf` — CLI MailForensics (contracts/cli.md). Couche mince au-dessus de `mf-engine`.
#![forbid(unsafe_code)]

use clap::{Parser, Subcommand};
use mf_core::{Address, Message};
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser)]
#[command(
    name = "mf",
    version,
    about = "Analyse forensic de courriels EML/MSG (lecture seule, hors-ligne)"
)]
struct Cli {
    /// Sortie JSON (un document par commande).
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// SHA-256, MD5 et format détecté.
    Hash { files: Vec<PathBuf> },
    /// Résumé du message : en-tête lisible, fichiers contenus, URL (et en-têtes avec --headers).
    Show {
        file: PathBuf,
        /// Affiche aussi tous les en-têtes bruts.
        #[arg(long)]
        headers: bool,
        /// Message embarqué à afficher, ex. « 2/1 » = PJ 2 puis PJ 1.
        #[arg(long)]
        path: Option<String>,
    },
    /// Tous les en-têtes dans l'ordre d'origine.
    Headers {
        file: PathBuf,
        #[arg(long)]
        path: Option<String>,
    },
    /// Fichiers contenus (arborescence des messages embarqués).
    Attachments { file: PathBuf },
    /// Enregistre une pièce jointe (octets d'origine, nom assaini) dans un dossier.
    Extract {
        file: PathBuf,
        /// Numéro de la pièce jointe (colonne « n. » de `mf attachments`).
        #[arg(long)]
        index: u32,
        /// Message embarqué contenant la PJ, ex. « 1 ».
        #[arg(long)]
        path: Option<String>,
        /// Dossier de destination (ne doit pas être celui de la preuve).
        #[arg(long)]
        out: PathBuf,
    },
    /// URL uniques du message, avec emplacements.
    Urls {
        file: PathBuf,
        #[arg(long)]
        path: Option<String>,
        /// Affiche la forme non cliquable (hxxp[://]…).
        #[arg(long)]
        defang: bool,
    },
}

fn parse_path(p: &Option<String>) -> Result<Vec<u32>, String> {
    p.as_deref()
        .map(|s| {
            s.split('/')
                .filter(|x| !x.is_empty())
                .map(|x| x.parse::<u32>().map_err(|e| format!("--path : {e}")))
                .collect()
        })
        .unwrap_or(Ok(Vec::new()))
}

fn fmt_addr(a: &Address) -> String {
    match (&a.display_name, &a.address) {
        (Some(n), Some(e)) if n != e => format!("{n} <{e}>"),
        (_, Some(e)) => e.clone(),
        (Some(n), None) => a
            .x500_dn
            .as_ref()
            .map_or_else(|| n.clone(), |dn| format!("{n} [EX {dn}]")),
        (None, None) => a.x500_dn.clone().unwrap_or_default(),
    }
}

fn fmt_list(v: &[Address]) -> String {
    v.iter().map(fmt_addr).collect::<Vec<_>>().join(", ")
}

fn human(n: u64) -> String {
    match n {
        n if n >= 1 << 20 => format!("{:.1} Mo", n as f64 / f64::from(1 << 20)),
        n if n >= 1 << 10 => format!("{:.1} Ko", n as f64 / 1024.0),
        n => format!("{n} o"),
    }
}

fn print_headers(m: &Message) {
    println!("En-têtes ({})", m.headers.len());
    for h in &m.headers {
        println!("  {}", h.raw.replace('\n', "\n  "));
    }
}

fn print_attachments(m: &Message, indent: usize) {
    let pad = "  ".repeat(indent);
    if m.attachments.is_empty() && indent == 1 {
        println!("{pad}Aucun fichier");
    }
    for a in &m.attachments {
        println!(
            "{pad}{:>2}. {}  [{}] {}  {:?}  sha256 {}",
            a.index,
            a.filename,
            a.declared_mime.as_deref().unwrap_or("?"),
            human(a.size),
            a.disposition,
            a.sha256
        );
        if let Some(inner) = &a.embedded {
            println!(
                "{pad}    ↳ message embarqué : « {} » ({} fichier(s), {} URL)",
                inner.subject.as_deref().unwrap_or(""),
                inner.attachments.len(),
                inner.urls.len()
            );
            print_attachments(inner, indent + 2);
        }
    }
}

fn kind_label(k: mf_core::BlockedKind) -> &'static str {
    use mf_core::BlockedKind::*;
    match k {
        Image => "image",
        Background => "image de fond",
        Stylesheet => "feuille de style",
        Script => "script",
        Frame => "cadre",
        Media => "média",
        Object => "objet",
        Other => "autre",
    }
}

/// FR-039 : ressources que le HTML demandait à charger, bloquées et localisées.
fn print_blocked(m: &Message) {
    if m.blocked_resources.is_empty() {
        return;
    }
    println!(
        "Ressources distantes bloquées ({}) — jamais chargées",
        m.blocked_resources.len()
    );
    for b in &m.blocked_resources {
        println!(
            "  {:>2}. {:<16} <{} {}>  {}  {}",
            b.number,
            kind_label(b.kind),
            b.element,
            b.attribute,
            b.line
                .map_or_else(|| "ligne ?".to_string(), |l| format!("ligne {l} du HTML")),
            if b.marker.is_some() {
                "(visible au rendu)"
            } else {
                "(retiré du rendu)"
            },
        );
        println!("      {}", b.url);
    }
}

fn print_urls(m: &Message, defang: bool) {
    println!("URL ({})", m.urls.len());
    if m.urls.is_empty() {
        println!("  Aucune URL");
    }
    for u in &m.urls {
        let locs = u
            .locations
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(", ");
        println!(
            "  {}  ({locs}{})",
            if defang { &u.defanged } else { &u.url },
            if u.occurrences > 1 {
                format!(", ×{}", u.occurrences)
            } else {
                String::new()
            }
        );
    }
}

fn run(cli: Cli) -> Result<(), String> {
    let out = |v: &dyn erased::Json| -> Result<(), String> {
        println!("{}", v.to_json().map_err(|e| e.to_string())?);
        Ok(())
    };
    match cli.cmd {
        Cmd::Hash { files } => {
            let mut all = Vec::new();
            for f in files {
                let (ev, _) = mf_core::read_evidence(&f).map_err(|e| e.to_string())?;
                if !cli.json {
                    println!(
                        "{}  {:?}  {}\n  sha256 {}\n  md5    {}",
                        ev.path,
                        ev.format,
                        human(ev.size),
                        ev.sha256,
                        ev.md5
                    );
                }
                all.push(ev);
            }
            if cli.json {
                out(&all)?;
            }
        }
        Cmd::Show {
            file,
            headers,
            path,
        } => {
            let a = mf_engine::open(&file).map_err(|e| e.to_string())?;
            let m = mf_engine::descend(&a.message, &parse_path(&path)?)
                .ok_or("--path : message embarqué introuvable")?;
            if cli.json {
                return out(m);
            }
            println!(
                "Preuve   {} ({:?}, {})\n  sha256 {}\n  md5    {}",
                a.evidence.path,
                a.evidence.format,
                human(a.evidence.size),
                a.evidence.sha256,
                a.evidence.md5
            );
            println!("De       {}", fmt_list(&m.from));
            if let Some(s) = &m.sender {
                println!("Envoyé par {}", fmt_addr(s));
            }
            if !m.reply_to.is_empty() {
                println!("Répondre à {}", fmt_list(&m.reply_to));
            }
            println!("À        {}", fmt_list(&m.to));
            if !m.cc.is_empty() {
                println!("Cc       {}", fmt_list(&m.cc));
            }
            if !m.bcc.is_empty() {
                println!("Cci      {}", fmt_list(&m.bcc));
            }
            println!("Objet    {}", m.subject.as_deref().unwrap_or(""));
            println!(
                "Date     {}  (UTC {})",
                m.date_raw.as_deref().unwrap_or("—"),
                m.date_utc.as_deref().unwrap_or("—")
            );
            for i in &m.issues {
                println!("⚠ {i}");
            }
            if headers {
                print_headers(m);
            } else {
                println!(
                    "En-têtes ({}) — ajouter --headers pour les afficher",
                    m.headers.len()
                );
            }
            println!("Fichiers contenus ({})", m.attachments.len());
            print_attachments(m, 1);
            print_urls(m, false);
            print_blocked(m);
        }
        Cmd::Headers { file, path } => {
            let a = mf_engine::open(&file).map_err(|e| e.to_string())?;
            let m = mf_engine::descend(&a.message, &parse_path(&path)?)
                .ok_or("--path : message embarqué introuvable")?;
            if cli.json {
                return out(&m.headers);
            }
            print_headers(m);
        }
        Cmd::Attachments { file } => {
            let a = mf_engine::open(&file).map_err(|e| e.to_string())?;
            if cli.json {
                return out(&a.message.attachments);
            }
            println!("Fichiers contenus ({})", a.message.attachments.len());
            print_attachments(&a.message, 1);
        }
        Cmd::Extract {
            file,
            index,
            path,
            out: dir,
        } => {
            let evidence_dir = std::fs::canonicalize(&file)
                .ok()
                .and_then(|p| p.parent().map(PathBuf::from));
            std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
            if evidence_dir.is_some() && std::fs::canonicalize(&dir).ok() == evidence_dir {
                return Err(
                    "destination refusée : ne pas écrire dans le dossier de la preuve".into(),
                );
            }
            let a = mf_engine::open(&file).map_err(|e| e.to_string())?;
            let x = mf_engine::extract_attachment(&a.message, &parse_path(&path)?, index)
                .map_err(|e| e.to_string())?;
            let dest = dir.join(&x.file_name);
            if dest.exists() {
                return Err(format!("{} existe déjà", dest.display()));
            }
            std::fs::write(&dest, &x.data).map_err(|e| format!("{}: {e}", dest.display()))?;
            if cli.json {
                return out(
                    &serde_json::json!({ "written": dest.display().to_string(), "sha256": x.sha256, "size": x.data.len() }),
                );
            }
            println!(
                "{}  ({}, sha256 {})",
                dest.display(),
                human(x.data.len() as u64),
                x.sha256
            );
        }
        Cmd::Urls { file, path, defang } => {
            let a = mf_engine::open(&file).map_err(|e| e.to_string())?;
            let m = mf_engine::descend(&a.message, &parse_path(&path)?)
                .ok_or("--path : message embarqué introuvable")?;
            if cli.json {
                return out(&m.urls);
            }
            print_urls(m, defang);
        }
    }
    Ok(())
}

/// Petit adaptateur pour sérialiser des valeurs hétérogènes.
mod erased {
    pub trait Json {
        fn to_json(&self) -> serde_json::Result<String>;
    }
    impl<T: serde::Serialize + ?Sized> Json for T {
        fn to_json(&self) -> serde_json::Result<String> {
            serde_json::to_string_pretty(self)
        }
    }
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("mf: {e}");
            ExitCode::from(2)
        }
    }
}
