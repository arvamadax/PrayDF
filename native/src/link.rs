// Tautan `praydf:<target ter-encode>` dari browser. Batas keamanan: halaman web mana pun
// bisa membuat tautan ini, jadi hanya http(s) dan file .pdf lokal berhuruf drive yang lolos.
use std::path::PathBuf;

const MAX_LEN: usize = 4096;
const PREFIX: &str = "praydf:";

#[derive(Debug, PartialEq)]
pub enum Target {
    Url(String),
    File(PathBuf),
}

pub fn is_link(arg: &str) -> bool {
    arg.get(..PREFIX.len()).is_some_and(|p| p.eq_ignore_ascii_case(PREFIX))
}

fn percent_decode(s: &str) -> Option<String> {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' {
            let h = std::str::from_utf8(b.get(i + 1..i + 3)?).ok()?;
            out.push(u8::from_str_radix(h, 16).ok()?);
            i += 3;
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

/// Validasi murni (tanpa I/O). Keberadaan file dicek pemanggil.
pub fn parse(arg: &str) -> Result<Target, String> {
    let bad = |m: &str| Err(m.to_string());
    if !is_link(arg) {
        return bad("Bukan tautan praydf:.");
    }
    let raw = &arg[PREFIX.len()..];
    let raw = raw.strip_prefix("//").unwrap_or(raw);
    if raw.len() > MAX_LEN * 3 {
        return bad("Tautan terlalu panjang.");
    }
    let Some(t) = percent_decode(raw) else { return bad("Tautan rusak (encoding tidak valid).") };
    if t.is_empty() {
        return bad("Tautan kosong.");
    }
    if t.len() > MAX_LEN {
        return bad("Tautan terlalu panjang.");
    }
    if t.chars().any(char::is_control) {
        return bad("Tautan berisi karakter kontrol.");
    }
    let lower = t.to_ascii_lowercase();
    for scheme in ["http://", "https://"] {
        if let Some(rest) = lower.strip_prefix(scheme) {
            let host = rest.split(['/', '?', '#']).next().unwrap_or("");
            if host.is_empty() || t.contains(char::is_whitespace) {
                return bad("Alamat web tidak valid.");
            }
            return Ok(Target::Url(t));
        }
    }
    let path = if lower.starts_with("file:") {
        // Hanya file:///C:/... ; file://host/... dan file:////host adalah UNC.
        let Some(rest) = t.get(8..).filter(|_| lower.starts_with("file:///")) else {
            return bad("Tautan file dari komputer lain ditolak.");
        };
        let Some(p) = percent_decode(rest) else { return bad("Tautan rusak (encoding tidak valid).") };
        p
    } else {
        t
    };
    local_pdf(&path.replace('/', "\\")).map(Target::File)
}

fn local_pdf(p: &str) -> Result<PathBuf, String> {
    let b = p.as_bytes();
    // Wajib "X:\..." : menolak UNC (\\server, bisa membocorkan hash NTLM), \\?\, dan path relatif.
    let drive = b.len() > 3 && b[0].is_ascii_alphabetic() && b[1] == b':' && b[2] == b'\\';
    if !drive || p[2..].contains(':') || p.chars().any(char::is_control) {
        return Err("Hanya file PDF di drive lokal (mis. C:\\...) yang bisa dibuka.".into());
    }
    if !p.to_ascii_lowercase().ends_with(".pdf") {
        return Err("Hanya file .pdf yang bisa dibuka.".into());
    }
    Ok(PathBuf::from(p))
}

/// Nama file aman dari URL, selalu berakhiran .pdf.
pub fn file_name(url: &str) -> String {
    let path = url.split(['?', '#']).next().unwrap_or("");
    let last = path.rsplit('/').next().unwrap_or("");
    let last = percent_decode(last).unwrap_or_default();
    let mut s: String = last
        .chars()
        .map(|c| if c.is_alphanumeric() || "-_. ".contains(c) { c } else { '_' })
        .take(100)
        .collect();
    s = s.trim_matches(|c| c == '.' || c == ' ').to_string();
    if s.is_empty() {
        s = "dokumen".into();
    }
    if !s.to_ascii_lowercase().ends_with(".pdf") {
        s.push_str(".pdf");
    }
    s
}

/// Unduh ke %TEMP%\praydf\ (sinkron).
pub fn download(url: &str) -> Result<PathBuf, String> {
    use windows_sys::Win32::System::Com::Urlmon::URLDownloadToFileW;
    let dir = std::env::temp_dir().join("praydf");
    std::fs::create_dir_all(&dir).map_err(|e| format!("Folder sementara gagal dibuat: {e}"))?;
    let name = file_name(url);
    let mut dst = dir.join(&name);
    // File lama yang masih dibuka jendela lain tidak bisa ditimpa: pakai nama baru.
    if dst.exists() && std::fs::remove_file(&dst).is_err() {
        let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_millis());
        dst = dir.join(format!("{t}-{name}"));
    }
    let (u, d) = (crate::wide(url), crate::wide(&dst));
    let hr = unsafe { URLDownloadToFileW(std::ptr::null_mut(), u.as_ptr(), d.as_ptr(), 0, std::ptr::null_mut()) };
    if hr != 0 {
        return Err(format!("Unduhan gagal (kode 0x{hr:08X})."));
    }
    Ok(dst)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn file(p: &str) -> Result<Target, String> {
        Ok(Target::File(PathBuf::from(p)))
    }

    #[test]
    fn accepts() {
        assert_eq!(parse("praydf:https%3A%2F%2Fexample.com%2Fa.pdf"), Ok(Target::Url("https://example.com/a.pdf".into())));
        assert_eq!(parse("PRAYDF://http%3A%2F%2Fexample.com%2Fx%3Fid%3D1"), Ok(Target::Url("http://example.com/x?id=1".into())));
        assert_eq!(parse("praydf:C%3A%5CUsers%5Cx%5CDownloads%5Ca.pdf"), file(r"C:\Users\x\Downloads\a.pdf"));
        assert_eq!(parse("praydf:D%3A%5CDok%5CB.PDF"), file(r"D:\Dok\B.PDF"));
        assert_eq!(parse("praydf:file%3A%2F%2F%2FC%3A%2FDok%2Fa%2520b.pdf"), file(r"C:\Dok\a b.pdf"));
    }

    #[test]
    fn rejects() {
        let long = format!("praydf:https%3A%2F%2Fexample.com%2F{}", "a".repeat(MAX_LEN));
        for bad in [
            "praydf:javascript%3Aalert(1)",
            "praydf:ftp%3A%2F%2Fexample.com%2Fa.pdf",
            "praydf:C%3A%5CWindows%5Cnotepad.exe",
            "praydf:%5C%5Chost%5Cshare%5Ca.pdf",
            "praydf://%5C%5C%3F%5CC%3A%5Ca.pdf",
            "praydf:file%3A%2F%2Fhost%2Fshare%2Fa.pdf",
            "praydf:file%3A%2F%2F%2F%2Fhost%2Fshare%2Fa.pdf",
            "praydf:file%3A%2F%2F%2FC%3A%2Fa.exe",
            "praydf:Downloads%5Ca.pdf",
            "praydf:C%3Aa.pdf",
            "praydf:C%3A%5Ca.pdf%3Astream",
            "praydf:C%3A%5Ca%0A.pdf",
            "praydf:https%3A%2F%2F%2Fa.pdf",
            "praydf:%ZZ",
            "praydf:",
            "",
            long.as_str(),
        ] {
            assert!(parse(bad).is_err(), "harus ditolak: {bad}");
        }
    }

    #[test]
    fn names() {
        assert_eq!(file_name("https://x.com/dir/Laporan%20Akhir.pdf?v=2"), "Laporan Akhir.pdf");
        assert_eq!(file_name("https://x.com/get?id=..%2F..%2Fevil"), "get.pdf");
        assert_eq!(file_name("https://x.com/"), "dokumen.pdf");
        assert_eq!(file_name("https://x.com/a..%5C..%5Cb.exe"), "a.._.._b.exe.pdf");
    }
}
