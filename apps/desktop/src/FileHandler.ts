// Entrée des fichiers (rôle de FileHandler.js dans msg-reader) : boutons, glisser-déposer
// plein écran, fichiers passés au lancement. Deux cibles :
// - bureau (Tauri) : dialogue natif et chemins absolus ;
// - web : <input type="file"> et glisser-déposer HTML5, dossiers parcourus récursivement.

import { inTauri, initialPaths, openSources, pickPaths, type WebFile } from "./ipc";
import type { MessageHandler } from "./MessageHandler";

const MAIL_EXT = /\.(eml|msg)$/i;

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
      const initial = await initialPaths();
      if (initial.length) await this.open(initial);
    } else {
      this.initWebDrop();
    }
  }

  private async pick(directory: boolean): Promise<void> {
    if (inTauri) {
      const paths = await pickPaths(directory);
      if (paths.length) await this.open(paths);
      return;
    }
    const input = document.createElement("input");
    input.type = "file";
    input.multiple = true;
    if (directory) input.webkitdirectory = true;
    else input.accept = ".eml,.msg,message/rfc822,application/vnd.ms-outlook";
    input.addEventListener("change", () => {
      const files = [...(input.files ?? [])]
        .map((file) => ({ path: file.webkitRelativePath || file.name, file }))
        .filter((f) => !directory || MAIL_EXT.test(f.path));
      if (files.length) void this.open(files);
    });
    input.click();
  }

  private initWebDrop(): void {
    let depth = 0;
    window.addEventListener("dragenter", (e) => {
      e.preventDefault();
      depth++;
      this.overlay.hidden = false;
    });
    window.addEventListener("dragover", (e) => e.preventDefault());
    window.addEventListener("dragleave", () => {
      depth = Math.max(0, depth - 1);
      if (!depth) this.overlay.hidden = true;
    });
    window.addEventListener("drop", (e) => {
      e.preventDefault();
      depth = 0;
      this.overlay.hidden = true;
      const items = [...(e.dataTransfer?.items ?? [])];
      const entries = items.map((i) => i.webkitGetAsEntry()).filter((x): x is FileSystemEntry => !!x);
      void (async () => {
        const files: WebFile[] = [];
        if (entries.length) {
          for (const entry of entries) await collect(entry, files, entry.isFile);
        } else {
          for (const file of e.dataTransfer?.files ?? []) files.push({ path: file.name, file });
        }
        if (files.length) await this.open(files);
      })();
    });
  }

  async open(sources: string[] | WebFile[]): Promise<void> {
    this.state.lastNotice = "Analyse en cours…";
    this.state.addResults(await openSources(sources));
  }
}

/** Parcours récursif d'une entrée déposée ; dans un dossier, seuls les .eml / .msg sont retenus. */
async function collect(entry: FileSystemEntry, out: WebFile[], explicit: boolean): Promise<void> {
  const path = entry.fullPath.replace(/^\//, "");
  if (entry.isFile) {
    if (!explicit && !MAIL_EXT.test(entry.name)) return;
    const file = await new Promise<File>((res, rej) => (entry as FileSystemFileEntry).file(res, rej));
    out.push({ path, file });
    return;
  }
  const reader = (entry as FileSystemDirectoryEntry).createReader();
  // readEntries renvoie les entrées par lots : lire jusqu'à épuisement.
  for (;;) {
    const batch = await new Promise<FileSystemEntry[]>((res, rej) => reader.readEntries(res, rej));
    if (!batch.length) break;
    batch.sort((a, b) => a.name.localeCompare(b.name));
    for (const child of batch) await collect(child, out, false);
  }
}
