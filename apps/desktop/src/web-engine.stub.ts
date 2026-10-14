// Build de bureau : le moteur est appelé via Tauri, jamais via WebAssembly.
export function createEngineWorker(): Worker {
  throw new Error("moteur WebAssembly indisponible dans l'application de bureau");
}
