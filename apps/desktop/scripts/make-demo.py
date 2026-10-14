"""Génère public/demo/index.json à partir du corpus, pour prévisualiser l'interface dans un
navigateur sans Tauri (`npm run dev` puis http://localhost:5173/?demo). Ne pas livrer."""
import json, pathlib, subprocess, sys

root = pathlib.Path(__file__).resolve().parents[3]
mf = root / "target" / "debug" / "mf"
files = sys.argv[1:] or [str(p) for p in sorted((root / "tests" / "corpus").glob("*/*")) if p.suffix in (".eml", ".msg")]
out = []
for f in files:
    try:
        ev = json.loads(subprocess.check_output([mf, "hash", "--json", f]))[0]
        msg = json.loads(subprocess.check_output([mf, "show", "--json", f], stderr=subprocess.DEVNULL))
    except subprocess.CalledProcessError:
        continue
    first = (msg["from"] or [{}])[0]
    out.append({
        "summary": {"id": len(out), "file_name": pathlib.Path(f).name, "format": ev["format"], "sha256": ev["sha256"],
                    "subject": msg["subject"], "from": first.get("display_name") or first.get("address"),
                    "date_utc": msg["date_utc"], "attachments": len(msg["attachments"]), "urls": len(msg["urls"]),
                    "issues": len(msg["issues"])},
        "views": {"": {"evidence": ev, "message": msg}},
    })
dest = root / "apps" / "desktop" / "public" / "demo"
dest.mkdir(parents=True, exist_ok=True)
(dest / "index.json").write_text(json.dumps(out))
print(f"{len(out)} messages → {dest / 'index.json'}")
