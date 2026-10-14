// Panneau de lecture : scellé de la preuve, enveloppe, bouton des en-têtes (FR-035),
// corps sûr, et colonne d'inventaire « Fichiers contenus » (FR-036) + « URL » (FR-037).

import { h, humanSize, toast } from "../dom";
import type { Address, Attachment, Header, MessageView, UrlEntry, UrlLocation } from "../ipc";

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

let bodyMode: "html" | "text" = "html";

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

  const grid = h("div", { class: "reading-grid" }, main, inventory(m.attachments, m.urls, a));
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

function body(view: MessageView): HTMLElement {
  const m = view.message;
  const wrap = h("section", { "aria-label": "Corps du message" });
  const hasHtml = !!m.body_html_safe;
  const hasText = !!m.body_text;
  const mode = hasHtml && (bodyMode === "html" || !hasText) ? "html" : "text";

  const tabs = h("div", { class: "body-tabs" });
  if (hasHtml && hasText) {
    for (const [k, label] of [["html", "HTML"], ["text", "Texte"]] as const) {
      tabs.append(
        h(
          "button",
          {
            class: "btn btn-small",
            type: "button",
            "aria-pressed": String(mode === k),
            onclick: () => {
              bodyMode = k;
              wrap.replaceWith(body(view));
            },
          },
          label,
        ),
      );
    }
  }
  if (mode === "html" && m.blocked_remote_resources > 0) {
    tabs.append(
      h("span", { class: "blocked" }, `${m.blocked_remote_resources} ressource(s) distante(s) bloquée(s)`),
    );
  }
  wrap.append(tabs);

  if (mode === "html" && m.body_html_safe) {
    // Seconde couche après la sanitisation côté moteur : sandbox sans scripts + CSP stricte.
    const doc =
      `<!doctype html><html><head><meta charset="utf-8">` +
      `<meta http-equiv="Content-Security-Policy" content="default-src 'none'; img-src data:; style-src 'unsafe-inline'">` +
      `<style>body{font:15px/1.45 system-ui,sans-serif;margin:16px;color:#1c2733;background:#fff;overflow-wrap:anywhere}img{max-width:100%;height:auto}a{color:#1f6f78;text-decoration:underline dotted;cursor:default}</style>` +
      `</head><body>${m.body_html_safe}</body></html>`;
    const frame = h("iframe", {
      class: "body-frame",
      sandbox: "allow-same-origin",
      referrerpolicy: "no-referrer",
      title: "Corps HTML neutralisé",
    }) as HTMLIFrameElement;
    frame.srcdoc = doc;
    frame.addEventListener("load", () => {
      const height = frame.contentDocument?.documentElement.scrollHeight;
      if (height) frame.style.height = `${Math.min(height + 8, 4000)}px`;
    });
    wrap.append(frame);
  } else if (hasText) {
    wrap.append(h("pre", { class: "body-text" }, m.body_text ?? ""));
  } else {
    wrap.append(h("p", { class: "none" }, "Aucun corps lisible dans ce message."));
  }
  return wrap;
}

function inventory(files: Attachment[], urls: UrlEntry[], a: ReadingActions): HTMLElement {
  return h(
    "aside",
    { class: "inventory", "aria-label": "Contenu du message" },
    filesSection(files, a),
    urlsSection(urls),
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

function urlsSection(urls: UrlEntry[]): HTMLElement {
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
