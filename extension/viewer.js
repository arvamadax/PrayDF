const $ = (id) => document.getElementById(id);
const stage = $("stage");
const side = $("sidebar");
const lib = (p) => new URL("lib/" + p, location.href).href;

const TEXT = ["txt", "md", "csv", "json", "log", "xml", "html"];
const IMAGE = ["png", "jpg", "jpeg", "gif", "webp", "svg"];

let current = null; // { destroy() } for the open document, so memory is freed on the next open
let file = null; // the open File, for download
let pdf = null; // { doc, viewer, eventBus, fit, thumbs } while a PDF is open
let sideOpen = matchMedia("(min-width: 768px)").matches;

async function open(f) {
  closeFind();
  await current?.destroy();
  current = pdf = null;
  file = f;
  stage.replaceChildren();
  setPdfUi(false);
  $("name").textContent = document.title = f.name;
  $("download").hidden = $("print").hidden = false;

  const ext = f.name.split(".").pop().toLowerCase();
  try {
    if (ext === "pdf" || f.type === "application/pdf") await showPdf(f);
    else if (ext === "docx") await showDocx(f);
    else if (TEXT.includes(ext)) showText(await f.text());
    else if (IMAGE.includes(ext)) showImage(f);
    else if (ext === "doc") fail("File .doc (Word lama) belum didukung. Simpan ulang sebagai .docx dari Word.");
    else fail(`Format .${ext} belum didukung. Yang bisa dibuka: PDF, DOCX, teks, dan gambar.`);
  } catch (e) {
    fail(`Gagal membuka ${f.name}: ${e.message}`);
  }
}

function fail(msg) {
  setPdfUi(false);
  $("download").hidden = $("print").hidden = true;
  stage.innerHTML = `<div class="empty"><p class="empty-title error"></p></div>`;
  stage.querySelector("p").textContent = msg;
}

function setPdfUi(on) {
  $("pdfTools").hidden = $("sideBtn").hidden = $("searchBtn").hidden = !on;
  $("openApp").hidden = !(on && globalThis.chrome?.downloads); // only inside the extension; the app opens PDF only
  side.hidden = !(on && sideOpen);
  document.body.classList.toggle("side-open", on && sideOpen);
  $("sideBtn").setAttribute("aria-pressed", sideOpen);
}

function paper(tag) {
  const el = document.createElement(tag);
  el.className = "paper";
  stage.append(el);
  return el;
}

function showText(text) {
  paper("pre").textContent = text;
}

function showImage(f) {
  const url = URL.createObjectURL(f);
  stage.append(Object.assign(document.createElement("img"), { src: url, alt: f.name, className: "image" }));
  current = { destroy: () => URL.revokeObjectURL(url) };
}

async function showDocx(f) {
  if (!window.mammoth) await loadScript(lib("mammoth.browser.min.js")); // loaded only when a .docx is opened
  const { value } = await window.mammoth.convertToHtml(
    { arrayBuffer: await f.arrayBuffer() },
    { styleMap: ["p[style-name='Title'] => h1.title:fresh"] }, // Word's Title style isn't mapped by default
  );
  paper("article").innerHTML = value; // mammoth escapes text; extension CSP blocks any script in links
}

function loadScript(src) {
  return new Promise((ok, err) => {
    const s = Object.assign(document.createElement("script"), { src, onload: ok, onerror: () => err(new Error("library tidak ditemukan")) });
    document.head.append(s);
  });
}

let pdfjs; // pdf.js is ~2 MB, so it loads on the first PDF only
async function loadPdfjs() {
  if (pdfjs) return pdfjs;
  // pdf.js measures pages at init, so its CSS must be applied first
  await new Promise((ok) => document.head.append(Object.assign(document.createElement("link"), { rel: "stylesheet", href: lib("pdf_viewer.css"), onload: ok, onerror: ok })));
  const core = await import("./lib/pdf.min.mjs"); // sets globalThis.pdfjsLib, which pdf_viewer.mjs reads
  core.GlobalWorkerOptions.workerSrc = lib("pdf.worker.min.mjs");
  return (pdfjs = { core, ...(await import("./lib/pdf_viewer.mjs")) });
}

