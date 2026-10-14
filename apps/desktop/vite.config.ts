import { fileURLToPath } from "node:url";
import { defineConfig, type Plugin } from "vite";

// research W3 (feature 003) : CSP de la version web. `connect-src 'self'` sert uniquement au
// chargement du module .wasm ; aucun code n'émet de requête ensuite.
const WEB_CSP = [
  "default-src 'none'",
  "script-src 'self' 'wasm-unsafe-eval'",
  "worker-src 'self'",
  "connect-src 'self'",
  "style-src 'self' 'unsafe-inline'",
  "img-src 'self' data:",
  "font-src 'self' data:",
  "frame-src 'self'",
  "base-uri 'none'",
  "form-action 'none'",
].join("; ");

function webCsp(): Plugin {
  return {
    name: "mf-web-csp",
    apply: "build",
    transformIndexHtml: (html) =>
      html.replace("<head>", `<head>\n    <meta http-equiv="Content-Security-Policy" content="${WEB_CSP}" />`),
  };
}

// Deux cibles, une seule interface :
//  - défaut : application de bureau Tauri (dist/) ;
//  - `--mode web` : page statique autonome (dist-web/, chemins relatifs, publiable sur GitHub Pages).
export default defineConfig(({ mode }) => {
  const web = mode === "web";
  return {
    base: web ? "./" : "/",
    clearScreen: false,
    server: { port: 5173, strictPort: true },
    plugins: web ? [webCsp()] : [],
    resolve: {
      alias: web
        ? []
        : [{ find: /^\.\/web-engine$/, replacement: fileURLToPath(new URL("./src/web-engine.stub.ts", import.meta.url)) }],
    },
    worker: { format: "es" },
    build: {
      target: "es2022",
      outDir: web ? "dist-web" : "dist",
      emptyOutDir: true,
      // Polices intégrées au CSS : aucune requête différée quand un style apparaît (FR-W02).
      assetsInlineLimit: (file: string) => /\.(woff2?)$/.test(file),
    },
  };
});
