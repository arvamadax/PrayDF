// PrayDF: viewer PDF ringan untuk Windows (Win32 + PDFium).
#![windows_subsystem = "windows"]

mod doc;
mod link;
mod print;

use doc::{rotate_rect, Doc};
use pdfium_render::prelude::Pdfium;
use std::cell::RefCell;
use std::ffi::OsString;
use std::mem::{size_of, zeroed};
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::ptr::{null, null_mut};
use std::sync::atomic::{AtomicBool, Ordering};
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Graphics::Dwm::DwmSetWindowAttribute;
use windows_sys::Win32::Graphics::Gdi::*;
use windows_sys::Win32::Storage::FileSystem::CopyFileW;
use windows_sys::Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED};
use windows_sys::Win32::System::Console::{AttachConsole, ATTACH_PARENT_PROCESS};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Registry::*;
use windows_sys::Win32::UI::Controls::Dialogs::*;
use windows_sys::Win32::UI::Controls::{SetWindowTheme, SetScrollInfo, EM_SETSEL, EM_SETLIMITTEXT, WM_MOUSELEAVE};
use windows_sys::Win32::UI::HiDpi::*;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;
use windows_sys::Win32::UI::Shell::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

const fn rgb(r: u8, g: u8, b: u8) -> u32 {
    r as u32 | (g as u32) << 8 | (b as u32) << 16
}
const C_TOOLBAR: u32 = rgb(0x32, 0x36, 0x39);
const C_BG: u32 = rgb(0x52, 0x56, 0x59);
const C_SIDE: u32 = rgb(0x2b, 0x2d, 0x30);
const C_TEXT: u32 = rgb(0xf1, 0xf3, 0xf4);
const C_SEC: u32 = rgb(0xbd, 0xc1, 0xc6);
const C_ACTIVE: u32 = rgb(0x8a, 0xb4, 0xf8);
const C_INPUT: u32 = rgb(0x20, 0x21, 0x24);
const C_HOVER: u32 = rgb(0x47, 0x4a, 0x4d);
const C_DIM: u32 = rgb(0x5f, 0x63, 0x68);
const C_SHADOW: u32 = rgb(0x3c, 0x3f, 0x42);
const C_HIT: u32 = rgb(0xff, 0xeb, 0x3b);
const C_HIT_CUR: u32 = rgb(0xff, 0x96, 0x32);

const B_MENU: u32 = 1;
const B_ZOOM_OUT: u32 = 2;
const B_ZOOM_IN: u32 = 3;
const B_FIT: u32 = 4;
const B_ROT: u32 = 5;
const B_OPEN: u32 = 6;
const B_FIND: u32 = 7;
const B_SAVE: u32 = 8;
const B_PRINT: u32 = 9;
const F_PREV: u32 = 10;
const F_NEXT: u32 = 11;
const F_CLOSE: u32 = 12;

const ZOOMS: [f32; 17] = [0.25, 0.33, 0.5, 0.67, 0.75, 0.8, 0.9, 1.0, 1.1, 1.25, 1.5, 1.75, 2.0, 2.5, 3.0, 4.0, 5.0];

#[derive(Clone, Copy, PartialEq)]
enum Fit {
    Width,
    Page,
}

struct App {
    lib: Option<&'static Pdfium>,
    main: HWND,
    view: HWND,
    side: HWND,
    find: HWND,
    page_edit: HWND,
    find_edit: HWND,
    font: HFONT,
    icons: HFONT,
    s: f32, // skala DPI (1.0 = 96 dpi)
    doc: Option<Doc>,
    error: Option<String>,
    query: String,
    zoom: f32,
    fit: Option<Fit>,
    next_fit: Fit,
    sidebar: bool,
    find_open: bool,
    hover: u32,
    mw: i32,
    vw: i32,
    vh: i32,
    sx: i32,
    sy: i32,
    doc_w: i32,
    doc_h: i32,
    pages: Vec<RECT>, // posisi halaman dalam koordinat dokumen
    sh: i32,
    side_sy: i32,
    side_doc_h: i32,
    thumbs: Vec<RECT>,
    page: usize,
    shown_page: usize,
    title: String,
}

thread_local! {
    static APP: RefCell<Option<App>> = const { RefCell::new(None) };
}

/// Akses state; `None` bila sedang dipakai (pesan masuk ulang) sehingga tidak pernah panic.
fn with<R>(f: impl FnOnce(&mut App) -> R) -> Option<R> {
    APP.with(|c| {
        let mut g = c.try_borrow_mut().ok()?;
        g.as_mut().map(f)
    })
}

fn wide(s: impl AsRef<std::ffi::OsStr>) -> Vec<u16> {
    s.as_ref().encode_wide().chain([0]).collect()
}

fn lo(x: isize) -> i32 {
    (x & 0xffff) as u16 as i16 as i32
}
fn hi(x: isize) -> i32 {
    ((x >> 16) & 0xffff) as u16 as i16 as i32
}
fn inside(r: &RECT, x: i32, y: i32) -> bool {
    x >= r.left && x < r.right && y >= r.top && y < r.bottom
}

// ---------- gambar ----------

unsafe fn fill(dc: HDC, r: &RECT, c: u32) {
    SetDCBrushColor(dc, c);
    FillRect(dc, r, GetStockObject(DC_BRUSH) as HBRUSH);
}

unsafe fn shape(dc: HDC, r: &RECT, c: u32, round: i32) {
    SetDCBrushColor(dc, c);
    let ob = SelectObject(dc, GetStockObject(DC_BRUSH));
    let op = SelectObject(dc, GetStockObject(NULL_PEN));
    if round == 0 {
        Ellipse(dc, r.left, r.top, r.right + 1, r.bottom + 1);
    } else {
        RoundRect(dc, r.left, r.top, r.right + 1, r.bottom + 1, round, round);
    }
    SelectObject(dc, ob);
    SelectObject(dc, op);
}

/// Sorotan teks: warna dikalikan (AND) dengan isi halaman sehingga teks tetap terbaca.
unsafe fn mark(dc: HDC, r: &RECT, c: u32) {
    SetDCBrushColor(dc, c);
    let ob = SelectObject(dc, GetStockObject(DC_BRUSH));
    PatBlt(dc, r.left, r.top, r.right - r.left, r.bottom - r.top, 0x00A0_00C9); // PATAND
    SelectObject(dc, ob);
}

unsafe fn text(dc: HDC, font: HFONT, s: &str, r: &RECT, c: u32, flags: u32) {
    // DrawTextW tetap membaca pointer walau panjang 0; Vec kosong punya pointer dangling.
    if s.is_empty() {
        return;
    }
    SelectObject(dc, font);
    SetTextColor(dc, c);
    SetBkMode(dc, TRANSPARENT as _);
    let w: Vec<u16> = s.encode_utf16().collect();
    let mut r = *r;
    DrawTextW(dc, w.as_ptr(), w.len() as i32, &mut r, flags);
}

unsafe fn icon(dc: HDC, font: HFONT, glyph: u32, r: &RECT, c: u32) {
    let s = char::from_u32(glyph).unwrap_or('?').to_string();
    text(dc, font, &s, r, c, DT_CENTER | DT_VCENTER | DT_SINGLELINE);
}

