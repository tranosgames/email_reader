// Panneau de lecture : scellé de la preuve, enveloppe, bouton des en-têtes (FR-035),
// corps sûr, et colonne d'inventaire « Fichiers contenus » (FR-036) + « URL » (FR-037).

import { h, humanSize, toast } from "../dom";
import type { Address, Attachment, BlockedKind, BlockedResource, Header, Message, MessageView, UrlEntry, UrlLocation } from "../ipc";

export interface ReadingActions {
  headersOpen: boolean;
  path: number[];
  onToggleHeaders: () => void;
  onOpenEmbedded: (index: number) => void;
  /** FR-038 : téléchargement après confirmation. */
  onDownload: (attachment: Attachment) => void;
  onGoToDepth: (depth: number) => void;
}

/** Fil d'Ariane : noms des PJ traversées, fournis par l'appelant. */
export interface Crumb {
  label: string;
}


export function renderReading(view: MessageView, crumbs: Crumb[], a: ReadingActions): HTMLElement {
  const m = view.message;
  const main = h("div", { class: "reading-main" });

  main.append(seal(view));
  if (crumbs.length > 1) main.append(breadcrumb(crumbs, a));

  main.append(h("h1", { class: "subject-line" }, m.subject?.trim() || "(sans objet)"));
  main.append(envelope(view));
  if (m.issues.length) main.append(h("ul", { class: "issues" }, ...m.issues.map((i) => h("li", {}, i))));

  // FR-035 — bouton « Afficher / Masquer les en-têtes »
  const panelId = "headers-panel";
  main.append(
    h(
      "div",
      { class: "header-toggle-row" },
      h(
        "button",
        {
          class: "btn",
          type: "button",
          "aria-expanded": String(a.headersOpen),
          "aria-controls": panelId,
          onclick: a.onToggleHeaders,
        },
        `${a.headersOpen ? "Masquer" : "Afficher"} les en-têtes (${m.headers.length})`,
      ),
      h("span", { class: "kbd", title: "Raccourci clavier" }, "H"),
    ),
  );
  if (a.headersOpen) main.append(headersPanel(panelId, m.headers));

  main.append(body(view));

  const grid = h("div", { class: "reading-grid" }, main, inventory(m.attachments, m.urls, a, m.blocked_resources));
  return grid;
}

function seal(view: MessageView): HTMLElement {
  const name = view.evidence.path.split(/[\\/]/).pop() ?? view.evidence.path;
  return h(
    "dl",
    { class: "seal", title: view.evidence.path },
    h("div", { class: "seal-file" }, `${name} — ${view.evidence.format.toUpperCase()}, ${humanSize(view.evidence.size)}`),
    h("dt", {}, "SHA-256"),
    h("dd", {}, view.evidence.sha256),
    h("dt", {}, "MD5"),
    h("dd", {}, view.evidence.md5),
  );
}

function breadcrumb(crumbs: Crumb[], a: ReadingActions): HTMLElement {
  const nav = h("nav", { class: "breadcrumb", "aria-label": "Provenance du message" });
  crumbs.forEach((c, depth) => {
    if (depth > 0) nav.append(h("span", { "aria-hidden": "true" }, "›"));
    if (depth < crumbs.length - 1) {
      nav.append(h("button", { type: "button", onclick: () => a.onGoToDepth(depth) }, c.label));
    } else {
      nav.append(h("strong", { "aria-current": "page" }, c.label));
    }
  });
  return nav;
}

function addr(x: Address): HTMLElement {
  const label =
    x.display_name && x.address && x.display_name !== x.address
      ? `${x.display_name} <${x.address}>`
      : (x.address ?? x.display_name ?? "");
  return h("span", { class: "addr" }, label, x.x500_dn ? h("span", { class: "x500" }, `Exchange : ${x.x500_dn}`) : null);
}

function addrList(list: Address[]): HTMLElement {
  const dd = h("dd");
  list.forEach((x, i) => {
    if (i) dd.append(", ");
    dd.append(addr(x));
  });
  return dd;
}