async function showPdf(f) {
  const { core, EventBus, PDFLinkService, PDFViewer, PDFFindController, LinkTarget } = await loadPdfjs();

  const task = core.getDocument({
    data: new Uint8Array(await f.arrayBuffer()),
    cMapUrl: lib("cmaps/"),
    standardFontDataUrl: lib("standard_fonts/"),
    wasmUrl: lib("wasm/"),
    iccUrl: lib("iccs/"),
    isEvalSupported: false,
  });
  task.onPassword = (update, reason) => {
    const pw = prompt(reason === core.PasswordResponses.INCORRECT_PASSWORD ? "Password salah. Coba lagi:" : "PDF ini dikunci. Password:");
    pw === null ? task.destroy() : update(pw);
  };
  const state = { fit: "page-width", thumbs: null };
  current = {
    destroy() {
      state.thumbs?.disconnect();
      side.replaceChildren();
      return task.destroy();
    },
  };
  const doc = await task.promise;
  setPdfUi(true); // open the sidebar before pdf.js measures the container for "page-width"

  const scroll = document.createElement("div");
  scroll.className = "pdf-scroll";
  scroll.innerHTML = `<div class="pdfViewer"></div>`;
  stage.append(scroll);

  const eventBus = new EventBus();
  const linkService = new PDFLinkService({ eventBus, externalLinkTarget: LinkTarget.BLANK, externalLinkRel: "noopener noreferrer" });
  const findController = new PDFFindController({ eventBus, linkService });
  const viewer = new PDFViewer({ container: scroll, eventBus, linkService, findController });
  linkService.setViewer(viewer);
  viewer.setDocument(doc);
  linkService.setDocument(doc);
  Object.assign(state, { doc, viewer, eventBus });

  eventBus.on("pagesinit", () => (viewer.currentScaleValue = state.fit));
  eventBus.on("scalechanging", (e) => ($("zoom").textContent = Math.round(e.scale * 100) + "%"));
  eventBus.on("pagechanging", (e) => showPage(e.pageNumber));
  eventBus.on("updatefindmatchescount", (e) => showMatches(e.matchesCount));
  eventBus.on("updatefindcontrolstate", (e) => showMatches(e.matchesCount, e.state));

  $("pageCount").textContent = `/ ${doc.numPages}`;
  $("fit").setAttribute("aria-label", "Pas satu halaman");
  state.thumbs = await buildThumbs(doc, viewer);
  pdf = state;
  showPage(1);
}

// Sidebar thumbnails: drawn only while near the visible area and cleared after, so memory stays flat on long PDFs
async function buildThumbs(doc, viewer) {
  const first = (await doc.getPage(1)).getViewport({ scale: 1 });
  const ratio = () => (viewer.pagesRotation % 180 ? first.height / first.width : first.width / first.height);
  const tasks = new Map(); // canvas -> RenderTask (null while the page is loading)

  const draw = async (btn) => {
    const canvas = btn.firstChild;
    if (tasks.has(canvas)) return;
    tasks.set(canvas, null);
    try {
      const page = await doc.getPage(+btn.dataset.page);
      const rotation = (page.rotate + viewer.pagesRotation) % 360;
      const viewport = page.getViewport({ scale: (120 * devicePixelRatio) / page.getViewport({ scale: 1, rotation }).width, rotation });
      if (!tasks.has(canvas)) return; // cleared while loading
      canvas.width = viewport.width;
      canvas.height = viewport.height;
      canvas.style.aspectRatio = `${viewport.width} / ${viewport.height}`;
      const task = page.render({ canvas, viewport });
      tasks.set(canvas, task);
      await task.promise;
    } catch {
      // cancelled or document closed: nothing to show
    }
  };
  const clear = (btn) => {
    const canvas = btn.firstChild;
    tasks.get(canvas)?.cancel();
    tasks.delete(canvas);
    canvas.width = canvas.height = 0;
  };

  const io = new IntersectionObserver(
    (entries) => entries.forEach((e) => (e.isIntersecting ? draw(e.target) : clear(e.target))),
    { root: side, rootMargin: "300px 0px" },
  );
  const frag = document.createDocumentFragment();
  for (let n = 1; n <= doc.numPages; n++) {
    const btn = Object.assign(document.createElement("button"), { className: "thumb", type: "button" });
    btn.dataset.page = n;
    btn.setAttribute("aria-label", `Halaman ${n}`);
    btn.innerHTML = `<canvas width="0" height="0"></canvas><span>${n}</span>`;
    btn.firstChild.style.aspectRatio = ratio();
    frag.append(btn);
    io.observe(btn);
  }
  side.replaceChildren(frag);
  side.onclick = (e) => {
    const btn = e.target.closest(".thumb");
    if (!btn) return;
    viewer.currentPageNumber = +btn.dataset.page;
    if (matchMedia("(max-width: 767px)").matches) $("sideBtn").click(); // on phones the panel covers the page
  };

  return {
    disconnect() {
      io.disconnect();
      tasks.forEach((t) => t?.cancel());
    },
    redraw() {
      for (const btn of side.children) {
        clear(btn);
        btn.firstChild.style.aspectRatio = ratio();
        io.unobserve(btn);
        io.observe(btn); // re-observing fires the callback again for visible thumbs
      }
    },
  };
}