/// Gambar bitmap BGRA ke kotak tujuan (dipakai layar dan printer).
pub(crate) unsafe fn blit(dc: HDC, x: i32, y: i32, w: i32, h: i32, b: &doc::Bmp) {
    let mut bi: BITMAPINFO = zeroed();
    bi.bmiHeader.biSize = size_of::<BITMAPINFOHEADER>() as u32;
    bi.bmiHeader.biWidth = b.w;
    bi.bmiHeader.biHeight = -b.h;
    bi.bmiHeader.biPlanes = 1;
    bi.bmiHeader.biBitCount = 32;
    bi.bmiHeader.biCompression = BI_RGB;
    SetStretchBltMode(dc, HALFTONE);
    SetBrushOrgEx(dc, 0, 0, null_mut());
    StretchDIBits(dc, x, y, w, h, 0, 0, b.w, b.h, b.px.as_ptr() as _, &bi, DIB_RGB_COLORS, SRCCOPY);
}

/// WM_PAINT dengan buffer ganda supaya tidak berkedip.
unsafe fn paint(h: HWND, f: impl FnOnce(HDC, &RECT)) {
    let mut ps: PAINTSTRUCT = zeroed();
    let dc = BeginPaint(h, &mut ps);
    let mut rc: RECT = zeroed();
    GetClientRect(h, &mut rc);
    let mem = CreateCompatibleDC(dc);
    let bmp = CreateCompatibleBitmap(dc, rc.right.max(1), rc.bottom.max(1));
    let old = SelectObject(mem, bmp);
    fill(mem, &rc, C_BG);
    f(mem, &rc);
    BitBlt(dc, 0, 0, rc.right, rc.bottom, mem, 0, 0, SRCCOPY);
    SelectObject(mem, old);
    DeleteObject(bmp);
    DeleteDC(mem);
    EndPaint(h, &ps);
}

// ---------- tata letak & state ----------

struct Toolbar {
    btns: Vec<(u32, RECT)>,
    edit: RECT,
    total: RECT,
    zoom: RECT,
    name: RECT,
    seps: [i32; 2],
}

impl App {
    fn px(&self, v: i32) -> i32 {
        (v as f32 * self.s).round() as i32
    }

    fn toolbar(&self) -> Toolbar {
        let p = |v| self.px(v);
        let (y0, bs) = (p(8), p(40));
        let b = |x: i32| RECT { left: x, top: y0, right: x + bs, bottom: y0 + bs };
        let mid = |x: i32, w: i32| RECT { left: x, top: p(14), right: x + w, bottom: p(42) };
        let mut btns = vec![(B_MENU, b(p(8)))];
        let mut r = self.mw - p(8);
        for id in [B_PRINT, B_SAVE, B_FIND, B_OPEN] {
            r -= bs;
            btns.push((id, b(r)));
        }
        let mut x = (self.mw - p(364)) / 2;
        let edit = mid(x, p(44));
        x += p(52);
        let total = mid(x, p(48));
        x += p(48);
        let sep0 = x + p(12);
        x += p(24);
        btns.push((B_ZOOM_OUT, b(x)));
        x += bs;
        let zoom = mid(x, p(56));
        x += p(56);
        btns.push((B_ZOOM_IN, b(x)));
        x += bs;
        let sep1 = x + p(12);
        x += p(24);
        btns.push((B_FIT, b(x)));
        btns.push((B_ROT, b(x + bs)));
        let name_end = if self.doc.is_some() { edit.left - p(16) } else { r - p(8) };
        let name = RECT { left: p(56), top: 0, right: name_end, bottom: p(56) };
        Toolbar { btns, edit, total, zoom, name, seps: [sep0, sep1] }
    }

    fn visible(&self, id: u32) -> bool {
        self.doc.is_some() || !matches!(id, B_ZOOM_OUT | B_ZOOM_IN | B_FIT | B_ROT)
    }
    fn enabled(&self, id: u32) -> bool {
        self.doc.is_some() || matches!(id, B_MENU | B_OPEN)
    }

    fn glyph(&self, id: u32) -> u32 {
        match id {
            B_MENU => 0xE700,
            B_ZOOM_OUT => 0xE71F,
            B_ZOOM_IN => 0xE8A3,
            B_FIT if self.next_fit == Fit::Page => 0xE73F,
            B_FIT => 0xE740,
            B_ROT => 0xE7AD,
            B_OPEN => 0xE8E5,
            B_FIND => 0xE721,
            B_SAVE => 0xE896,
            B_PRINT => 0xE749,
            F_PREV => 0xE70E,
            F_NEXT => 0xE70D,
            _ => 0xE711,
        }
    }

    fn hit(&self, x: i32, y: i32) -> u32 {
        if y >= self.px(56) {
            return 0;
        }
        let tb = self.toolbar();
        let found = tb.btns.iter().find(|(id, r)| self.visible(*id) && self.enabled(*id) && inside(r, x, y));
        found.map_or(0, |(id, _)| *id)
    }

    fn find_rects(&self) -> [RECT; 4] {
        let p = |v| self.px(v);
        let b = |x: i32| RECT { left: p(x), top: p(8), right: p(x + 32), bottom: p(40) };
        [RECT { left: p(232), top: 0, right: p(300), bottom: p(48) }, b(300), b(332), b(364)]
    }

    fn fit_zoom(&self, f: Fit) -> f32 {
        let Some(doc) = &self.doc else { return 1.0 };
        let unit = self.s * 96.0 / 72.0;
        let pad = self.px(8) as f32 * 2.0;
        let maxw = (0..doc.count()).map(|i| doc.size(i).0).fold(1.0, f32::max);
        let zw = (self.vw as f32 - pad) / (maxw * unit);
        let z = match f {
            Fit::Width => zw,
            Fit::Page => zw.min((self.vh as f32 - pad) / (doc.size(self.page).1 * unit)),
        };
        z.clamp(0.1, 5.0)
    }

    /// Halaman dan posisi relatif (0..1) pada jarak `ay` dari atas tampilan.
    fn anchor(&self, ay: i32) -> (usize, f32) {
        let y = self.sy + ay;
        let i = self.pages.partition_point(|r| r.bottom < y).min(self.pages.len().saturating_sub(1));
        match self.pages.get(i) {
            Some(r) => (i, ((y - r.top) as f32 / (r.bottom - r.top).max(1) as f32).clamp(0.0, 1.0)),
            None => (0, 0.0),
        }
    }