function envelope(view: MessageView): HTMLElement {
  const m = view.message;
  const dl = h("dl", { class: "envelope" });
  const row = (label: string, dd: HTMLElement) => dl.append(h("dt", {}, label), dd);
  row("De", addrList(m.from));
  if (m.sender) row("Envoyé par", addrList([m.sender]));
  if (m.reply_to.length) row("Répondre à", addrList(m.reply_to));
  row("À", addrList(m.to));
  if (m.cc.length) row("Cc", addrList(m.cc));
  if (m.bcc.length) row("Cci", addrList(m.bcc));
  row(
    "Date",
    h(
      "dd",
      {},
      m.date_raw ?? "—",
      m.date_utc ? h("span", { class: "utc" }, ` (UTC ${m.date_utc.replace("T", " ").replace(/\+00:00$/, "")})`) : null,
    ),
  );
  return dl;
}

function headersPanel(id: string, headers: Header[]): HTMLElement {
  if (!headers.length) {
    return h("div", { class: "headers-panel", id }, h("p", { class: "empty" }, "Ce message ne contient aucun en-tête de transport."));
  }
  return h(
    "div",
    { class: "headers-panel", id, role: "region", "aria-label": "En-têtes bruts" },
    h(
      "ol",
      {},
      ...headers.map((hd) => {
        const value = hd.raw.slice(hd.raw.indexOf(":") + 1).replace(/^ /, "");
        return h(
          "li",
          { title: hd.offset ? `octets ${hd.offset[0]}–${hd.offset[1]}` : "" },
          h("span", { class: "hidx" }, String(hd.index + 1)),
          h("span", { class: "hname" }, `${hd.name}:`),
          h("span", {}, value),
        );
      }),
    ),
  );
}

// ——— Corps du message : rendu HTML neutralisé, texte, source HTML (FR-008, FR-039) ———

type BodyMode = "html" | "text" | "source";
let bodyMode: BodyMode = "html";
/** Liste des ressources bloquées dépliée ou non (mémorisé pendant la session). */
let blockedOpen = false;
/** Action à effectuer une fois la vue reconstruite (aller au repère / à la ligne). */
let pendingFocus: { target: "render"; marker: number } | { target: "source"; line: number } | null = null;

const KIND_LABEL: Record<BlockedKind, string> = {
  image: "Image",
  background: "Image de fond",
  stylesheet: "Feuille de style",
  script: "Script",
  frame: "Cadre",
  media: "Média",
  object: "Objet",
  other: "Autre",
};

/** Styles injectés dans l'iframe du rendu : repères des ressources bloquées. */
const FRAME_CSS =
  "body{font:15px/1.45 system-ui,sans-serif;margin:16px;color:#1c2733;background:#fff;overflow-wrap:anywhere}" +
  "img{max-width:100%;height:auto}a{color:#1f6f78;text-decoration:underline dotted;cursor:default}" +
  ".mf-blocked{outline:2px dashed #b88a00;outline-offset:-2px}" +
  ".mf-badge{display:inline-block;font:700 11px/1.6 system-ui,sans-serif;color:#3d2f00;background:#f2c230;" +
  "border-radius:9px;padding:0 7px;margin:2px;vertical-align:middle;white-space:nowrap}" +
  ".mf-flash{outline:3px solid #1f6f78!important;outline-offset:2px;box-shadow:0 0 0 6px rgba(31,111,120,.25)}";