function showPage(n) {
  $("pageInput").value = n;
  const old = side.querySelector(".active");
  old?.classList.remove("active");
  old?.removeAttribute("aria-current");
  const btn = side.children[n - 1];
  if (!btn) return;
  btn.classList.add("active");
  btn.setAttribute("aria-current", "page");
  if (!side.hidden) btn.scrollIntoView({ block: "nearest" });
}

function refit() {
  const value = pdf?.viewer.currentScaleValue;
  if (value?.startsWith("page-")) pdf.viewer.currentScaleValue = value; // pdf.js doesn't refit on container resize
}

$("sideBtn").onclick = () => {
  sideOpen = !sideOpen;
  setPdfUi(true);
  refit();
  showPage(pdf.viewer.currentPageNumber);
};
addEventListener("resize", refit);

$("pageInput").addEventListener("focus", (e) => e.target.select());
$("pageInput").addEventListener("change", (e) => {
  if (!pdf) return;
  const n = parseInt(e.target.value, 10);
  if (n >= 1 && n <= pdf.doc.numPages) pdf.viewer.currentPageNumber = n;
  e.target.value = pdf.viewer.currentPageNumber;
});
$("zoomIn").onclick = () => pdf.viewer.increaseScale();
$("zoomOut").onclick = () => pdf.viewer.decreaseScale();
$("fit").onclick = () => {
  pdf.fit = pdf.fit === "page-width" ? "page-fit" : "page-width";
  pdf.viewer.currentScaleValue = pdf.fit;
  $("fit").setAttribute("aria-label", pdf.fit === "page-width" ? "Pas satu halaman" : "Pas lebar");
};
$("rotate").onclick = () => {
  pdf.viewer.pagesRotation = (pdf.viewer.pagesRotation + 90) % 360;
  pdf.thumbs.redraw();
};

// Search
function openFind() {
  if (!pdf) return;
  $("findbar").hidden = false;
  $("findInput").focus();
  $("findInput").select();
}
function closeFind() {
  $("findbar").hidden = true;
  $("findCount").textContent = "";
  pdf?.eventBus.dispatch("findbarclose", { source: null });
}
function find(type, findPrevious = false) {
  pdf?.eventBus.dispatch("find", {
    source: null, type, query: $("findInput").value, findPrevious,
    caseSensitive: false, entireWord: false, highlightAll: true, matchDiacritics: false,
  });
}
function showMatches({ current, total }, state) {
  const label = total ? `${current} dari ${total}` : state === pdfjs.FindState.NOT_FOUND ? "Tidak ada" : "";
  $("findCount").textContent = $("findInput").value ? label : "";
}
$("searchBtn").onclick = openFind;
$("findInput").addEventListener("input", () => find(""));
$("findInput").addEventListener("keydown", (e) => {
  if (e.key === "Enter") find("again", e.shiftKey);
  if (e.key === "Escape") closeFind();
});
$("findNext").onclick = () => find("again");
$("findPrev").onclick = () => find("again", true);
$("findClose").onclick = closeFind;
addEventListener("keydown", (e) => {
  if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "f" && pdf) {
    e.preventDefault();
    openFind();
  }
});

