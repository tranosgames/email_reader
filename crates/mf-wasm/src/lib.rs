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
