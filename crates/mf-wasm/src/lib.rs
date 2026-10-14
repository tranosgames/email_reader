//! Liaison WebAssembly du moteur (contracts/wasm-api.md de la feature 003).
//! Exactement le même code que la CLI et l'application de bureau : seule l'origine des octets change.
#![forbid(unsafe_code)]

use wasm_bindgen::prelude::*;

/// Analyse un fichier `.eml` / `.msg` déjà lu par le navigateur.
/// Retourne le JSON `{ evidence, message }` (mêmes types que `mf show --json`).
#[wasm_bindgen]
pub fn analyze(file_name: &str, bytes: Vec<u8>) -> Result<String, JsError> {
    let analyzed =
        mf_engine::open_bytes(file_name, bytes).map_err(|e| JsError::new(&e.to_string()))?;
    serde_json::to_string(&analyzed).map_err(|e| JsError::new(&e.to_string()))
}

/// Version du moteur embarqué.
#[wasm_bindgen]
pub fn engine_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Octets de la pièce jointe `index` du message désigné par `path` (FR-038), après
/// revérification de l'empreinte. Le navigateur ne fournit que les octets de la preuve :
/// le message est ré-analysé, ce qui garantit les mêmes indices que l'affichage.
#[wasm_bindgen]
pub fn extract_attachment(
    file_name: &str,
    bytes: Vec<u8>,
    path: Vec<u32>,
    index: u32,
) -> Result<Vec<u8>, JsError> {
    let analyzed =
        mf_engine::open_bytes(file_name, bytes).map_err(|e| JsError::new(&e.to_string()))?;
    mf_engine::extract_attachment(&analyzed.message, &path, index)
        .map(|x| x.data)
        .map_err(|e| JsError::new(&e.to_string()))
}

/// Nom de fichier assaini proposé à l'enregistrement (même règle que le bureau et la CLI).
#[wasm_bindgen]
pub fn safe_file_name(name: &str) -> String {
    mf_engine::safe_file_name(name)
}