function blockedPanel(m: Message, rebuild: () => void): HTMLElement | null {
  const list = m.blocked_resources;
  if (!list.length) return null;
  const details = h("details", { class: "blocked-panel", open: blockedOpen }) as HTMLDetailsElement;
  details.addEventListener("toggle", () => (blockedOpen = details.open));
  details.append(
    h(
      "summary",
      {},
      `${list.length} ressource${list.length > 1 ? "s" : ""} distante${list.length > 1 ? "s" : ""} bloquée${list.length > 1 ? "s" : ""}`,
      h("span", { class: "summary-hint" }, blockedOpen ? "masquer le détail" : "voir où elles se trouvent"),
    ),
    h(
      "p",
      { class: "blocked-why" },
      "Le message demande à charger ces éléments depuis Internet. Ils ne sont jamais chargés : le serveur distant apprendrait que le message a été ouvert, quand, et depuis quelle adresse IP (pixel de suivi).",
    ),
    h(
      "ol",
      { class: "blocked-list" },
      ...list.map((b) =>
        h(
          "li",
          {},
          h("span", { class: "num", "aria-label": `Ressource ${b.number}` }, String(b.number)),
          h(
            "div",
            { class: "blocked-body" },
            h(
              "div",
              { class: "blocked-what" },
              h("strong", {}, KIND_LABEL[b.kind]),
              h("span", { class: "tagname" }, `<${b.element} ${b.attribute}>`),
              b.line ? h("span", {}, `ligne ${b.line} du HTML`) : null,
              h("span", { class: "muted" }, b.marker !== null ? "repère dans le rendu" : "retiré du rendu"),
            ),
            h("div", { class: "url-text" }, b.url),
            h(
              "div",
              { class: "blocked-actions" },
              b.marker !== null
                ? h(
                    "button",
                    {
                      class: "btn btn-small",
                      type: "button",
                      onclick: () => {
                        pendingFocus = { target: "render", marker: b.marker as number };
                        bodyMode = "html";
                        rebuild();
                      },
                    },
                    "Voir dans le rendu",
                  )
                : null,
              b.line
                ? h(
                    "button",
                    {
                      class: "btn btn-small",
                      type: "button",
                      onclick: () => {
                        pendingFocus = { target: "source", line: b.line as number };
                        bodyMode = "source";
                        rebuild();
                      },
                    },
                    "Voir dans la source",
                  )
                : null,
              h("button", { class: "btn btn-small", type: "button", onclick: () => copy(b.url, "URL") }, "Copier"),
            ),
          ),
        ),
      ),
    ),
  );
  return details;
}

/** Repères visibles dans le rendu, à l'emplacement de chaque ressource bloquée. */
function decorateRender(doc: Document, m: Message): Map<number, Element> {
  const byMarker = new Map<number, BlockedResource[]>();
  for (const b of m.blocked_resources) {
    if (b.marker !== null) byMarker.set(b.marker, [...(byMarker.get(b.marker) ?? []), b]);
  }
  const found = new Map<number, Element>();
  for (const el of doc.querySelectorAll("img[src*='#mf-blocked-'], [background*='#mf-blocked-']")) {
    const v = el.getAttribute("src") ?? el.getAttribute("background") ?? "";
    const k = Number(v.slice(v.indexOf("#mf-blocked-") + 12));
    if (k) found.set(k, el);
  }
  for (const el of doc.querySelectorAll<HTMLElement>("[style*='--mf-blocked']")) {
    const k = Number(el.style.getPropertyValue("--mf-blocked").trim());
    if (k) found.set(k, el);
  }
  for (const [k, el] of found) {
    const items = byMarker.get(k) ?? [];
    if (!items.length) continue;
    el.classList.add("mf-blocked");
    const badge = doc.createElement("span");
    badge.className = "mf-badge";
    badge.textContent = `bloqué ${items.map((b) => b.number).join(", ")}`;
    badge.title = items.map((b) => `${b.number}. ${KIND_LABEL[b.kind]} : ${b.url}`).join("\n");
    const tag = el.tagName.toLowerCase();
    if (tag === "td" || tag === "th") el.prepend(badge);
    else if (tag === "tr") (el.firstElementChild ?? el).prepend(badge);
    else el.before(badge);
  }
  return found;
}

function flash(el: Element): void {
  el.scrollIntoView({ block: "center", behavior: "smooth" });
  el.classList.add("mf-flash");
  setTimeout(() => el.classList.remove("mf-flash"), 2200);
}

