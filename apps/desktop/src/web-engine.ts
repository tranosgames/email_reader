// Version web : crée le Web Worker qui embarque le moteur WebAssembly.
// Dans le build de bureau, ce module est remplacé par web-engine.stub.ts (vite.config.ts),
// pour que l'application Tauri ne dépende pas du module .wasm.
export function createEngineWorker(): Worker {
  return new Worker(new URL("./engine.worker.ts", import.meta.url), { type: "module" });
}
