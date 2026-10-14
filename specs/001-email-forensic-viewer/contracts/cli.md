# Contrat CLI — `mf`

Binaire `mf` (crate `mf-cli`). Expose toutes les capacités du moteur (FR-032). Conventions :

- Entrées : chemins de fichiers ou dossiers (`-r` récursif). Jamais d'écriture à côté des preuves.
- Sortie : texte lisible par défaut ; `--json` → un document JSON par commande sur stdout
  (schéma : `report.schema.json` pour `report`, objets du data-model sinon).
- Erreurs : stderr ; codes de sortie : `0` succès, `1` erreur d'usage, `2` au moins une preuve en
  erreur de parsing, `3` divergence d'empreinte, `4` erreur d'E/S sur la destination d'export.
- Aucun accès réseau, quelle que soit l'option.

## Commandes

| Commande | Description | FR |
|---|---|---|
| `mf hash <preuves…>` | SHA-256 + MD5 + format détecté | FR-002, FR-003 |
| `mf show <preuve> [--path 2/1]` | En-tête lisible + corps texte (`--path` = provenance dans les messages embarqués) | FR-007, FR-018 |
| `mf headers <preuve> [--raw]` | En-têtes ordonnés, chaîne `Received`, résultats d'auth, DKIM | FR-011–FR-014 |
| `mf findings <preuves…> [--min-severity medium]` | Constats | FR-015, FR-017, FR-022 |
| `mf attachments <preuve> [--tree]` | PJ, types, empreintes, alertes, arborescence d'imbrication | FR-016–FR-018 |
| `mf extract <preuve> --out <dossier> [--index 1,3] [--neutralize]` | Export PJ + `manifest.json` | FR-019 |
| `mf iocs <preuves…> [--format csv\|json] [--defang]` | IOC dédoublonnés | FR-021–FR-023 |
| `mf props <preuve.msg> [--path …] [--tag 0x007D]` | Explorateur de propriétés MAPI | FR-031 |
| `mf raw <preuve> [--hex] [--stream "__substg1.0_1000001F"]` | Source brute / flux / hexdump | FR-031 |
| `mf report <preuves…> --out <rapport> [--format html\|json\|both] [--case <fichier>]` | Rapport + journal d'audit | FR-024–FR-027 |

## Exemples

```console
$ mf hash suspect.msg
suspect.msg  msg  412 873 o
  sha256 3c67dc70854b994e469fc14496d23bf778e718831806ebafc72436c840439f9c
  md5    …

$ mf findings --json lot/*.eml | jq '.[] | select(.severity=="high")'

$ mf extract suspect.eml --out ./pj --neutralize
facture.pdf.exe → ./pj/001_facture.pdf.exe_  sha256 34b9…cab
manifest: ./pj/manifest.json
```

## `manifest.json` (export de PJ)

```json
{
  "tool": "mailforensics 0.1.0",
  "exported_utc": "2026-10-14T10:00:00Z",
  "evidence": { "path": "suspect.eml", "sha256": "…", "md5": "…", "verified": true },
  "files": [
    { "provenance": "suspect.eml › PJ 1", "original_name": "facture.pdf.exe",
      "written_as": "001_facture.pdf.exe_", "size": 32, "sha256": "…", "md5": "…" }
  ]
}
```