function sourceView(m: Message): HTMLElement {
  const lines = (m.body_html ?? "").split(/\r?\n/);
  const atLine = new Map<number, number[]>();
  for (const b of m.blocked_resources) if (b.line) atLine.set(b.line, [...(atLine.get(b.line) ?? []), b.number]);
  return h(
    "ol",
    { class: "html-source", "aria-label": "Code source HTML du message" },
    ...lines.map((text, i) => {
      const nums = atLine.get(i + 1);
      return h(
        "li",
        { "data-line": String(i + 1), class: nums ? "has-blocked" : undefined },
        h("span", { class: "ln", "aria-hidden": "true" }, String(i + 1)),
        h("code", {}, text || " "),
        nums ? h("span", { class: "num" }, nums.join(", ")) : null,
      );
    }),
  );
}

function body(view: MessageView): HTMLElement {
  const m = view.message;
  const wrap = h("section", { "aria-label": "Corps du message" });
  const rebuild = () => wrap.replaceWith(body(view));
  const hasHtml = !!m.body_html_safe;
  const hasText = !!m.body_text;
  let mode: BodyMode = bodyMode;
  if (!hasHtml && mode !== "text") mode = "text";
  if (mode === "text" && !hasText && hasHtml) mode = "html";

  const tabs = h("div", { class: "body-tabs" });
  const choices: [BodyMode, string, boolean][] = [
    ["html", "HTML", hasHtml],
    ["text", "Texte", hasText],
    ["source", "Source HTML", hasHtml],
  ];
  if (choices.filter((c) => c[2]).length > 1) {
    for (const [k, label, ok] of choices) {
      if (!ok) continue;
      tabs.append(
        h(
          "button",
          {
            class: "btn btn-small",
            type: "button",
            "aria-pressed": String(mode === k),
            onclick: () => {
              bodyMode = k;
              rebuild();
            },
          },
          label,
        ),
      );
    }
  }
  wrap.append(tabs);
  if (mode !== "text") {
    const panel = blockedPanel(m, rebuild);
    if (panel) wrap.append(panel);
  }

  if (mode === "html" && m.body_html_safe) {
    // Seconde couche après la sanitisation côté moteur : sandbox sans scripts + CSP stricte.
    const doc =
      `<!doctype html><html><head><meta charset="utf-8">` +
      `<meta http-equiv="Content-Security-Policy" content="default-src 'none'; img-src data:; style-src 'unsafe-inline'">` +
      `<style>${FRAME_CSS}</style></head><body>${m.body_html_safe}</body></html>`;
    const frame = h("iframe", {
      class: "body-frame",
      sandbox: "allow-same-origin",
      referrerpolicy: "no-referrer",
      title: "Corps HTML neutralisé",
    }) as HTMLIFrameElement;
    frame.srcdoc = doc;
    frame.addEventListener("load", () => {
      const d = frame.contentDocument;
      if (!d) return;
      const markers = decorateRender(d, m);
      frame.style.height = `${Math.min(d.documentElement.scrollHeight + 8, 4000)}px`;
      const focus = pendingFocus;
      if (focus?.target === "render") {
        pendingFocus = null;
        const el = markers.get(focus.marker);
        if (el) setTimeout(() => flash(el), 50);
      }
    });
    wrap.append(frame);
  } else if (mode === "source") {
    const src = sourceView(m);
    wrap.append(src);
    const focus = pendingFocus;
    if (focus?.target === "source") {
      pendingFocus = null;
      requestAnimationFrame(() => {
        const li = src.querySelector(`li[data-line="${focus.line}"]`);
        if (li) flash(li);
      });
    }
  } else if (hasText) {
    wrap.append(h("pre", { class: "body-text" }, m.body_text ?? ""));
  } else {
    wrap.append(h("p", { class: "none" }, "Aucun corps lisible dans ce message."));
  }
  return wrap;
}

function inventory(files: Attachment[], urls: UrlEntry[], a: ReadingActions, blocked: BlockedResource[]): HTMLElement {
  return h(
    "aside",
    { class: "inventory", "aria-label": "Contenu du message" },
    filesSection(files, a),
    urlsSection(urls, blocked),
  );
}

