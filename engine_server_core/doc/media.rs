//! docx 包内媒体读取：图片按需从原始归档里直取，不额外落库、不产生副本。

use std::collections::HashMap;
use std::io::{Cursor, Read};

use quick_xml::events::Event;
use quick_xml::Reader;
use zip::ZipArchive;

use crate::core::error::EngineError;

/// 单张图片解压后的安全上限（防压缩炸弹）
pub const MAX_MEDIA_BYTES: u64 = 24 * 1024 * 1024;

#[derive(Clone, Debug)]
pub struct MediaEntry {
    /// 规范化后的包内路径，如 `word/media/image1.png`
    pub path: String,
    pub bytes: Vec<u8>,
    pub mime: &'static str,
}

pub fn mime_of(path: &str) -> &'static str {
    let lower = path.to_ascii_lowercase();
    match lower.rsplit('.').next().unwrap_or_default() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "bmp" => "image/bmp",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "tif" | "tiff" => "image/tiff",
        "emf" => "image/emf",
        "wmf" => "image/wmf",
        _ => "application/octet-stream",
    }
}

/// 只允许读 `word/media/` 下的条目，且路径不能跳出该目录。
pub fn normalize_media_path(raw: &str) -> Result<String, EngineError> {
    let path = raw.trim().trim_start_matches('/');
    if path.is_empty() {
        return Err(EngineError::invalid_input("图片路径为空"));
    }
    if path.contains('\0') || path.contains('\\') {
        return Err(EngineError::invalid_input("图片路径非法"));
    }

    let mut parts: Vec<&str> = Vec::new();
    for seg in path.split('/') {
        match seg {
            "" | "." => continue,
            ".." => {
                return Err(EngineError::invalid_input("图片路径非法"));
            }
            s => parts.push(s),
        }
    }
    if parts.len() < 3 || parts[0] != "word" || parts[1] != "media" {
        return Err(EngineError::invalid_input("只允许读取 word/media 下的图片"));
    }
    Ok(parts.join("/"))
}

/// 解析 `word/_rels/document.xml.rels`，得到 `rId -> 包内路径`。
/// 只保留图片关系；拿不到关系表时返回空表（正文照常渲染，只是图不显示）。
pub fn load_document_rels(docx: &[u8]) -> HashMap<String, String> {
    let Ok(mut archive) = ZipArchive::new(Cursor::new(docx)) else {
        return HashMap::new();
    };
    let Ok(entry) = archive.by_name("word/_rels/document.xml.rels") else {
        return HashMap::new();
    };
    if entry.size() > 4 * 1024 * 1024 {
        return HashMap::new();
    }
    let mut buf = Vec::new();
    if entry.take(4 * 1024 * 1024 + 1).read_to_end(&mut buf).is_err() {
        return HashMap::new();
    }

    let mut map = HashMap::new();
    let mut reader = Reader::from_reader(&buf[..]);
    reader.config_mut().trim_text(true);
    let mut ev = Vec::new();
    loop {
        match reader.read_event_into(&mut ev) {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                if e.local_name().as_ref() != b"Relationship" {
                    continue;
                }
                let mut id = String::new();
                let mut target = String::new();
                for attr in e.attributes().flatten() {
                    let key = attr.key.as_ref();
                    let val = attr.unescape_value().unwrap_or_default().to_string();
                    if key.ends_with(b"Id") {
                        id = val;
                    } else if key.ends_with(b"Target") {
                        target = val;
                    }
                }
                if id.is_empty() || target.is_empty() {
                    continue;
                }
                if let Some(path) = resolve_target(&target) {
                    map.insert(id, path);
                }
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
        ev.clear();
    }
    map
}

/// rels 里的 Target 相对 `word/` 目录（也可能是 `/word/media/x.png` 这类绝对写法）。
fn resolve_target(target: &str) -> Option<String> {
    let t = target.trim().trim_start_matches('/');
    if t.is_empty() || !is_media_like(t) {
        return None;
    }
    let full = if t.starts_with("word/") {
        t.to_string()
    } else {
        format!("word/{t}")
    };
    normalize_media_path(&full).ok()
}

fn is_media_like(path: &str) -> bool {
    mime_of(path).starts_with("image/")
}

/// 从 docx 里取一张图片。
pub fn read_media(docx: &[u8], raw_path: &str) -> Result<Option<MediaEntry>, EngineError> {
    let path = normalize_media_path(raw_path)?;
    let mut archive = ZipArchive::new(Cursor::new(docx))
        .map_err(|_| EngineError::invalid_input("打开 .docx 归档失败"))?;
    let entry = match archive.by_name(&path) {
        Ok(f) => f,
        Err(zip::result::ZipError::FileNotFound) => return Ok(None),
        Err(_) => return Err(EngineError::invalid_input("读取图片条目失败")),
    };
    if entry.size() > MAX_MEDIA_BYTES {
        return Err(EngineError::invalid_input("图片体积超过限制"));
    }
    let mut bytes = Vec::new();
    entry
        .take(MAX_MEDIA_BYTES.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|_| EngineError::invalid_input("解压图片失败"))?;
    if (bytes.len() as u64) > MAX_MEDIA_BYTES {
        return Err(EngineError::invalid_input("图片实际体积超限"));
    }
    let mime = mime_of(&path);
    Ok(Some(MediaEntry { path, bytes, mime }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use zip::write::SimpleFileOptions;

    fn pkg() -> Vec<u8> {
        let mut buf = Vec::new();
        {
            let mut zw = zip::ZipWriter::new(Cursor::new(&mut buf));
            let opts = SimpleFileOptions::default();
            zw.start_file("word/media/image1.png", opts).unwrap();
            zw.write_all(b"\x89PNG fake").unwrap();
            zw.start_file("word/_rels/document.xml.rels", opts).unwrap();
            zw.write_all(
                br#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId5" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="media/image1.png"/>
  <Relationship Id="rId9" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/header" Target="header1.xml"/>
</Relationships>"#,
            )
            .unwrap();
            zw.finish().unwrap();
        }
        buf
    }

    #[test]
    fn rels_map_rId_to_package_path_and_skip_non_image() {
        let map = load_document_rels(&pkg());
        assert_eq!(map.get("rId5").map(String::as_str), Some("word/media/image1.png"));
        assert!(map.get("rId9").is_none(), "非图片关系不入表");
    }

    #[test]
    fn read_media_returns_bytes_and_mime() {
        let m = read_media(&pkg(), "word/media/image1.png").unwrap().unwrap();
        assert_eq!(m.mime, "image/png");
        assert_eq!(m.bytes, b"\x89PNG fake");
        assert!(read_media(&pkg(), "word/media/none.png").unwrap().is_none());
    }

    #[test]
    fn media_path_cannot_escape_package() {
        assert!(normalize_media_path("word/media/../../secret").is_err());
        assert!(normalize_media_path("word/document.xml").is_err());
        assert!(normalize_media_path("").is_err());
        assert_eq!(
            normalize_media_path("/word/media/image1.png").unwrap(),
            "word/media/image1.png"
        );
    }
}