    /// Hitung ulang posisi halaman dan thumbnail; titik pada `ay` tetap di tempatnya.
    fn relayout(&mut self, ay: i32) {
        let (ap, frac) = self.anchor(ay);
        self.pages.clear();
        self.thumbs.clear();
        if let Some(f) = self.fit {
            self.zoom = self.fit_zoom(f);
        }
        let gap = self.px(8);
        let (tw, th, sw) = (self.px(120) as f32, self.px(150) as f32, self.px(200));
        let (tgap, ttop) = (self.px(32), self.px(16));
        let Some(doc) = &self.doc else {
            (self.doc_w, self.doc_h, self.side_doc_h) = (0, 0, 0);
            return;
        };
        let ppp = self.zoom * self.s * 96.0 / 72.0;
        let sizes: Vec<(i32, i32)> = (0..doc.count())
            .map(|i| {
                let (w, h) = doc.size(i);
                ((w * ppp).round().max(1.0) as i32, (h * ppp).round().max(1.0) as i32)
            })
            .collect();
        let maxw = sizes.iter().map(|s| s.0).max().unwrap_or(0);
        self.doc_w = self.vw.max(maxw + 2 * gap);
        let mut y = gap;
        for &(w, h) in &sizes {
            let x = (self.doc_w - w) / 2;
            self.pages.push(RECT { left: x, top: y, right: x + w, bottom: y + h });
            y += h + gap;
        }
        self.doc_h = y;
        let mut y = ttop;
        for i in 0..doc.count() {
            let (w, h) = doc.size(i);
            let k = (tw / w).min(th / h);
            let (w, h) = ((w * k).round() as i32, (h * k).round() as i32);
            let x = (sw - w) / 2;
            self.thumbs.push(RECT { left: x, top: y, right: x + w, bottom: y + h });
            y += h + tgap;
        }
        self.side_doc_h = y;
        if let Some(r) = self.pages.get(ap) {
            self.sy = r.top + ((r.bottom - r.top) as f32 * frac) as i32 - ay;
        }
        self.scroll(self.sx, self.sy);
        self.side_scroll(self.side_sy);
    }

    fn scroll(&mut self, x: i32, y: i32) {
        self.sx = x.clamp(0, (self.doc_w - self.vw).max(0));
        let max_sy = (self.doc_h - self.vh).max(0);
        self.sy = y.clamp(0, max_sy);
        let n = self.pages.len();
        if n == 0 {
            return;
        }
        let line = self.sy + self.vh / 3;
        let mut p = self.pages.partition_point(|r| r.bottom < line).min(n - 1);
        if max_sy > 0 && self.sy >= max_sy {
            p = n - 1;
        }
        self.set_page(p);
    }

    fn side_scroll(&mut self, y: i32) {
        self.side_sy = y.clamp(0, (self.side_doc_h - self.sh).max(0));
    }

    fn set_page(&mut self, p: usize) {
        self.page = p;
        if let Some(r) = self.thumbs.get(p) {
            let (top, bottom) = (r.top - self.px(16), r.bottom + self.px(28));
            if top < self.side_sy {
                self.side_scroll(top);
            } else if bottom > self.side_sy + self.sh {
                self.side_scroll(bottom - self.sh);
            }
        }
    }

    fn goto_page(&mut self, p: usize) {
        let Some(r) = self.pages.get(p) else { return };
        let y = r.top - self.px(8);
        self.scroll(self.sx, y);
        self.set_page(p);
    }

    fn zoom_step(&mut self, dir: i32, ay: i32) {
        let z = self.zoom;
        self.zoom = if dir > 0 {
            ZOOMS.iter().copied().find(|&s| s > z + 0.001).unwrap_or(5.0)
        } else {
            ZOOMS.iter().rev().copied().find(|&s| s < z - 0.001).unwrap_or(0.25)
        };
        self.fit = None;
        self.relayout(ay);
    }

    fn open(&mut self, path: &Path) {
        self.doc = None;
        self.error = None;
        self.query.clear();
        self.find_open = false;
        let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        match self.lib {
            None => self.error = Some("pdfium.dll tidak ditemukan.\nLetakkan pdfium.dll di samping praydf.exe.".into()),
            Some(lib) => match Doc::open(lib, path) {
                Ok(d) => self.doc = Some(d),
                Err(e) => self.error = Some(format!("Tidak bisa membuka {name}\n\n{e}")),
            },
        }
        (self.sx, self.sy, self.side_sy, self.page) = (0, 0, 0, 0);
        self.shown_page = usize::MAX;
        self.fit = None;
        self.next_fit = Fit::Page;
        self.zoom = self.fit_zoom(Fit::Width).min(1.0);
        self.relayout(0);
        self.scroll(0, 0);
    }

    /// Gulir agar hasil cari aktif terlihat.
    fn show_hit(&mut self) {
        let Some(doc) = &self.doc else { return };
        let Some(hit) = doc.hits.get(doc.cur) else { return };
        let Some(r) = self.pages.get(hit.page).copied() else { return };
        let q = hit.rects.first().map_or([0.0; 4], |x| rotate_rect(*x, doc.rot));
        let x = r.left + (q[0] * (r.right - r.left) as f32) as i32;
        let y = r.top + (q[1] * (r.bottom - r.top) as f32) as i32;
        let m = self.px(40);
        let nx = if x < self.sx || x > self.sx + self.vw - m { x - self.vw / 2 } else { self.sx };
        let ny = if y < self.sy || y > self.sy + self.vh - m { y - self.vh / 3 } else { self.sy };
        self.scroll(nx, ny);
    }

    // ---------- lukis ----------

    unsafe fn paint_toolbar(&self, dc: HDC, rc: &RECT) {
        let p = |v| self.px(v);
        fill(dc, &RECT { bottom: p(56), ..*rc }, C_TOOLBAR);
        let tb = self.toolbar();
        let name = self.doc.as_ref().map_or("PrayDF", |d| d.name.as_str());
        let flags = DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS | DT_NOPREFIX;
        text(dc, self.font, name, &tb.name, C_TEXT, flags);
        for (id, r) in &tb.btns {
            if !self.visible(*id) {
                continue;
            }
            let on = self.enabled(*id);
            if on && self.hover == *id {
                shape(dc, r, C_HOVER, 0);
            }
            icon(dc, self.icons, self.glyph(*id), r, if on { C_TEXT } else { C_DIM });
        }
        if let Some(doc) = &self.doc {
            shape(dc, &tb.edit, C_INPUT, p(6));
            let c = DT_VCENTER | DT_SINGLELINE;
            text(dc, self.font, &format!("/ {}", doc.count()), &tb.total, C_SEC, c | DT_LEFT);
            text(dc, self.font, &format!("{}%", (self.zoom * 100.0).round()), &tb.zoom, C_TEXT, c | DT_CENTER);
            for x in tb.seps {
                fill(dc, &RECT { left: x, top: p(18), right: x + 1, bottom: p(38) }, C_DIM);
            }
        }
    }

    unsafe fn paint_view(&mut self, dc: HDC, rc: &RECT) {
        let center = DT_CENTER | DT_NOPREFIX | DT_WORDBREAK;
        if let Some(e) = &self.error {
            let r = RECT { top: rc.bottom / 3, ..*rc };
            text(dc, self.font, e, &r, C_TEXT, center);
            return;
        }
        let Some(doc) = self.doc.as_mut() else {
            let r = RECT { top: rc.bottom / 3, ..*rc };
            text(dc, self.font, "Buka PDF dengan Ctrl+O, tombol folder, atau seret file ke sini.", &r, C_SEC, center);
            return;
        };
        let first = self.pages.partition_point(|r| r.bottom < self.sy);
        for (i, page) in self.pages.iter().enumerate().skip(first) {
            if page.top > self.sy + self.vh {
                break;
            }
            let (w, h) = (page.right - page.left, page.bottom - page.top);
            let d = RECT { left: page.left - self.sx, top: page.top - self.sy, right: page.right - self.sx, bottom: page.bottom - self.sy };
            fill(dc, &RECT { left: d.left - 1, top: d.top - 1, right: d.right + 1, bottom: d.bottom + 2 }, C_SHADOW);
            match doc.bitmap(i, w, h, false) {
                Some(b) => blit(dc, d.left, d.top, w, h, b),
                None => fill(dc, &d, rgb(255, 255, 255)),
            }
            for (k, hit) in doc.hits.iter().enumerate().filter(|(_, x)| x.page == i) {
                for q in &hit.rects {
                    let q = rotate_rect(*q, doc.rot);
                    let r = RECT {
                        left: d.left + (q[0] * w as f32) as i32 - 1,
                        top: d.top + (q[1] * h as f32) as i32 - 1,
                        right: d.left + (q[2] * w as f32).ceil() as i32 + 1,
                        bottom: d.top + (q[3] * h as f32).ceil() as i32 + 1,
                    };
                    mark(dc, &r, if k == doc.cur { C_HIT_CUR } else { C_HIT });
                }
            }
        }
    }

