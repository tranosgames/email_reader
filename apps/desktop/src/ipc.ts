// Wrappers typés des commandes Tauri (contracts/tauri-commands.md).
// Hors Tauri (navigateur, `?demo`), un jeu de démonstration statique remplace le moteur.

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

export const inTauri = "__TAURI_INTERNALS__" in window;

async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<T>(cmd, args);
}

// ——— Démonstration hors Tauri ———
interface DemoEntry {
  summary: Summary;
  views: Record<string, MessageView>;
}
let demo: DemoEntry[] | null = null;
async function loadDemo(): Promise<DemoEntry[]> {
  if (!demo) demo = (await (await fetch("/demo/index.json")).json()) as DemoEntry[];
  return demo;
}

export async function openPaths(paths: string[]): Promise<OpenResult[]> {
  if (inTauri) return invoke<OpenResult[]>("open_paths", { paths });
  return (await loadDemo()).map((d) => ({ kind: "ready", summary: d.summary }) as OpenResult);
}

export async function getMessage(id: number, path: number[]): Promise<MessageView> {
  if (inTauri) return invoke<MessageView>("get_message", { id, path });
  const entry = (await loadDemo()).find((d) => d.summary.id === id);
  const view = entry?.views[path.join("/")];
  if (!view) throw new Error("message introuvable");
  return view;
}

export async function initialPaths(): Promise<string[]> {
  if (inTauri) return invoke<string[]>("initial_paths");
  return new URLSearchParams(location.search).has("demo") ? ["demo"] : [];
}

export async function pickFiles(directory: boolean): Promise<string[]> {
  if (!inTauri) return ["demo"];
  const { open } = await import("@tauri-apps/plugin-dialog");
  const picked = await open({
    multiple: true,
    directory,
    filters: directory ? undefined : [{ name: "Courriels", extensions: ["eml", "msg"] }],
  });
  if (!picked) return [];
  return Array.isArray(picked) ? picked : [picked];
}
