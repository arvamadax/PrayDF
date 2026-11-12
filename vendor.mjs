// Copies the libraries the extension needs from node_modules into extension/lib/ (MV3 forbids loading code from a CDN).
import { cpSync, rmSync } from "node:fs";

const pdf = "node_modules/pdfjs-dist/";
const files = {
  "build/pdf.min.mjs": "pdf.min.mjs",
  "build/pdf.worker.min.mjs": "pdf.worker.min.mjs",
  "web/pdf_viewer.mjs": "pdf_viewer.mjs",
  "web/pdf_viewer.css": "pdf_viewer.css",
  "web/images": "images",
  cmaps: "cmaps",
  standard_fonts: "standard_fonts",
  wasm: "wasm",
  iccs: "iccs",
};

rmSync("extension/lib", { recursive: true, force: true });
for (const [from, to] of Object.entries(files)) cpSync(pdf + from, "extension/lib/" + to, { recursive: true });
cpSync("node_modules/mammoth/mammoth.browser.min.js", "extension/lib/mammoth.browser.min.js");
console.log("extension/lib/ ready");