    unsafe fn paint_side(&mut self, dc: HDC, rc: &RECT) {
        fill(dc, rc, C_SIDE);
        let (b3, b4, b20) = (self.px(3), self.px(4), self.px(20));
        let Some(doc) = self.doc.as_mut() else { return };
        for (i, t) in self.thumbs.iter().enumerate() {
            let d = RECT { top: t.top - self.side_sy, bottom: t.bottom - self.side_sy, ..*t };
            if d.bottom + b20 < 0 {
                continue;
            }
            if d.top > rc.bottom {
                break;
            }
            if i == self.page {
                fill(dc, &RECT { left: d.left - b3, top: d.top - b3, right: d.right + b3, bottom: d.bottom + b3 }, C_ACTIVE);
            }
            let (w, h) = (d.right - d.left, d.bottom - d.top);
            match doc.bitmap(i, w, h, true) {
                Some(b) => blit(dc, d.left, d.top, w, h, b),
                None => fill(dc, &d, rgb(255, 255, 255)),
            }
            let label = RECT { left: 0, top: d.bottom + b4, right: rc.right, bottom: d.bottom + b4 + b20 };
            let c = if i == self.page { C_ACTIVE } else { C_SEC };
            text(dc, self.font, &(i + 1).to_string(), &label, c, DT_CENTER | DT_SINGLELINE);
        }
    }

    unsafe fn paint_find(&self, dc: HDC, rc: &RECT) {
        fill(dc, rc, C_DIM);
        fill(dc, &RECT { left: 1, top: 1, right: rc.right - 1, bottom: rc.bottom - 1 }, C_TOOLBAR);
        let p = |v| self.px(v);
        shape(dc, &RECT { left: p(8), top: p(8), right: p(224), bottom: p(40) }, C_INPUT, p(6));
        let [count, prev, next, close] = self.find_rects();
        let label = match &self.doc {
            Some(d) if !self.query.is_empty() && d.hits.is_empty() => "0 hasil".to_string(),
            Some(d) if !d.hits.is_empty() => format!("{} dari {}", d.cur + 1, d.hits.len()),
            _ => String::new(),
        };
        text(dc, self.font, &label, &count, C_SEC, DT_CENTER | DT_VCENTER | DT_SINGLELINE);
        for (id, r) in [(F_PREV, prev), (F_NEXT, next), (F_CLOSE, close)] {
            icon(dc, self.icons, self.glyph(id), &r, C_TEXT);
        }
    }
}

// ---------- sinkronisasi ke kontrol Win32 (selalu di luar pinjaman state) ----------

unsafe fn set_scroll(h: HWND, bar: SCROLLBAR_CONSTANTS, total: i32, page: i32, pos: i32, keep: bool) {
    let si = SCROLLINFO {
        cbSize: size_of::<SCROLLINFO>() as u32,
        fMask: SIF_RANGE | SIF_PAGE | SIF_POS | if keep { SIF_DISABLENOSCROLL } else { 0 },
        nMin: 0,
        nMax: (total - 1).max(0),
        nPage: page.max(0) as u32,
        nPos: pos,
        nTrackPos: 0,
    };
    SetScrollInfo(h, bar, &si, 1);
}

fn sync() {
    let Some((st, page_text, title)) = with(|a| {
        let page_text = (a.doc.is_some() && a.page != a.shown_page).then(|| {
            a.shown_page = a.page;
            (a.page + 1).to_string()
        });
        let t = a.doc.as_ref().map_or("PrayDF".to_string(), |d| format!("{} - PrayDF", d.name));
        let title = (t != a.title).then(|| {
            a.title = t.clone();
            t
        });
        let st = (a.main, a.view, a.side, a.find, a.page_edit, [a.doc_h, a.vh, a.sy, a.doc_w, a.vw, a.sx, a.side_doc_h, a.sh, a.side_sy]);
        (st, page_text, title)
    }) else {
        return;
    };
    let (main, view, side, find, edit, v) = st;
    unsafe {
        set_scroll(view, SB_VERT, v[0], v[1], v[2], true);
        set_scroll(view, SB_HORZ, v[3], v[4], v[5], false);
        set_scroll(side, SB_VERT, v[6], v[7], v[8], true);
        if let Some(t) = page_text {
            SetWindowTextW(edit, wide(t).as_ptr());
        }
        if let Some(t) = title {
            SetWindowTextW(main, wide(t).as_ptr());
        }
        for h in [main, view, side, find] {
            InvalidateRect(h, null(), 0);
        }
    }
}

fn layout_children() {
    let Some((hw, rc, sw, tb, edit, f, show, p)) = with(|a| {
        let mut rc: RECT = unsafe { zeroed() };
        unsafe { GetClientRect(a.main, &mut rc) };
        a.mw = rc.right;
        let sw = if a.sidebar { a.px(200) } else { 0 };
        let e = a.toolbar().edit;
        let edit = RECT { left: e.left + a.px(4), top: e.top + a.px(4), right: e.right - a.px(4), bottom: e.bottom - a.px(4) };
        let (fw, fh) = (a.px(404), a.px(48));
        let fx = rc.right - fw - a.px(8) - unsafe { GetSystemMetrics(SM_CXVSCROLL) };
        let f = RECT { left: fx, top: a.px(64), right: fx + fw, bottom: a.px(64) + fh };
        let hw = [a.main, a.side, a.view, a.page_edit, a.find, a.find_edit];
        (hw, rc, sw, a.px(56), edit, f, [a.sidebar, a.doc.is_some(), a.find_open], [a.px(16), a.px(14), a.px(200), a.px(20)])
    }) else {
        return;
    };
    let [main, side, view, page_edit, find, find_edit] = hw;
    let sh = |on: bool| if on { SW_SHOWNA } else { SW_HIDE };
    unsafe {
        MoveWindow(side, 0, tb, sw, rc.bottom - tb, 1);
        ShowWindow(side, sh(show[0]));
        MoveWindow(view, sw, tb, rc.right - sw, rc.bottom - tb, 1);
        MoveWindow(page_edit, edit.left, edit.top, edit.right - edit.left, edit.bottom - edit.top, 1);
        ShowWindow(page_edit, sh(show[1]));
        MoveWindow(find, f.left, f.top, f.right - f.left, f.bottom - f.top, 1);
        MoveWindow(find_edit, p[0], p[1], p[2], p[3], 1);
        let vis = if show[2] { SWP_SHOWWINDOW } else { SWP_HIDEWINDOW };
        SetWindowPos(find, HWND_TOP, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | vis);
        InvalidateRect(main, null(), 0);
    }
}

