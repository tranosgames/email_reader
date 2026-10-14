// Web Worker de la version web (feature 003, contracts/wasm-api.md) : le moteur Rust compilé en
// WebAssembly analyse les octets hors du fil principal. Rien n'est envoyé sur le réseau.

import init, { analyze, engine_version } from "./wasm/mf_wasm.js";
import wasmUrl from "./wasm/mf_wasm_bg.wasm?url";

interface Job {
  id: number;
  name: string;
  bytes: ArrayBuffer;
}

const ready = init({ module_or_path: wasmUrl }).then(() => {
  self.postMessage({ ready: true, version: engine_version() });
});

self.onmessage = async (e: MessageEvent<Job>) => {
  const { id, name, bytes } = e.data;
  try {
    await ready;
    const view = JSON.parse(analyze(name, new Uint8Array(bytes)));
    self.postMessage({ id, ok: true, view });
  } catch (err) {
    self.postMessage({ id, ok: false, error: err instanceof Error ? err.message : String(err) });
  }
};
