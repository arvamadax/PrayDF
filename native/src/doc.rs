// Dokumen PDF: buka, render ke BGRA dengan cache kecil, cari teks, rotasi.
use pdfium_render::prelude::*;
use std::path::{Path, PathBuf};

pub struct Bmp {
    key: (usize, i32, i32, u8), // halaman, lebar & tinggi kotak yang diminta, rotasi
    pub w: i32,
    pub h: i32,
    pub px: Vec<u8>, // BGRA, baris dari atas
}

/// LRU sederhana berbatas jumlah byte.
struct Cache {
    items: Vec<Bmp>,
    budget: usize,
}

impl Cache {
    fn get(&mut self, key: (usize, i32, i32, u8)) -> Option<usize> {
        let i = self.items.iter().position(|b| b.key == key)?;
        let b = self.items.remove(i);
        self.items.push(b);
        Some(self.items.len() - 1)
    }
    fn put(&mut self, b: Bmp) -> usize {
        self.items.push(b);
        let mut total: usize = self.items.iter().map(|b| b.px.len()).sum();
        // Yang baru dimasukkan (terakhir) selalu disimpan.
        while total > self.budget && self.items.len() > 1 {
            total -= self.items.remove(0).px.len();
        }
        self.items.len() - 1
    }
}

pub struct Hit {
    pub page: usize,
    pub rects: Vec<[f32; 4]>, // l, t, r, b ternormalisasi 0..1, halaman tanpa rotasi tampilan
}

pub struct Doc {
    pdf: PdfDocument<'static>,
    pub path: PathBuf,
    pub name: String,
    sizes: Vec<(f32, f32)>, // ukuran halaman dalam point (sudah termasuk /Rotate)
    pub rot: u8,            // putaran tampilan, kelipatan 90° searah jarum jam
    pages: Cache,
    thumbs: Cache,
    pub hits: Vec<Hit>,
    pub cur: usize,
}

fn rotation(rot: u8) -> PdfPageRenderRotation {
    match rot % 4 {
        1 => PdfPageRenderRotation::Degrees90,
        2 => PdfPageRenderRotation::Degrees180,
        3 => PdfPageRenderRotation::Degrees270,
        _ => PdfPageRenderRotation::None,
    }
}

impl Doc {
    pub fn open(lib: &'static Pdfium, path: &Path) -> Result<Doc, String> {
        let pdf = lib.load_pdf_from_file(path, None).map_err(|e| match e {
            PdfiumError::PdfiumLibraryInternalError(PdfiumInternalError::PasswordError) => {
                "File ini dilindungi kata sandi.".to_string()
            }
            PdfiumError::PdfiumLibraryInternalError(PdfiumInternalError::FormatError) => {
                "File ini bukan PDF atau sudah rusak.".to_string()
            }
            PdfiumError::IoError(e) => format!("File tidak bisa dibaca: {e}"),
            e => format!("PDF tidak bisa dibuka: {e:?}"),
        })?;
        let sizes: Vec<(f32, f32)> = pdf
            .pages()
            .iter()
            .map(|p| (p.width().value.max(1.0), p.height().value.max(1.0)))
            .collect();
        if sizes.is_empty() {
            return Err("PDF ini tidak punya halaman.".into());
        }
        let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        Ok(Doc {
            pdf,
            path: path.to_path_buf(),
            name,
            sizes,
            rot: 0,
            pages: Cache { items: Vec::new(), budget: 48 << 20 },
            thumbs: Cache { items: Vec::new(), budget: 8 << 20 },
            hits: Vec::new(),
            cur: 0,
        })
    }

    pub fn count(&self) -> usize {
        self.sizes.len()
    }

    /// Ukuran halaman dalam point, tanpa rotasi tampilan.
    pub fn raw_size(&self, i: usize) -> (f32, f32) {
        self.sizes.get(i).copied().unwrap_or((612.0, 792.0))
    }

    /// Ukuran halaman dalam point setelah rotasi tampilan.
    pub fn size(&self, i: usize) -> (f32, f32) {
        let (w, h) = self.raw_size(i);
        if self.rot % 2 == 1 { (h, w) } else { (w, h) }
    }