fn open_path(p: &Path) {
    with(|a| a.open(p));
    layout_children();
    sync();
}

fn handles() -> Option<(HWND, HWND, HWND, HWND)> {
    with(|a| (a.main, a.view, a.page_edit, a.find_edit))
}

fn edit_text(h: HWND) -> String {
    let mut buf = [0u16; 512];
    let n = unsafe { GetWindowTextW(h, buf.as_mut_ptr(), buf.len() as i32) };
    String::from_utf16_lossy(&buf[..n.max(0) as usize])
}

fn msgbox(owner: HWND, s: &str) {
    unsafe { MessageBoxW(owner, wide(s).as_ptr(), wide("PrayDF").as_ptr(), MB_ICONWARNING | MB_OK) };
}

fn file_dialog(owner: HWND, save: bool, name: &str) -> Option<PathBuf> {
    let mut buf = vec![0u16; 4096];
    for (d, c) in buf.iter_mut().zip(name.encode_utf16().take(4000)) {
        *d = c;
    }
    let filter: Vec<u16> = "Dokumen PDF (*.pdf)\0*.pdf\0Semua file\0*.*\0\0".encode_utf16().collect();
    let ext = wide("pdf");
    unsafe {
        let mut o: OPENFILENAMEW = zeroed();
        o.lStructSize = size_of::<OPENFILENAMEW>() as u32;
        o.hwndOwner = owner;
        o.lpstrFilter = filter.as_ptr();
        o.lpstrFile = buf.as_mut_ptr();
        o.nMaxFile = buf.len() as u32;
        o.lpstrDefExt = ext.as_ptr();
        o.Flags = OFN_EXPLORER | OFN_PATHMUSTEXIST | if save { OFN_OVERWRITEPROMPT } else { OFN_FILEMUSTEXIST };
        let ok = if save { GetSaveFileNameW(&mut o) } else { GetOpenFileNameW(&mut o) };
        if ok == 0 {
            return None;
        }
    }
    let n = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    Some(PathBuf::from(OsString::from_wide(&buf[..n])))
}

fn find_go(back: bool) {
    let Some((_, _, _, fe)) = handles() else { return };
    let q = edit_text(fe);
    with(|a| {
        let Some(doc) = a.doc.as_mut() else { return };
        if q != a.query {
            doc.search(&q);
            doc.cur = doc.hits.iter().position(|h| h.page >= a.page).unwrap_or(0);
            a.query = q;
        } else if !doc.hits.is_empty() {
            let n = doc.hits.len();
            doc.cur = if back { (doc.cur + n - 1) % n } else { (doc.cur + 1) % n };
        }
        a.show_hit();
    });
    sync();
}

fn command(id: u32) {
    match id {
        B_MENU => {
            with(|a| a.sidebar = !a.sidebar);
            layout_children();
        }
        B_ZOOM_IN | B_ZOOM_OUT => {
            with(|a| a.zoom_step(if id == B_ZOOM_IN { 1 } else { -1 }, a.vh / 2));
        }
        B_FIT => {
            with(|a| {
                a.fit = Some(a.next_fit);
                a.next_fit = if a.next_fit == Fit::Page { Fit::Width } else { Fit::Page };
                let p = a.page;
                a.relayout(a.vh / 2);
                if a.fit == Some(Fit::Page) {
                    a.goto_page(p);
                }
            });
        }
        B_ROT => {
            with(|a| {
                if let Some(d) = a.doc.as_mut() {
                    d.rot = (d.rot + 1) % 4;
                    a.relayout(a.vh / 2);
                }
            });
        }
        B_OPEN => {
            let Some((main, ..)) = handles() else { return };
            if let Some(p) = file_dialog(main, false, "") {
                open_path(&p);
            }
        }
        B_SAVE => {
            let Some((main, src, name)) = with(|a| a.doc.as_ref().map(|d| (a.main, d.path.clone(), d.name.clone()))).flatten() else {
                return;
            };
            if let Some(dst) = file_dialog(main, true, &name) {
                if dst != src && unsafe { CopyFileW(wide(&src).as_ptr(), wide(&dst).as_ptr(), 0) } == 0 {
                    msgbox(main, &format!("Gagal menyimpan salinan:\n{}", std::io::Error::last_os_error()));
                }
            }
        }
        B_PRINT => {
            let Some((main, n)) = with(|a| a.doc.as_ref().map(|d| (a.main, d.count()))).flatten() else { return };
            let Some(job) = print::dialog(main, n) else { return };
            if let Some(Some(Err(e))) = with(|a| a.doc.as_ref().map(|d| print::run(job, d))) {
                msgbox(main, &format!("Gagal mencetak:\n{e}"));
            }
        }
        B_FIND => {
            let Some(fe) = with(|a| {
                a.find_open = a.doc.is_some();
                a.find_open.then_some(a.find_edit)
            })
            .flatten() else {
                return;
            };
            layout_children();
            unsafe {
                SetFocus(fe);
                SendMessageW(fe, EM_SETSEL, 0, -1);
            }
        }
        F_PREV | F_NEXT => find_go(id == F_PREV),
        F_CLOSE => {
            let Some(view) = with(|a| {
                a.find_open = false;
                a.query.clear();
                if let Some(d) = a.doc.as_mut() {
                    d.hits.clear();
                }
                a.view
            }) else {
                return;
            };
            layout_children();
            unsafe { SetFocus(view) };
        }
        _ => {}
    }
    sync();
}

/// Tombol keyboard global. `true` bila sudah ditangani.
fn on_key(msg: &MSG) -> bool {
    let vk = msg.wParam as u16;
    let down = |k: VIRTUAL_KEY| unsafe { GetKeyState(k as i32) } < 0;
    let Some((_, view, page_edit, find_edit)) = handles() else { return false };
    if down(VK_CONTROL) {
        let id = match vk {
            0x4F => B_OPEN,  // O
            0x53 => B_SAVE,  // S
            0x50 => B_PRINT, // P
            0x46 => B_FIND,  // F
            0x30 => B_FIT,   // 0
            VK_OEM_PLUS | VK_ADD => B_ZOOM_IN,
            VK_OEM_MINUS | VK_SUBTRACT => B_ZOOM_OUT,
            _ => return false,
        };
        command(id);
        return true;
    }
    if msg.hwnd == page_edit {
        match vk {
            VK_RETURN => {
                let n: usize = edit_text(page_edit).trim().parse().unwrap_or(0);
                with(|a| {
                    let count = a.doc.as_ref().map_or(0, |d| d.count());
                    if count > 0 && n > 0 {
                        a.goto_page(n.min(count) - 1);
                    }
                    a.shown_page = usize::MAX;
                });
            }
            VK_ESCAPE => {
                with(|a| a.shown_page = usize::MAX);
            }
            _ => return false,
        }
        unsafe { SetFocus(view) };
        sync();
        return true;
    }
    if msg.hwnd == find_edit {
        match vk {
            VK_RETURN => find_go(down(VK_SHIFT)),
            VK_ESCAPE => command(F_CLOSE),
            _ => return false,
        }
        return true;
    }
    if vk == VK_F3 {
        find_go(down(VK_SHIFT));
        return true;
    }
    let done = with(|a| {
        let (line, pg) = (a.px(60), a.vh - a.px(60));
        let (dx, dy) = match vk {
            VK_DOWN => (0, line),
            VK_UP => (0, -line),
            VK_RIGHT => (line, 0),
            VK_LEFT => (-line, 0),
            VK_NEXT | VK_SPACE => (0, pg),
            VK_PRIOR => (0, -pg),
            VK_HOME => (0, -a.sy),
            VK_END => (0, a.doc_h),
            _ => return false,
        };
        a.scroll(a.sx + dx, a.sy + dy);
        true
    });
    if done == Some(true) {
        sync();
        return true;
    }
    false
}

