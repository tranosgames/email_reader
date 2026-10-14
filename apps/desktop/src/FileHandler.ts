// Entrée des fichiers (rôle de FileHandler.js dans msg-reader) : boutons, glisser-déposer
// plein écran, fichiers passés au lancement.

import { inTauri, initialPaths, openPaths, pickFiles } from "./ipc";
import type { MessageHandler } from "./MessageHandler";

export class FileHandler {
  private overlay = document.getElementById("drop-overlay") as HTMLDivElement;

  constructor(private state: MessageHandler) {}

  async init(): Promise<void> {
    document.getElementById("open-files")?.addEventListener("click", () => this.pick(false));
    document.getElementById("open-folder")?.addEventListener("click", () => this.pick(true));

    if (inTauri) {
      const { getCurrentWebview } = await import("@tauri-apps/api/webview");
      await getCurrentWebview().onDragDropEvent((event) => {
        const p = event.payload;
        if (p.type === "enter" || p.type === "over") this.overlay.hidden = false;
        else if (p.type === "leave") this.overlay.hidden = true;
        else if (p.type === "drop") {
          this.overlay.hidden = true;
          void this.open(p.paths);
        }
      });
    }

    const initial = await initialPaths();
    if (initial.length) await this.open(initial);
  }

  private async pick(directory: boolean): Promise<void> {
    const paths = await pickFiles(directory);
    if (paths.length) await this.open(paths);
  }

  async open(paths: string[]): Promise<void> {
    this.state.lastNotice = "Analyse en cours…";
    this.state.addResults(await openPaths(paths));
  }
}
