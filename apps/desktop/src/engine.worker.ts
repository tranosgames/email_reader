// Web Worker de la version web (feature 003, contracts/wasm-api.md) : le moteur Rust compilé en
// WebAssembly analyse les octets hors du fil principal. Rien n'est envoyé sur le réseau.
// Le worker conserve les octets de chaque preuve pour l'extraction des pièces jointes (FR-038).

import init, { analyze, engine_version, extract_attachment, safe_file_name } from "./wasm/mf_wasm.js";
import wasmUrl from "./wasm/mf_wasm_bg.wasm?url";

type Job =
  | { id: number; kind: "analyze"; name: string; bytes: ArrayBuffer }
  | { id: number; kind: "extract"; key: number; path: number[]; index: number; filename: string }
  | { id: number; kind: "release"; key: number };

/** `self` d'un worker dédié (le tsconfig cible le DOM, pas WebWorker). */
const scope = self as unknown as { postMessage(message: unknown, transfer?: Transferable[]): void };

const evidence = new Map<number, { name: string; bytes: Uint8Array }>();

const ready = init({ module_or_path: wasmUrl }).then(() => {
  scope.postMessage({ ready: true, version: engine_version() });
});

self.onmessage = async (e: MessageEvent<Job>) => {
  const job = e.data;
  try {
    await ready;
    if (job.kind === "analyze") {
      const bytes = new Uint8Array(job.bytes);
      const view = JSON.parse(analyze(job.name, bytes));
      evidence.set(job.id, { name: job.name, bytes });
      scope.postMessage({ id: job.id, ok: true, view, key: job.id });
    } else if (job.kind === "extract") {
      const ev = evidence.get(job.key);
      if (!ev) throw new Error("preuve inconnue");
      const data = extract_attachment(ev.name, ev.bytes, new Uint32Array(job.path), job.index);
      const buf = data.buffer as ArrayBuffer;
      scope.postMessage({ id: job.id, ok: true, data: buf, name: safe_file_name(job.filename) }, [buf]);
    } else {
      evidence.delete(job.key);
      scope.postMessage({ id: job.id, ok: true });
    }
  } catch (err) {
    scope.postMessage({ id: job.id, ok: false, error: err instanceof Error ? err.message : String(err) });
  }
};