fn scroll_code(h: HWND, bar: SCROLLBAR_CONSTANTS, code: i32, pos: i32, line: i32, page: i32) -> i32 {
    match code {
        x if x == SB_LINEUP as i32 => pos - line,
        x if x == SB_LINEDOWN as i32 => pos + line,
        x if x == SB_PAGEUP as i32 => pos - page,
        x if x == SB_PAGEDOWN as i32 => pos + page,
        x if x == SB_TOP as i32 => 0,
        x if x == SB_BOTTOM as i32 => i32::MAX,
        x if x == SB_THUMBTRACK as i32 || x == SB_THUMBPOSITION as i32 => unsafe {
            let mut si: SCROLLINFO = zeroed();
            si.cbSize = size_of::<SCROLLINFO>() as u32;
            si.fMask = SIF_TRACKPOS;
            GetScrollInfo(h, bar, &mut si);
            si.nTrackPos
        },
        _ => pos,
    }
}

fn drop_files(w: WPARAM) {
    let hdrop = w as HDROP;
    let mut buf = vec![0u16; 4096];
    let n = unsafe {
        let n = DragQueryFileW(hdrop, 0, buf.as_mut_ptr(), buf.len() as u32);
        DragFinish(hdrop);
        n as usize
    };
    if n > 0 && n < buf.len() {
        open_path(&PathBuf::from(OsString::from_wide(&buf[..n])));
    }
}

unsafe fn ctl_color(w: WPARAM) -> LRESULT {
    let dc = w as HDC;
    SetTextColor(dc, C_TEXT);
    SetBkColor(dc, C_INPUT);
    SetDCBrushColor(dc, C_INPUT);
    GetStockObject(DC_BRUSH) as LRESULT
}

// ---------- window procedure ----------

static IN_PANIC: AtomicBool = AtomicBool::new(false);
static DOWNLOAD: std::sync::Mutex<Option<Result<PathBuf, String>>> = std::sync::Mutex::new(None);
const WM_DOWNLOADED: u32 = WM_APP + 1;

fn report_panic(e: Box<dyn std::any::Any + Send>) {
    if IN_PANIC.swap(true, Ordering::SeqCst) {
        return;
    }
    let m = e
        .downcast_ref::<&str>()
        .map(|s| s.to_string())
        .or_else(|| e.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "tidak diketahui".into());
    let s = format!("Terjadi kesalahan internal:\n{m}\n\nAplikasi tetap berjalan.");
    unsafe { MessageBoxW(null_mut(), wide(s).as_ptr(), wide("PrayDF").as_ptr(), MB_ICONERROR | MB_OK) };
    IN_PANIC.store(false, Ordering::SeqCst);
}

type Handler = unsafe fn(HWND, u32, WPARAM, LPARAM) -> LRESULT;

fn guard(f: Handler, h: HWND, m: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    match catch_unwind(AssertUnwindSafe(|| unsafe { f(h, m, w, l) })) {
        Ok(r) => r,
        Err(e) => {
            report_panic(e);
            unsafe { DefWindowProcW(h, m, w, l) }
        }
    }
}

unsafe extern "system" fn main_proc(h: HWND, m: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    guard(main_msg, h, m, w, l)
}
unsafe extern "system" fn view_proc(h: HWND, m: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    guard(view_msg, h, m, w, l)
}
unsafe extern "system" fn side_proc(h: HWND, m: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    guard(side_msg, h, m, w, l)
}
unsafe extern "system" fn find_proc(h: HWND, m: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    guard(find_msg, h, m, w, l)
}

unsafe fn main_msg(h: HWND, m: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    match m {
        WM_SIZE => layout_children(),
        WM_PAINT => paint(h, |dc, rc| {
            with(|a| a.paint_toolbar(dc, rc));
        }),
        WM_ERASEBKGND => return 1,
        WM_LBUTTONDOWN => {
            if let Some(id) = with(|a| a.hit(lo(l), hi(l))) {
                command(id);
            }
        }
        WM_MOUSEMOVE => {
            let mut t = TRACKMOUSEEVENT { cbSize: size_of::<TRACKMOUSEEVENT>() as u32, dwFlags: TME_LEAVE, hwndTrack: h, dwHoverTime: 0 };
            TrackMouseEvent(&mut t);
            let changed = with(|a| {
                let id = a.hit(lo(l), hi(l));
                std::mem::replace(&mut a.hover, id) != id
            });
            if changed == Some(true) {
                InvalidateRect(h, null(), 0);
            }
        }
        WM_MOUSELEAVE => {
            with(|a| a.hover = 0);
            InvalidateRect(h, null(), 0);
        }
        WM_MOUSEWHEEL => {
            if let Some((_, view, ..)) = handles() {
                return SendMessageW(view, m, w, l);
            }
        }
        WM_CTLCOLOREDIT => return ctl_color(w),
        WM_DROPFILES => drop_files(w),
        WM_DOWNLOADED => match DOWNLOAD.lock().unwrap_or_else(|e| e.into_inner()).take() {
            Some(Ok(p)) => open_path(&p),
            Some(Err(e)) => {
                with(|a| a.error = Some(e));
                sync();
            }
            None => {}
        },
        WM_GETMINMAXINFO => {
            if let Some((x, y)) = with(|a| (a.px(760), a.px(420))) {
                let mi = &mut *(l as *mut MINMAXINFO);
                mi.ptMinTrackSize = POINT { x, y };
            }
        }
        WM_DESTROY => {
            with(|a| a.doc = None);
            PostQuitMessage(0);
        }
        _ => return DefWindowProcW(h, m, w, l),
    }
    0
}

unsafe fn view_msg(h: HWND, m: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    match m {
        WM_SIZE => {
            with(|a| {
                (a.vw, a.vh) = (lo(l), hi(l));
                a.relayout(0);
            });
            sync();
        }
        WM_PAINT => paint(h, |dc, rc| {
            with(|a| a.paint_view(dc, rc));
        }),
        WM_ERASEBKGND => return 1,
        WM_VSCROLL | WM_HSCROLL => {
            with(|a| {
                let vert = m == WM_VSCROLL;
                let (bar, pos, page) = if vert { (SB_VERT, a.sy, a.vh) } else { (SB_HORZ, a.sx, a.vw) };
                let p = scroll_code(h, bar, lo(w as isize), pos, a.px(40), page - a.px(40));
                if vert {
                    a.scroll(a.sx, p)
                } else {
                    a.scroll(p, a.sy)
                }
            });
            sync();
        }
        WM_MOUSEWHEEL => {
            let delta = hi(w as isize);
            let mut pt = POINT { x: lo(l), y: hi(l) };
            ScreenToClient(h, &mut pt);
            with(|a| {
                if GetKeyState(VK_CONTROL as i32) < 0 {
                    a.zoom_step(delta.signum(), pt.y);
                } else if GetKeyState(VK_SHIFT as i32) < 0 {
                    a.scroll(a.sx - delta * a.px(100) / 120, a.sy);
                } else {
                    a.scroll(a.sx, a.sy - delta * a.px(100) / 120);
                }
            });
            sync();
        }
        WM_LBUTTONDOWN => {
            SetFocus(h);
        }
        WM_DROPFILES => drop_files(w),
        _ => return DefWindowProcW(h, m, w, l),
    }
    0
}

