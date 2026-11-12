// Cetak: PrintDlgW -> StartDocW -> render tiap halaman sesuai DPI printer -> StretchDIBits.
use crate::doc::Doc;
use std::mem::{size_of, zeroed};
use windows_sys::Win32::Foundation::{GlobalFree, HWND};
use windows_sys::Win32::Graphics::Gdi::*;
use windows_sys::Win32::Storage::Xps::{AbortDoc, EndDoc, EndPage, StartDocW, StartPage, DOCINFOW};
use windows_sys::Win32::UI::Controls::Dialogs::*;

pub struct Job {
    dc: HDC,
    from: usize,
    to: usize,
}

/// Tampilkan dialog cetak. `None` bila dibatalkan.
pub fn dialog(owner: HWND, pages: usize) -> Option<Job> {
    unsafe {
        let n = pages.min(u16::MAX as usize) as u16;
        let mut pd: PRINTDLGW = zeroed();
        pd.lStructSize = size_of::<PRINTDLGW>() as u32;
        pd.hwndOwner = owner;
        pd.Flags = PD_RETURNDC | PD_NOSELECTION | PD_USEDEVMODECOPIESANDCOLLATE;
        pd.nMinPage = 1;
        pd.nMaxPage = n;
        pd.nFromPage = 1;
        pd.nToPage = n;
        pd.nCopies = 1;
        let ok = PrintDlgW(&mut pd) != 0;
        if !pd.hDevMode.is_null() {
            GlobalFree(pd.hDevMode);
        }
        if !pd.hDevNames.is_null() {
            GlobalFree(pd.hDevNames);
        }
        if !ok || pd.hDC.is_null() {
            return None;
        }
        let (from, to) = if pd.Flags & PD_PAGENUMS != 0 {
            (pd.nFromPage.max(1) as usize, (pd.nToPage as usize).min(pages))
        } else {
            (1, pages)
        };
        Some(Job { dc: pd.hDC, from, to })
    }
}

/// Cetak halaman `from..=to`; DC printer selalu dilepas.
pub fn run(job: Job, doc: &Doc) -> Result<(), String> {
    unsafe {
        let dc = job.dc;
        let name: Vec<u16> = doc.name.encode_utf16().chain([0]).collect();
        let mut di: DOCINFOW = zeroed();
        di.cbSize = size_of::<DOCINFOW>() as i32;
        di.lpszDocName = name.as_ptr();
        if StartDocW(dc, &di) <= 0 {
            DeleteDC(dc);
            return Err("Printer menolak memulai dokumen.".into());
        }
        let (pw, ph) = (GetDeviceCaps(dc, HORZRES as i32), GetDeviceCaps(dc, VERTRES as i32));
        let dpi = GetDeviceCaps(dc, LOGPIXELSX as i32).max(72) as f32;
        let mut result = Ok(());
        for i in job.from.saturating_sub(1)..job.to {
            if StartPage(dc) <= 0 {
                result = Err("Printer menolak halaman.".into());
                break;
            }
            let (w, h) = doc.raw_size(i);
            // Halaman landscape di kertas portrait (atau sebaliknya) diputar 90°.
            let rot = u8::from((w > h) != (pw > ph));
            let (dw, dh) = if rot == 1 { (h, w) } else { (w, h) };
            let s = (pw as f32 / dw).min(ph as f32 / dh);
            let (ow, oh) = ((dw * s) as i32, (dh * s) as i32);
            // Render maksimal 300 dpi agar RAM tetap kecil; printer memperbesar sisanya.
            let k = (300.0 / dpi).min(1.0);
            let unrot_w = (if rot == 1 { oh } else { ow }) as f32 * k;
            if let Some(b) = doc.render(i, unrot_w as i32, rot, true) {
                crate::blit(dc, (pw - ow) / 2, (ph - oh) / 2, ow, oh, &b);
            }
            if EndPage(dc) <= 0 {
                result = Err("Printer gagal menyelesaikan halaman.".into());
                break;
            }
        }
        if result.is_ok() {
            EndDoc(dc);
        } else {
            AbortDoc(dc);
        }
        DeleteDC(dc);
        result
    }
}
