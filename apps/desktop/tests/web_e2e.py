"""E2E de la version web (specs/003-web-version, tâches W008, W009, W014).

Prérequis : `npm run build:web`, `cargo build -p mf-cli`, `pip install playwright && playwright install chromium`.
Lancement : `npm run test:web` (depuis apps/desktop).

Vérifie, dans un vrai navigateur :
- dépôt de fichiers, liste, fichier illisible, doublon ;
- concordance avec `mf show --json` (empreintes, en-têtes, fichiers, URL) pour chaque fichier ;
- bouton / touche H, ouverture d'un message joint ;
- aucune requête réseau après le chargement du moteur, et fonctionnement hors-ligne.
"""

import functools
import http.server
import json
import pathlib
import subprocess
import sys
import threading

from playwright.sync_api import sync_playwright

HERE = pathlib.Path(__file__).resolve().parent
APP = HERE.parent
ROOT = APP.parent.parent
DIST = APP / "dist-web"
CORPUS = ROOT / "tests" / "corpus"
MF = ROOT / "target" / "debug" / "mf"

MAILS = sorted(p for p in CORPUS.glob("*/*") if p.suffix in (".eml", ".msg") and p.parent.name != "malformed")
BROKEN = CORPUS / "malformed" / "truncated.msg"

failures: list[str] = []


def check(cond: bool, what: str) -> None:
    print(("ok    " if cond else "ÉCHEC ") + what)
    if not cond:
        failures.append(what)


def cli(path: pathlib.Path) -> tuple[dict, dict]:
    ev = json.loads(subprocess.check_output([MF, "hash", "--json", path]))[0]
    msg = json.loads(subprocess.check_output([MF, "show", "--json", path]))
    return ev, msg


def serve() -> str:
    class Quiet(http.server.SimpleHTTPRequestHandler):
        def log_message(self, *args: object) -> None:
            pass

    handler = functools.partial(Quiet, directory=str(DIST))
    srv = http.server.ThreadingHTTPServer(("127.0.0.1", 0), handler)
    threading.Thread(target=srv.serve_forever, daemon=True).start()
    return f"http://127.0.0.1:{srv.server_address[1]}/"


def main() -> int:
    if not DIST.joinpath("index.html").exists():
        print("dist-web/ absent : lancer `npm run build:web`")
        return 2
    url = serve()
    expected = {p.name: cli(p) for p in MAILS}

    with sync_playwright() as pw:
        browser = pw.chromium.launch()
        ctx = browser.new_context(viewport={"width": 1440, "height": 920})
        page = ctx.new_page()
        requests: list[str] = []
        page.on("request", lambda r: requests.append(r.url))
        ctx.on("request", lambda r: requests.append(r.url))
        errors: list[str] = []
        page.on("pageerror", lambda e: errors.append(str(e)))

        page.goto(url)
        # Attente par sélecteur : la CSP de la page interdit l'évaluation de code (wait_for_function).
        page.wait_for_selector("html[data-engine]", state="attached")
        check(all(r.startswith(url) for r in requests), f"chargement : uniquement la même origine ({len(requests)} ressources)")
        requests.clear()  # à partir d'ici, plus aucune requête n'est admise (FR-W02)

        # Dépôt via le sélecteur de fichiers (même chemin de code que le glisser-déposer).
        with page.expect_file_chooser() as fc:
            page.click("#open-files")
        fc.value.set_files([str(p) for p in MAILS] + [str(BROKEN)])
        page.wait_for_selector(".message-item.failed")
        items = page.locator(".message-item:not(.failed)")
        check(items.count() == len(MAILS), f"{len(MAILS)} messages listés")
        check("Illisible" in page.locator(".message-item.failed").inner_text(), "fichier OLE tronqué listé en erreur")

        # Concordance avec la CLI pour chaque message.
        for i in range(items.count()):
            items.nth(i).click()
            page.wait_for_selector(".seal-file")
            name = page.locator(".seal-file").inner_text().split(" — ")[0]
            ev, msg = expected[name]
            seal = page.locator(".seal").inner_text()
            check(ev["sha256"] in seal and ev["md5"] in seal, f"{name} : SHA-256 et MD5 identiques à la CLI")
            btn = page.locator(".header-toggle-row button").inner_text()
            check(f"({len(msg['headers'])})" in btn, f"{name} : {len(msg['headers'])} en-têtes")
            files = page.locator(".inventory section").nth(0)
            names = files.locator(".file-name").all_inner_texts()
            check(names == [a["filename"] for a in msg["attachments"]], f"{name} : fichiers contenus {names}")
            hashes = files.locator(".file-hash").all_inner_texts()
            check(all(a["sha256"] in h for a, h in zip(msg["attachments"], hashes)), f"{name} : SHA-256 des fichiers")
            urls = page.locator(".url-text").all_inner_texts()
            check(urls == [u["url"] for u in msg["urls"]], f"{name} : {len(urls)} URL identiques")

        # Bouton des en-têtes (touche H) sur le message de phishing.
        page.locator(".message-item", has_text="Urgent : mot de passe").click()
        page.keyboard.press("h")
        rows = page.locator(".headers-panel li")
        check(rows.count() == len(expected["phishing-basic.eml"][1]["headers"]), "touche H : tous les en-têtes dépliés")
        check(rows.nth(0).inner_text().startswith("1\nReceived:") or "Received:" in rows.nth(0).inner_text(), "premier en-tête = Received")
        page.screenshot(path=str(APP / "test-results" / "web-entetes.png"))

        # Message joint : test_email.msg › PJ 3 (fichier .msg joint).
        page.locator(".message-item", has_text="Test Email").click()
        page.locator("button.open-embedded").nth(1).click()
        check("PJ 3" in page.locator(".breadcrumb").inner_text(), "message joint ouvert avec fil d'Ariane")
        check(page.locator(".inventory .file-name").count() == 2, "message joint : 2 fichiers contenus")
        page.keyboard.press("Escape")
        check(page.locator(".breadcrumb").count() == 0, "Échap revient au message parent")

        # Hors-ligne : un nouveau dépôt fonctionne toujours (doublon détecté).
        ctx.set_offline(True)
        with page.expect_file_chooser() as fc:
            page.click("#open-files")
        fc.value.set_files([str(MAILS[0])])
        page.wait_for_selector("#list-status:has-text('doublon')")
        check(True, "hors-ligne : analyse toujours possible (doublon signalé)")

        page.locator(".message-item", has_text="Votre relevé").click()
        page.wait_for_selector(".body-frame")
        page.wait_for_timeout(500)
        check("1 ressource" in page.locator(".blocked").inner_text(), "pixel de suivi bloqué et compté")
        page.screenshot(path=str(APP / "test-results" / "web-lecture.png"))

        check(not requests, f"aucune requête réseau après le chargement (vu : {requests[:3]})")
        check(not errors, f"aucune erreur JavaScript ({errors[:2]})")
        browser.close()

    print(f"\n{len(failures)} échec(s)")
    return 1 if failures else 0


if __name__ == "__main__":
    (APP / "test-results").mkdir(exist_ok=True)
    sys.exit(main())