unsafe fn side_msg(h: HWND, m: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    match m {
        WM_SIZE => {
            with(|a| {
                a.sh = hi(l);
                a.side_scroll(a.side_sy);
            });
            sync();
        }
        WM_PAINT => paint(h, |dc, rc| {
            with(|a| a.paint_side(dc, rc));
        }),
        WM_ERASEBKGND => return 1,
        WM_VSCROLL => {
            with(|a| {
                let p = scroll_code(h, SB_VERT, lo(w as isize), a.side_sy, a.px(40), a.sh - a.px(40));
                a.side_scroll(p);
            });
            sync();
        }
        WM_MOUSEWHEEL => {
            with(|a| a.side_scroll(a.side_sy - hi(w as isize) * a.px(100) / 120));
            sync();
        }
        WM_LBUTTONDOWN => {
            let y = hi(l);
            with(|a| {
                let y = y + a.side_sy;
                let label = a.px(28);
                if let Some(i) = a.thumbs.iter().position(|r| y >= r.top - a.px(4) && y < r.bottom + label) {
                    a.goto_page(i);
                }
            });
            sync();
        }
        _ => return DefWindowProcW(h, m, w, l),
    }
    0
}

unsafe fn find_msg(h: HWND, m: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    match m {
        WM_PAINT => paint(h, |dc, rc| {
            with(|a| a.paint_find(dc, rc));
        }),
        WM_ERASEBKGND => return 1,
        WM_CTLCOLOREDIT => return ctl_color(w),
        WM_LBUTTONDOWN => {
            let (x, y) = (lo(l), hi(l));
            let id = with(|a| {
                let [_, p, n, c] = a.find_rects();
                [(F_PREV, p), (F_NEXT, n), (F_CLOSE, c)].into_iter().find(|(_, r)| inside(r, x, y)).map(|(id, _)| id)
            });
            if let Some(Some(id)) = id {
                command(id);
            }
        }
        _ => return DefWindowProcW(h, m, w, l),
    }
    0
}

// ---------- registry ----------

fn reg_set(key: &str, name: &str, val: &str) -> Result<(), String> {
    let (k, n, v) = (wide(key), wide(name), wide(val));
    let np = if name.is_empty() { null() } else { n.as_ptr() };
    let r = unsafe { RegSetKeyValueW(HKEY_CURRENT_USER, k.as_ptr(), np, REG_SZ, v.as_ptr() as _, (v.len() * 2) as u32) };
    if r == 0 {
        Ok(())
    } else {
        Err(format!("Gagal menulis HKCU\\{key} (kode {r})"))
    }
}

fn reg_del_value(key: &str, name: &str) -> Result<(), String> {
    let (k, n) = (wide(key), wide(name));
    let r = unsafe { RegDeleteKeyValueW(HKEY_CURRENT_USER, k.as_ptr(), n.as_ptr()) };
    if r == 0 || r == ERROR_FILE_NOT_FOUND {
        Ok(())
    } else {
        Err(format!("Gagal menghapus HKCU\\{key}\\{name} (kode {r})"))
    }
}

fn register(on: bool) -> Result<(), String> {
    const PROGID: &str = r"Software\Classes\PrayDF.PDF";
    const APPKEY: &str = r"Software\Classes\Applications\praydf.exe";
    const OPENWITH: &str = r"Software\Classes\.pdf\OpenWithProgids";
    const PROTO: &str = r"Software\Classes\praydf";
    // Default apps: Capabilities + RegisteredApplications (Settings > Apps > Default apps).
    const CAPS: &str = r"Software\PrayDF\Capabilities";
    const CAPS_ROOT: &str = r"Software\PrayDF";
    const REGAPP: &str = r"Software\RegisteredApplications";
    if on {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        let cmd = format!("\"{}\" \"%1\"", exe.display());
        reg_set(PROGID, "", "PDF Document")?;
        reg_set(&format!(r"{PROGID}\DefaultIcon"), "", &format!("\"{}\",0", exe.display()))?;
        reg_set(&format!(r"{PROGID}\shell\open\command"), "", &cmd)?;
        reg_set(OPENWITH, "PrayDF.PDF", "")?;
        reg_set(&format!(r"{APPKEY}\shell\open\command"), "", &cmd)?;
        reg_set(&format!(r"{APPKEY}\SupportedTypes"), ".pdf", "")?;
        reg_set(PROTO, "", "URL:PrayDF Protocol")?;
        reg_set(PROTO, "URL Protocol", "")?;
        reg_set(&format!(r"{PROTO}\shell\open\command"), "", &cmd)?;
        // Capabilities: Settings > Apps > Default apps menampilkan PrayDF.
        reg_set(CAPS, "ApplicationName", "PrayDF")?;
        reg_set(CAPS, "ApplicationDescription", "Fast and private PDF viewer")?;
        reg_set(CAPS, "ApplicationIcon", &format!("\"{}\",0", exe.display()))?;
        reg_set(&format!(r"{CAPS}\FileAssociations"), ".pdf", "PrayDF.PDF")?;
        reg_set(&format!(r"{CAPS}\URLAssociations"), "praydf", "praydf")?;
        reg_set(REGAPP, "PrayDF", r"Software\PrayDF\Capabilities")?;
    } else {
        unsafe {
            for k in [PROGID, APPKEY, PROTO, CAPS_ROOT] {
                let r = RegDeleteTreeW(HKEY_CURRENT_USER, wide(k).as_ptr());
                if r != 0 && r != ERROR_FILE_NOT_FOUND {
                    return Err(format!("Gagal menghapus HKCU\\{k} (kode {r})"));
                }
            }
        }
        reg_del_value(OPENWITH, "PrayDF.PDF")?;
        reg_del_value(REGAPP, "PrayDF")?;
    }
    unsafe { SHChangeNotify(SHCNE_ASSOCCHANGED as _, SHCNF_IDLIST, null(), null()) };
    Ok(())
}

// ---------- start ----------

fn load_pdfium() -> Option<&'static Pdfium> {
    let dir = std::env::current_exe().ok()?.parent()?.to_path_buf();
    // Di samping exe; atau native/pdfium/bin saat dijalankan dari target/release.
    let bindings = [dir.join("pdfium.dll"), dir.join(r"..\..\pdfium\bin\pdfium.dll")]
        .iter()
        .find_map(|p| Pdfium::bind_to_library(p).ok())
        .or_else(|| Pdfium::bind_to_system_library().ok())?;
    Some(Box::leak(Box::new(Pdfium::new(bindings))))
}

type WndProc = unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM) -> LRESULT;

unsafe fn create(ex: u32, class: &str, style: u32, parent: HWND) -> HWND {
    let hinst = GetModuleHandleW(null());
    CreateWindowExW(ex, wide(class).as_ptr(), wide("").as_ptr(), style, 0, 0, 0, 0, parent, null_mut(), hinst, null())
}

