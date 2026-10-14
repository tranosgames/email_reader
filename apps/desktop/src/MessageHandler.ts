// État de la session (même rôle que MessageHandler.js de msg-reader) :
// liste des preuves, sélection, épingles, message embarqué courant, en-têtes dépliés.

import type { OpenResult, Summary } from "./ipc";

export interface FailedEntry {
  path: string;
  error: string;
}

type Listener = () => void;

export class MessageHandler {
  readonly items: Summary[] = [];
  readonly failed: FailedEntry[] = [];
  readonly pinned = new Set<number>();
  selectedId: number | null = null;
  /** Indices de PJ menant au message embarqué affiché ([] = message racine). */
  path: number[] = [];
  /** FR-035 : replié par défaut, mémorisé pour toute la session. */
  headersOpen = false;
  lastNotice = "";
  private listeners: Listener[] = [];

  onChange(fn: Listener): void {
    this.listeners.push(fn);
  }

  private emit(): void {
    for (const fn of this.listeners) fn();
  }

  addResults(results: OpenResult[]): void {
    let added = 0;
    let dup = 0;
    for (const r of results) {
      if (r.kind === "ready") {
        this.items.push(r.summary);
        added++;
      } else if (r.kind === "duplicate") dup++;
      else this.failed.push({ path: r.path, error: r.error });
    }
    const failed = results.filter((r) => r.kind === "failed").length;
    const parts = [`${added} message(s) ajouté(s)`];
    if (dup) parts.push(`${dup} doublon(s) ignoré(s)`);
    if (failed) parts.push(`${failed} fichier(s) illisible(s)`);
    this.lastNotice = parts.join(", ");
    if (this.selectedId === null && this.items.length) this.select(this.sorted()[0].id);
    else this.emit();
  }

  /** Épinglés d'abord, puis du plus récent au plus ancien. */
  sorted(): Summary[] {
    return [...this.items].sort((a, b) => {
      const pa = this.pinned.has(a.id) ? 0 : 1;
      const pb = this.pinned.has(b.id) ? 0 : 1;
      if (pa !== pb) return pa - pb;
      return (b.date_utc ?? "").localeCompare(a.date_utc ?? "");
    });
  }

  select(id: number): void {
    this.selectedId = id;
    this.path = [];
    this.emit();
  }

  move(delta: number): void {
    const list = this.sorted();
    if (!list.length) return;
    const i = list.findIndex((s) => s.id === this.selectedId);
    const next = list[Math.min(list.length - 1, Math.max(0, i + delta))];
    this.select(next.id);
  }

  togglePin(id: number | null = this.selectedId): void {
    if (id === null) return;
    if (this.pinned.has(id)) this.pinned.delete(id);
    else this.pinned.add(id);
    this.emit();
  }

  toggleHeaders(): void {
    this.headersOpen = !this.headersOpen;
    this.emit();
  }

  openEmbedded(attachmentIndex: number): void {
    this.path = [...this.path, attachmentIndex];
    this.emit();
  }

  goToDepth(depth: number): void {
    this.path = this.path.slice(0, depth);
    this.emit();
  }
}