function filesSection(files: Attachment[], a: ReadingActions): HTMLElement {
  const sec = h("section", {}, h("h2", {}, "Fichiers contenus", h("span", { class: "count" }, String(files.length))));
  if (!files.length) {
    sec.append(h("p", { class: "none" }, "Aucun fichier"));
    return sec;
  }
  sec.append(
    h(
      "ul",
      {},
      ...files.map((f) =>
        h(
          "li",
          {},
          h("div", { class: "file-name" }, f.filename),
          h(
            "div",
            { class: "file-meta" },
            h("span", {}, f.declared_mime ?? "type non déclaré"),
            h("span", {}, humanSize(f.size)),
            f.disposition === "inline" ? h("span", { class: "tag-inline" }, "inline") : h("span", {}, "attaché"),
          ),
          // Les deux empreintes en clair (FR-036) : le MD5 n'est plus caché dans une infobulle.
          h("dl", { class: "file-hash" }, h("dt", {}, "SHA-256"), h("dd", {}, f.sha256), h("dt", {}, "MD5"), h("dd", {}, f.md5)),
          h(
            "div",
            { class: "file-actions" },
            f.extractable
              ? h(
                  "button",
                  { class: "btn btn-small download", type: "button", onclick: () => a.onDownload(f) },
                  "Télécharger",
                )
              : h(
                  "button",
                  {
                    class: "btn btn-small download",
                    type: "button",
                    disabled: true,
                    title: "Message Outlook embarqué : il n'existe pas de fichier d'origine à extraire. Ouvrez-le pour consulter son contenu.",
                  },
                  "Téléchargement indisponible",
                ),
            f.embedded
              ? h(
                  "button",
                  { class: "btn btn-small open-embedded", type: "button", onclick: () => a.onOpenEmbedded(f.index) },
                  `Ouvrir le message joint (${f.embedded.attachments.length} fichier(s), ${f.embedded.urls.length} URL)`,
                )
              : null,
          ),
        ),
      ),
    ),
  );
  return sec;
}

function locationLabel(l: UrlLocation): string {
  switch (l.kind) {
    case "header":
      return `en-tête ${l.name}`;
    case "body_text":
      return "texte";
    case "html_attribute":
      return `HTML ${l.attribute}`;
    case "html_text":
      return "texte HTML";
    case "html_css":
      return "CSS";
  }
}

async function copy(text: string, what: string): Promise<void> {
  try {
    await navigator.clipboard.writeText(text);
    toast(`${what} copiée`);
  } catch {
    toast("Copie impossible : sélectionnez l'URL puis Ctrl+C");
  }
}

function urlsSection(urls: UrlEntry[], blocked: BlockedResource[]): HTMLElement {
  const sec = h("section", {}, h("h2", {}, "URL", h("span", { class: "count" }, String(urls.length))));
  if (!urls.length) {
    sec.append(h("p", { class: "none" }, "Aucune URL"));
    return sec;
  }
  sec.append(
    h(
      "ul",
      {},
      ...urls.map((u) =>
        h(
          "li",
          {},
          // Jamais un lien : texte sélectionnable uniquement.
          h("div", { class: "url-text" }, u.url),
          h(
            "div",
            { class: "url-meta" },
            h("span", {}, u.locations.map(locationLabel).join(", ")),
            u.occurrences > 1 ? h("span", {}, `${u.occurrences} fois`) : null,
            ((nums) =>
              nums.length
                ? h("span", { class: "tag-blocked", title: "Ressource que le message chargerait automatiquement : bloquée" }, `chargement bloqué (${nums.join(", ")})`)
                : null)(blocked.filter((b) => b.url === u.url).map((b) => b.number)),
            h(
              "span",
              { class: "actions" },
              h("button", { class: "btn btn-small", type: "button", onclick: () => copy(u.url, "URL") }, "Copier"),
              h(
                "button",
                { class: "btn btn-small", type: "button", title: u.defanged, onclick: () => copy(u.defanged, "URL défangée") },
                "Copier défangée",
              ),
            ),
          ),
        ),
      ),
    ),
  );
  return sec;
}