    /// Render halaman `i`, lebar asli (sebelum rotasi) `w` px. Dipakai juga oleh cetak.
    pub fn render(&self, i: usize, w: i32, rot: u8, printing: bool) -> Option<Bmp> {
        let page = self.pdf.pages().get(i as PdfPageIndex).ok()?;
        let cfg = PdfRenderConfig::new()
            .set_target_width(w.max(1))
            .rotate(rotation(rot), false)
            .set_format(PdfBitmapFormat::BGRA)
            .set_reverse_byte_order(false) // GDI butuh BGRA apa adanya
            .render_form_data(true)
            .set_clear_color(PdfColor::WHITE)
            .use_print_quality(printing);
        let bmp = page.render_with_config(&cfg).ok()?;
        let (bw, bh) = (bmp.width() as i32, bmp.height() as i32);
        let px = bmp.as_raw_bytes();
        (bw > 0 && bh > 0 && px.len() >= (bw * bh * 4) as usize)
            .then_some(Bmp { key: (i, w, 0, rot), w: bw, h: bh, px })
    }

    /// Bitmap halaman untuk kotak tampilan `w`×`h` (sudah dirotasi), dari cache bila ada.
    pub fn bitmap(&mut self, i: usize, w: i32, h: i32, thumb: bool) -> Option<&Bmp> {
        let key = (i, w, h, self.rot);
        let c = if thumb { &mut self.thumbs } else { &mut self.pages };
        let idx = match c.get(key) {
            Some(idx) => idx,
            None => {
                let unrot_w = if self.rot % 2 == 1 { h } else { w };
                let mut b = self.render(i, unrot_w, self.rot, false)?;
                b.key = key;
                let c = if thumb { &mut self.thumbs } else { &mut self.pages };
                c.put(b)
            }
        };
        let c = if thumb { &self.thumbs } else { &self.pages };
        c.items.get(idx)
    }

    /// Cari `q` di semua halaman; mengisi `hits` (urut per halaman).
    pub fn search(&mut self, q: &str) {
        self.hits.clear();
        self.cur = 0;
        if q.trim().is_empty() {
            return;
        }
        let probe = PdfRenderConfig::new().set_target_width(10000);
        for (i, page) in self.pdf.pages().iter().enumerate() {
            let Ok(text) = page.text() else { continue };
            let Ok(search) = text.search(q, &PdfSearchOptions::new()) else { continue };
            let (pw, ph) = self.raw_size(i);
            let (nw, nh) = (10000.0f32, 10000.0 * ph / pw);
            for seg in search.iter(PdfSearchDirection::SearchForward) {
                let rects = seg
                    .iter()
                    .filter_map(|s| {
                        let r = s.bounds();
                        let a = page.points_to_pixels(r.left(), r.top(), &probe).ok()?;
                        let b = page.points_to_pixels(r.right(), r.bottom(), &probe).ok()?;
                        let (x0, x1) = (a.0.min(b.0) as f32 / nw, a.0.max(b.0) as f32 / nw);
                        let (y0, y1) = (a.1.min(b.1) as f32 / nh, a.1.max(b.1) as f32 / nh);
                        Some([x0, y0, x1, y1])
                    })
                    .collect();
                self.hits.push(Hit { page: i, rects });
            }
        }
    }
}

/// Putar kotak ternormalisasi (l,t,r,b) sebanyak `rot`×90° searah jarum jam.
pub fn rotate_rect(r: [f32; 4], rot: u8) -> [f32; 4] {
    let [l, t, rr, b] = r;
    match rot % 4 {
        1 => [1.0 - b, l, 1.0 - t, rr],
        2 => [1.0 - rr, 1.0 - b, 1.0 - l, 1.0 - t],
        3 => [t, 1.0 - rr, b, 1.0 - l],
        _ => r,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rotate_rect_quarter_turns() {
        let r = [0.1, 0.2, 0.3, 0.5];
        let mut x = r;
        for _ in 0..4 {
            x = rotate_rect(x, 1);
        }
        for k in 0..4 {
            assert!((x[k] - r[k]).abs() < 1e-6);
        }
        // 90° searah jarum jam: pojok kiri-atas pindah ke kanan-atas.
        assert_eq!(rotate_rect([0.0, 0.0, 0.25, 0.5], 1), [0.5, 0.0, 1.0, 0.25]);
    }
}