unsafe fn run(lib: Option<&'static Pdfium>, start: Option<link::Target>) {
    let hinst = GetModuleHandleW(null());
    let s = GetDpiForSystem() as f32 / 96.0;
    let procs: [(&str, WndProc); 4] = [("DokMain", main_proc), ("DokPage", view_proc), ("DokSide", side_proc), ("DokFind", find_proc)];
    for (name, proc_) in procs {
        let cls = wide(name);
        let wc = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(proc_),
            hInstance: hinst,
            hCursor: LoadCursorW(null_mut(), IDC_ARROW),
            hIcon: LoadIconW(hinst, 1 as _), // ikon resource 1 dari praydf.rc
            lpszClassName: cls.as_ptr(),
            ..zeroed()
        };
        RegisterClassW(&wc);
    }
    let font_of = |px: f32, face: &str| {
        let f = wide(face);
        CreateFontW(-(px * s).round() as i32, 0, 0, 0, FW_NORMAL as i32, 0, 0, 0, DEFAULT_CHARSET as u32, OUT_DEFAULT_PRECIS as u32, CLIP_DEFAULT_PRECIS as u32, CLEARTYPE_QUALITY as u32, 0, f.as_ptr())
    };
    let font = font_of(14.0, "Segoe UI");
    let icons = font_of(16.0, "Segoe MDL2 Assets");
    APP.with(|c| {
        *c.borrow_mut() = Some(App {
            lib,
            main: null_mut(),
            view: null_mut(),
            side: null_mut(),
            find: null_mut(),
            page_edit: null_mut(),
            find_edit: null_mut(),
            font,
            icons,
            s,
            doc: None,
            error: None,
            query: String::new(),
            zoom: 1.0,
            fit: None,
            next_fit: Fit::Page,
            sidebar: true,
            find_open: false,
            hover: 0,
            mw: 0,
            vw: 0,
            vh: 0,
            sx: 0,
            sy: 0,
            doc_w: 0,
            doc_h: 0,
            pages: Vec::new(),
            sh: 0,
            side_sy: 0,
            side_doc_h: 0,
            thumbs: Vec::new(),
            page: 0,
            shown_page: usize::MAX,
            title: String::new(),
        })
    });
    let (cx, cy) = (GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN));
    let main = CreateWindowExW(
        WS_EX_ACCEPTFILES,
        wide("DokMain").as_ptr(),
        wide("PrayDF").as_ptr(),
        WS_OVERLAPPEDWINDOW | WS_CLIPCHILDREN,
        CW_USEDEFAULT,
        CW_USEDEFAULT,
        cx * 3 / 4,
        cy * 17 / 20,
        null_mut(),
        null_mut(),
        hinst,
        null(),
    );
    if main.is_null() {
        msgbox(null_mut(), "Jendela tidak bisa dibuat.");
        return;
    }
    let dark: i32 = 1;
    DwmSetWindowAttribute(main, 20, &dark as *const i32 as _, 4); // DWMWA_USE_IMMERSIVE_DARK_MODE
    let side = create(0, "DokSide", WS_CHILD | WS_VSCROLL | WS_CLIPSIBLINGS, main);
    let view = create(WS_EX_ACCEPTFILES, "DokPage", WS_CHILD | WS_VISIBLE | WS_VSCROLL | WS_HSCROLL | WS_CLIPSIBLINGS, main);
    let find = create(0, "DokFind", WS_CHILD | WS_CLIPSIBLINGS | WS_CLIPCHILDREN, main);
    let page_edit = create(0, "EDIT", WS_CHILD | (ES_CENTER | ES_NUMBER | ES_AUTOHSCROLL) as u32, main);
    let find_edit = create(0, "EDIT", WS_CHILD | WS_VISIBLE | ES_AUTOHSCROLL as u32, find);
    for hw in [side, view] {
        SetWindowTheme(hw, wide("DarkMode_Explorer").as_ptr(), null());
    }
    for e in [page_edit, find_edit] {
        SendMessageW(e, WM_SETFONT, font as WPARAM, 0);
    }
    SendMessageW(page_edit, EM_SETLIMITTEXT, 6, 0);
    with(|a| {
        (a.main, a.view, a.side, a.find, a.page_edit, a.find_edit) = (main, view, side, find, page_edit, find_edit);
    });
    layout_children();
    let mut url = None;
    match start {
        Some(link::Target::File(p)) => open_path(&p),
        Some(link::Target::Url(u)) => {
            with(|a| a.error = Some(format!("Mengunduh…\n{u}")));
            sync();
            url = Some(u);
        }
        None => {
            if lib.is_none() {
                with(|a| a.error = Some("pdfium.dll tidak ditemukan.\nLetakkan pdfium.dll di samping praydf.exe.".into()));
            }
            sync();
        }
    }
    ShowWindow(main, SW_SHOWNORMAL);
    SetFocus(view);
    if let Some(u) = url {
        // Unduh di thread lain agar jendela tetap responsif; hasil dikirim lewat WM_DOWNLOADED.
        let target = main as isize;
        std::thread::spawn(move || {
            unsafe { CoInitializeEx(null(), COINIT_APARTMENTTHREADED as u32) };
            let r = link::download(&u).map_err(|e| format!("{e}\n{u}"));
            *DOWNLOAD.lock().unwrap_or_else(|e| e.into_inner()) = Some(r);
            unsafe { PostMessageW(target as HWND, WM_DOWNLOADED, 0, 0) };
        });
    }
    let mut msg: MSG = zeroed();
    while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
        if msg.message == WM_KEYDOWN {
            match catch_unwind(AssertUnwindSafe(|| on_key(&msg))) {
                Ok(true) => continue,
                Ok(false) => {}
                Err(e) => {
                    report_panic(e);
                    continue;
                }
            }
        }
        TranslateMessage(&msg);
        DispatchMessageW(&msg);
    }
}

fn main() {
    let arg = std::env::args_os().nth(1);
    if let Some(flag @ ("--register" | "--unregister")) = arg.as_deref().and_then(|a| a.to_str()) {
        unsafe { AttachConsole(ATTACH_PARENT_PROCESS) };
        let on = flag == "--register";
        match register(on) {
            Ok(()) => println!("{}", if on { "PrayDF terdaftar untuk .pdf (HKCU)." } else { "Pendaftaran PrayDF dihapus." }),
            Err(e) => {
                eprintln!("{e}");
                std::process::exit(1);
            }
        }
        return;
    }
    let start = match arg {
        Some(a) if link::is_link(&a.to_string_lossy()) => {
            let target = a.to_str().ok_or_else(|| "Tautan rusak.".to_string()).and_then(link::parse);
            match target {
                Ok(link::Target::File(p)) if !p.is_file() => {
                    msgbox(null_mut(), &format!("File tidak ditemukan:\n{}", p.display()));
                    return;
                }
                Ok(t) => Some(t),
                Err(e) => {
                    msgbox(null_mut(), &format!("Tautan praydf ditolak.\n{e}"));
                    return;
                }
            }
        }
        a => a.map(|p| link::Target::File(PathBuf::from(p))),
    };
    unsafe { SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_SYSTEM_AWARE) };
    let lib = load_pdfium();
    unsafe { run(lib, start) };
}
