//! Application de bureau MailForensics : couche IPC mince au-dessus de `mf-engine`
//! (contracts/tauri-commands.md). Aucune logique d'analyse ici (Constitution III).
#![forbid(unsafe_code)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use mf_core::{Evidence, Message};
use mf_engine::Analyzed;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

#[derive(Default)]
struct AppState {
    loaded: Mutex<Vec<Analyzed>>,
}

/// Ligne de la liste latérale (comme msg-reader : expéditeur, objet, date, badges).
#[derive(Serialize)]
struct Summary {
    id: usize,
    file_name: String,
    format: mf_core::Format,
    sha256: String,
    subject: Option<String>,
    from: Option<String>,
    date_utc: Option<String>,
    attachments: usize,
    urls: usize,
    issues: usize,
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum OpenResult {
    Ready { summary: Summary },
    Duplicate { path: String, of: usize },
    Failed { path: String, error: String },
}

#[derive(Serialize)]
struct MessageView<'a> {
    evidence: &'a Evidence,
    message: &'a Message,
}

fn summary(id: usize, a: &Analyzed) -> Summary {
    let m = &a.message;
    Summary {
        id,
        file_name: Path::new(&a.evidence.path).file_name().map_or_else(
            || a.evidence.path.clone(),
            |n| n.to_string_lossy().into_owned(),
        ),
        format: a.evidence.format,
        sha256: a.evidence.sha256.clone(),
        subject: m.subject.clone(),
        from: m
            .from
            .first()
            .and_then(|f| f.display_name.clone().or_else(|| f.address.clone())),
        date_utc: m.date_utc.clone(),
        attachments: m.attachments.len(),
        urls: m.urls.len(),
        issues: m.issues.len(),
    }
}

/// Dossiers : on ne retient que les `.eml` / `.msg` ; fichiers déposés : tous (détection par signature).
fn expand(paths: Vec<String>) -> Vec<PathBuf> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        let Ok(rd) = std::fs::read_dir(dir) else {
            return;
        };
        let mut entries: Vec<_> = rd.filter_map(Result::ok).map(|e| e.path()).collect();
        entries.sort();
        for p in entries {
            if p.is_dir() {
                walk(&p, out);
            } else if p
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("eml") || e.eq_ignore_ascii_case("msg"))
            {
                out.push(p);
            }
        }
    }
    let mut out = Vec::new();
    for p in paths.into_iter().map(PathBuf::from) {
        if p.is_dir() {
            walk(&p, &mut out)
        } else {
            out.push(p)
        }
    }
    out
}

#[tauri::command]
fn open_paths(paths: Vec<String>, state: tauri::State<'_, AppState>) -> Vec<OpenResult> {
    let files = expand(paths);
    let mut results = Vec::with_capacity(files.len());
    for f in files {
        let path = f.display().to_string();
        match mf_engine::open(&f) {
            Ok(a) => {
                let mut loaded = state.loaded.lock().unwrap_or_else(|e| e.into_inner());
                if let Some(of) = loaded
                    .iter()
                    .position(|x| x.evidence.sha256 == a.evidence.sha256)
                {
                    results.push(OpenResult::Duplicate { path, of });
                } else {
                    let id = loaded.len();
                    results.push(OpenResult::Ready {
                        summary: summary(id, &a),
                    });
                    loaded.push(a);
                }
            }
            Err(e) => results.push(OpenResult::Failed {
                path,
                error: e.to_string(),
            }),
        }
    }
    results
}

/// Message racine (`path` vide) ou embarqué (`path` = indices de PJ successifs).
#[tauri::command]
fn get_message(
    id: usize,
    path: Vec<u32>,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let loaded = state.loaded.lock().unwrap_or_else(|e| e.into_inner());
    let a = loaded.get(id).ok_or("preuve inconnue")?;
    let message = mf_engine::descend(&a.message, &path).ok_or("message embarqué introuvable")?;
    serde_json::to_value(MessageView {
        evidence: &a.evidence,
        message,
    })
    .map_err(|e| e.to_string())
}

/// FR-038 : enregistre une pièce jointe à l'emplacement choisi dans le dialogue natif, après
/// confirmation côté interface. L'écriture est faite par le moteur, jamais par le frontend.
#[tauri::command]
fn save_attachment(
    id: usize,
    path: Vec<u32>,
    index: u32,
    dest: String,
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    let loaded = state.loaded.lock().unwrap_or_else(|e| e.into_inner());
    let a = loaded.get(id).ok_or("preuve inconnue")?;
    let x = mf_engine::extract_attachment(&a.message, &path, index).map_err(|e| e.to_string())?;
    let dest = PathBuf::from(dest);
    let dest_dir = dest.parent().and_then(|d| std::fs::canonicalize(d).ok());
    let evidence_dir = std::fs::canonicalize(&a.evidence.path)
        .ok()
        .and_then(|p| p.parent().map(PathBuf::from));
    if dest_dir.is_some() && dest_dir == evidence_dir {
        return Err(
            "Destination refusée : choisissez un autre dossier que celui de la preuve.".into(),
        );
    }
    std::fs::write(&dest, &x.data).map_err(|e| format!("{}: {e}", dest.display()))?;
    Ok(x.sha256)
}

/// Fichiers passés au lancement (association `.eml` / `.msg`, double-clic).
#[tauri::command]
fn initial_paths() -> Vec<String> {
    std::env::args()
        .skip(1)
        .filter(|a| !a.starts_with('-'))
        .collect()
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            open_paths,
            get_message,
            initial_paths,
            save_attachment
        ])
        .run(tauri::generate_context!())
        .unwrap_or_else(|e| {
            eprintln!("MailForensics n'a pas pu démarrer : {e}");
            std::process::exit(1);
        });
}
