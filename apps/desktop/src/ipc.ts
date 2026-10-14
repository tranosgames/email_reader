// Accès au moteur : commandes Tauri (contracts/tauri-commands.md) dans l'application de bureau,
// module WebAssembly dans un Web Worker dans la version web (specs/003-web-version).

export type Format = "eml" | "msg" | "pst" | "unknown";

export interface Address {
  display_name: string | null;
  address: string | null;
  x500_dn: string | null;
}

export interface Header {
  index: number;
  name: string;
  raw: string;
  value: string;
  offset: [number, number] | null;
}

export interface Attachment {
  index: number;
  filename: string;
  declared_mime: string | null;
  size: number;
  sha256: string;
  md5: string;
  disposition: "inline" | "attachment" | "unknown";
  content_id: string | null;
  embedded: Message | null;
}

export type UrlLocation =
  | { kind: "header"; name: string }
  | { kind: "body_text" }
  | { kind: "html_attribute"; attribute: string }
  | { kind: "html_text" }
  | { kind: "html_css" };

export interface UrlEntry {
  url: string;
  defanged: string;
  host: string | null;
  locations: UrlLocation[];
  occurrences: number;
}

export interface Message {
  subject: string | null;
  from: Address[];
  sender: Address | null;
  reply_to: Address[];
  to: Address[];
  cc: Address[];
  bcc: Address[];
  date_raw: string | null;
  date_utc: string | null;
  message_id: string | null;
  headers: Header[];
  body_text: string | null;
  body_html_safe: string | null;
  blocked_remote_resources: number;
  attachments: Attachment[];
  urls: UrlEntry[];
  issues: string[];
}

export interface Evidence {
  path: string;
  size: number;
  sha256: string;
  md5: string;
  format: Format;
}

export interface Summary {
  id: number;
  file_name: string;
  format: Format;
  sha256: string;
  subject: string | null;
  from: string | null;
  date_utc: string | null;
  attachments: number;
  urls: number;
  issues: number;
}

export type OpenResult =
  | { kind: "ready"; summary: Summary }
  | { kind: "duplicate"; path: string; of: number }
  | { kind: "failed"; path: string; error: string };

export interface MessageView {
  evidence: Evidence;
  message: Message;
}

import { createEngineWorker } from "./web-engine";

export const inTauri = "__TAURI_INTERNALS__" in window;

async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<T>(cmd, args);
}

// ——— Backend web (feature 003) : moteur WebAssembly dans un Web Worker ———

/** Fichier fourni par le navigateur, avec son chemin relatif (dossier déposé). */
export interface WebFile {
  path: string;
  file: File;
}

/** FR-W07 : au-delà, l'onglet risque de manquer de mémoire ; l'application de bureau s'impose. */
export const WEB_MAX_SIZE = 500 * 1024 * 1024;

const webViews: MessageView[] = [];
let worker: Worker | null = null;
let nextJob = 0;
const pending = new Map<number, (r: { ok: boolean; view?: MessageView; error?: string }) => void>();

function engine(): Worker {
  if (!worker) {
    worker = createEngineWorker();
    worker.onmessage = (e: MessageEvent) => {
      const d = e.data as { id?: number; ok: boolean; view?: MessageView; error?: string };
      if (d.id === undefined) {
        // Module prêt : plus aucune ressource ne sera demandée au serveur (FR-W02).
        document.documentElement.dataset.engine = (e.data as { version?: string }).version ?? "ready";
        return;
      }
      pending.get(d.id)?.(d);
      pending.delete(d.id);
    };
  }
  return worker;
}

/** Version web : charge le moteur dès l'ouverture de la page, pour que toute requête réseau
 *  ait lieu avant que l'analyste ne dépose une preuve. */
export function warmUp(): void {
  if (!inTauri) engine();
}

async function analyzeInWorker(name: string, file: File): Promise<{ ok: boolean; view?: MessageView; error?: string }> {
  const bytes = await file.arrayBuffer();
  const id = nextJob++;
  return new Promise((resolve) => {
    pending.set(id, resolve);
    engine().postMessage({ id, name, bytes }, [bytes]); // transfert sans copie
  });
}

/** Même résumé que le backend Tauri (src-tauri/src/main.rs › summary). */
function summarize(id: number, v: MessageView): Summary {
  const m = v.message;
  const first = m.from[0];
  return {
    id,
    file_name: v.evidence.path.split("/").pop() ?? v.evidence.path,
    format: v.evidence.format,
    sha256: v.evidence.sha256,
    subject: m.subject,
    from: first ? (first.display_name ?? first.address) : null,
    date_utc: m.date_utc,
    attachments: m.attachments.length,
    urls: m.urls.length,
    issues: m.issues.length,
  };
}

async function openWebFiles(files: WebFile[]): Promise<OpenResult[]> {
  const results: OpenResult[] = [];
  for (const { path, file } of files) {
    if (file.size > WEB_MAX_SIZE) {
      results.push({
        kind: "failed",
        path,
        error: `fichier de ${Math.round(file.size / 1048576)} Mo : au-delà de 500 Mo, utilisez l'application de bureau`,
      });
      continue;
    }
    const r = await analyzeInWorker(path, file);
    if (!r.ok || !r.view) {
      results.push({ kind: "failed", path, error: r.error ?? "erreur inconnue" });
      continue;
    }
    const view = r.view;
    const of = webViews.findIndex((v) => v.evidence.sha256 === view.evidence.sha256);
    if (of >= 0) {
      results.push({ kind: "duplicate", path, of });
    } else {
      webViews.push(view);
      results.push({ kind: "ready", summary: summarize(webViews.length - 1, view) });
    }
  }
  return results;
}

// ——— API commune aux deux cibles ———

export async function openSources(sources: string[] | WebFile[]): Promise<OpenResult[]> {
  if (inTauri) return invoke<OpenResult[]>("open_paths", { paths: sources as string[] });
  return openWebFiles(sources as WebFile[]);
}

export async function getMessage(id: number, path: number[]): Promise<MessageView> {
  if (inTauri) return invoke<MessageView>("get_message", { id, path });
  const view = webViews[id];
  if (!view) throw new Error("preuve inconnue");
  return view; // l'interface descend elle-même dans les messages embarqués
}

export async function initialPaths(): Promise<string[]> {
  return inTauri ? invoke<string[]>("initial_paths") : [];
}

/** Dialogue natif (bureau) ; côté web, FileHandler utilise un <input type="file">. */
export async function pickPaths(directory: boolean): Promise<string[]> {
  const { open } = await import("@tauri-apps/plugin-dialog");
  const picked = await open({
    multiple: true,
    directory,
    filters: directory ? undefined : [{ name: "Courriels", extensions: ["eml", "msg"] }],
  });
  if (!picked) return [];
  return Array.isArray(picked) ? picked : [picked];
}
