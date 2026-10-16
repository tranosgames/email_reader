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
  /** Faux pour un message Outlook embarqué (objet MAPI) : pas de fichier d'origine. */
  extractable: boolean;
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

export type BlockedKind = "image" | "background" | "stylesheet" | "script" | "frame" | "media" | "object" | "other";

/** Ressource distante bloquée et localisée (FR-039). */
export interface BlockedResource {
  number: number;
  url: string;
  kind: BlockedKind;
  element: string;
  attribute: string;
  line: number | null;
  marker: number | null;
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
  /** HTML brut tel que dans la preuve (affiché comme texte dans la vue « Source HTML »). */
  body_html: string | null;
  body_html_safe: string | null;
  blocked_remote_resources: number;
  blocked_resources: BlockedResource[];
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
/** Clé des octets de chaque preuve dans le worker (même indice que webViews). */
const webKeys: number[] = [];
let worker: Worker | null = null;
let nextJob = 0;
interface WorkerReply {
  ok: boolean;
  view?: MessageView;
  key?: number;
  data?: ArrayBuffer;
  name?: string;
  error?: string;
}
const pending = new Map<number, (r: WorkerReply) => void>();

function ask(job: Record<string, unknown>, transfer: Transferable[] = []): Promise<WorkerReply> {
  const id = nextJob++;
  return new Promise((resolve) => {
    pending.set(id, resolve);
    engine().postMessage({ ...job, id }, transfer);
  });
}

function engine(): Worker {
  if (!worker) {
    worker = createEngineWorker();
    worker.onmessage = (e: MessageEvent) => {
      const d = e.data as WorkerReply & { id?: number };
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

async function analyzeInWorker(name: string, file: File): Promise<WorkerReply> {
  const bytes = await file.arrayBuffer();
  return ask({ kind: "analyze", name, bytes }, [bytes]); // transfert sans copie
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
      void ask({ kind: "release", key: r.key });
    } else {
      webViews.push(view);
      webKeys.push(r.key ?? -1);
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

/** Boîte de confirmation système : `ask` natif (bureau) ou `window.confirm` (web). */
export async function confirmBox(message: string, title: string): Promise<boolean> {
  if (!inTauri) return window.confirm(`${title}\n\n${message}`);
  const { ask: nativeAsk } = await import("@tauri-apps/plugin-dialog");
  return nativeAsk(message, { title, kind: "warning", okLabel: "Télécharger", cancelLabel: "Annuler" });
}

/**
 * FR-038 : enregistre la pièce jointe `index` du message `path` de la preuve `id`.
 * Bureau : dialogue d'enregistrement natif puis écriture par le moteur.
 * Web : octets extraits par le worker, téléchargement via Blob (aucune requête réseau).
 * Retourne le nom enregistré, ou null si l'analyste a annulé le choix d'emplacement.
 */
export async function saveAttachment(id: number, path: number[], att: Attachment): Promise<string | null> {
  if (inTauri) {
    const { save } = await import("@tauri-apps/plugin-dialog");
    const dest = await save({ defaultPath: att.filename.replace(/[\\/]/g, "_"), title: "Enregistrer la pièce jointe" });
    if (!dest) return null;
    await invoke<string>("save_attachment", { id, path, index: att.index, dest });
    return dest;
  }
  const r = await ask({ kind: "extract", key: webKeys[id], path, index: att.index, filename: att.filename });
  if (!r.ok || !r.data) throw new Error(r.error ?? "extraction impossible");
  // application/octet-stream : le navigateur enregistre sans tenter d'ouvrir ni d'afficher.
  const url = URL.createObjectURL(new Blob([r.data], { type: "application/octet-stream" }));
  const link = document.createElement("a");
  link.href = url;
  link.download = r.name ?? "piece-jointe";
  document.body.append(link);
  link.click();
  link.remove();
  setTimeout(() => URL.revokeObjectURL(url), 10_000);
  return link.download;
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