// Hand the open PDF to the PrayDF desktop app: save it to Downloads\PrayDF, then pass the full path via praydf:
$("openApp").onclick = async () => {
  const btn = $("openApp");
  const url = URL.createObjectURL(file);
  btn.disabled = true;
  try {
    const api = globalThis.browser ?? chrome; // Firefox: promise-based browser.*; Chromium: chrome.*
    const id = await api.downloads.download({ url, filename: "PrayDF/" + file.name, conflictAction: "uniquify", saveAs: false });
    let path = null;
    // shortcut: polls for up to 10 s instead of listening to downloads.onChanged; fine for a local blob
    for (let i = 0; i < 100 && !path; i++) {
      const [item] = await api.downloads.search({ id });
      if (item?.state === "interrupted") throw new Error(item.error || "unduhan gagal");
      if (item?.state === "complete") path = item.filename;
      else await new Promise((ok) => setTimeout(ok, 100));
    }
    if (!path) throw new Error("menyimpan file terlalu lama");
    location.href = "praydf:" + encodeURIComponent(path);
  } catch (e) {
    alert(`Tidak bisa membuka di aplikasi: ${e.message}`);
  } finally {
    URL.revokeObjectURL(url);
    btn.disabled = false;
  }
};

// Download and print
$("download").onclick = () => {
  const url = URL.createObjectURL(file);
  Object.assign(document.createElement("a"), { href: url, download: file.name }).click();
  setTimeout(() => URL.revokeObjectURL(url), 60_000); // the download reads the URL after this click returns
};

$("print").onclick = async () => {
  if (!pdf) return print(); // DOCX, text, images: print CSS hides the UI
  const btn = $("print");
  const area = $("printArea");
  const { doc } = pdf;
  btn.disabled = true;
  const cleanup = () => {
    area.replaceChildren();
    document.body.classList.remove("print-pdf");
    btn.disabled = false;
  };
  try {
    // Render pages at 150 dpi into images, so the browser prints them like Chrome's viewer does
    for (let n = 1; n <= doc.numPages; n++) {
      const page = await doc.getPage(n);
      const viewport = page.getViewport({ scale: 150 / 72 });
      const canvas = Object.assign(document.createElement("canvas"), { width: viewport.width, height: viewport.height });
      await page.render({ canvas, viewport, intent: "print" }).promise;
      // toDataURL is synchronous (~10 ms); toBlob's callback can be throttled to ~1 s per page
      const img = new Image();
      const loaded = new Promise((ok) => (img.onload = img.onerror = ok)); // decode() can stall for hidden images
      img.src = canvas.toDataURL("image/png");
      canvas.width = canvas.height = 0; // free the full-size canvas right away
      area.append(img);
      await loaded;
    }
    document.body.classList.add("print-pdf");
    addEventListener("afterprint", cleanup, { once: true });
    print();
  } catch {
    cleanup(); // document was closed or a page failed to render
  }
};

// Opened by background.js for a PDF tab: viewer.html#<encoded PDF URL>
async function openUrl(url) {
  $("name").textContent = document.title = "Memuat PDF…";
  try {
    const res = await fetch(url, { credentials: "include" });
    if (!res.ok) throw new Error(`HTTP ${res.status}`);
    const cd = res.headers.get("content-disposition") ?? "";
    let name = cd.match(/filename\*=UTF-8''([^;]+)/i)?.[1] ?? cd.match(/filename="?([^";]+)/i)?.[1] ?? new URL(url).pathname.split("/").pop();
    try {
      name = decodeURIComponent(name);
    } catch {} // keep the raw name if it isn't valid percent-encoding
    if (!/\.pdf$/i.test(name || "")) name = (name || "dokumen") + ".pdf";
    await open(new File([await res.blob()], name, { type: "application/pdf" }));
  } catch (e) {
    $("name").textContent = document.title = "PrayDF";
    fail(`Gagal memuat PDF dari ${url}: ${e.message}`);
  }
}
// Open: picker, the labels that point to it, drag and drop
$("picker").addEventListener("change", (e) => {
  const f = e.target.files[0];
  if (f) open(f);
  e.target.value = ""; // allow reopening the same file
});
for (const label of document.querySelectorAll('label[for="picker"]')) {
  label.addEventListener("keydown", (e) => {
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      $("picker").click();
    }
  });
}
document.addEventListener("dragover", (e) => {
  e.preventDefault();
  stage.classList.add("drag");
});
document.addEventListener("dragleave", (e) => e.relatedTarget || stage.classList.remove("drag"));
document.addEventListener("drop", (e) => {
  e.preventDefault();
  stage.classList.remove("drag");
  const f = e.dataTransfer.files[0];
  if (f) open(f);
});

try {
  const startUrl = decodeURIComponent(location.hash.slice(1));
  if (/^https?:\/\//i.test(startUrl)) openUrl(startUrl);
} catch {} // malformed hash: show the normal start screen
