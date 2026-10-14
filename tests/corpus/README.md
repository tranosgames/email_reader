# Corpus de référence

| Fichier | Origine | Licence | Rôle |
|---|---|---|---|
| `eml/phishing-basic.eml` | synthétique (projet) | MIT | RFC 2047, 2 `Received`, Reply-To divergent, PJ `.pdf.exe` |
| `eml/html-links-inline.eml` | synthétique (projet) | MIT | liens HTML dupliqués, texte ≠ cible, pixel distant, image `cid:`, script |
| `nested/eml-in-eml.eml` | synthétique (projet) | MIT | `message/rfc822` imbriqué avec sa propre PJ et ses URL |
| `msg/ascii.msg`, `msg/unicode.msg`, `msg/attachment.msg`, `msg/test_email.msg` | jeu de test de [msg_parser](https://github.com/marirs/msg-parser-rs) | MIT | MSG ANSI / Unicode, PJ, MSG embarqués (objet et fichier `.msg` joint) |
| `malformed/not-an-email.bin` | synthétique | MIT | format inconnu |
| `malformed/truncated.msg` | 600 premiers octets de `ascii.msg` | MIT | OLE tronqué |

Ne jamais ajouter ici de preuve réelle issue d'un dossier client.
