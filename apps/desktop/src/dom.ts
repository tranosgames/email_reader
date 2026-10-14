// Construction de DOM sans innerHTML : tout contenu issu d'une preuve est inséré comme texte.

type Child = Node | string | null | undefined | false;
type Attrs = Record<string, string | boolean | ((e: Event) => void) | undefined>;

export function h(tag: string, attrs: Attrs = {}, ...children: Child[]): HTMLElement {
  const el = document.createElement(tag);
  for (const [k, v] of Object.entries(attrs)) {
    if (v === undefined || v === false) continue;
    if (typeof v === "function") el.addEventListener(k.replace(/^on/, ""), v);
    else if (v === true) el.setAttribute(k, "");
    else el.setAttribute(k, v);
  }
  for (const c of children) {
    if (c === null || c === undefined || c === false) continue;
    el.append(typeof c === "string" ? document.createTextNode(c) : c);
  }
  return el;
}

export function humanSize(n: number): string {
  if (n >= 1 << 20) return `${(n / (1 << 20)).toFixed(1).replace(".", ",")} Mo`;
  if (n >= 1 << 10) return `${(n / 1024).toFixed(1).replace(".", ",")} Ko`;
  return `${n} o`;
}

export function formatDate(utc: string | null): string {
  if (!utc) return "";
  const d = new Date(utc);
  if (Number.isNaN(d.getTime())) return utc;
  return d.toLocaleString("fr-FR", { dateStyle: "short", timeStyle: "short" });
}

export function toast(text: string): void {
  const t = h("div", { class: "toast", role: "status" }, text);
  document.body.append(t);
  setTimeout(() => t.remove(), 1800);
}
