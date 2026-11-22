use std::collections::{HashMap, HashSet};
use std::io::{Cursor, Read, Write};

use quick_xml::events::{BytesEnd, BytesStart, BytesText, Event};
use quick_xml::reader::Reader;
use quick_xml::writer::Writer;
use sha2::{Digest, Sha256};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::core::error::EngineError;
use crate::core::ids::encode_hex;
use crate::doc::diff::diff_text_spans;
use crate::doc::model::{
    DocNodeRecord, DocRefRecord, ExportMode, NodeKind, ParsedDocx, SpanOp, MAX_DOCX_UPLOAD_BYTES,
    MAX_TOTAL_UNCOMPRESSED_BYTES, MAX_XML_UNCOMPRESSED_BYTES, MAX_ZIP_ENTRIES,
};

const OLE2_MAGIC: [u8; 8] = [0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];

#[derive(Clone, Debug, Default)]
struct StyleMeta {
    name: String,
    outline_lvl: Option<u8>,
}

#[derive(Clone, Debug)]
pub struct ParagraphPatch {
    pub before_text: String,
    pub new_text: String,
    pub mode: ExportMode,
    pub author: String,
    pub date_iso: String,
}

pub fn validate_docx_bytes(bytes: &[u8]) -> Result<(), EngineError> {
    if bytes.is_empty() {
        return Err(EngineError::invalid_input("上传文件为空"));
    }
    if bytes.len() > MAX_DOCX_UPLOAD_BYTES {
        return Err(EngineError::invalid_input("文档大小超过 50 MB 上限"));
    }
    if bytes.starts_with(&OLE2_MAGIC) {
        return Err(EngineError::invalid_input(
            "检测到二进制 .doc/.wps 文件，暂不直接解析。请先用本机 Word 或 WPS「另存为 .docx」并核对版式后再上传。",
        ));
    }
    if !(bytes.starts_with(b"PK\x03\x04") || bytes.starts_with(b"PK\x05\x06")) {
        return Err(EngineError::invalid_input(
            "文件格式无效，仅支持标准 .docx（OOXML）文档",
        ));
    }

    let mut archive = ZipArchive::new(Cursor::new(bytes))
        .map_err(|_| EngineError::invalid_input(".docx 压缩包损坏或格式不完整"))?;
    if archive.len() > MAX_ZIP_ENTRIES {
        return Err(EngineError::invalid_input("压缩包条目数量异常超限"));
    }

    let mut total_uncompressed: u64 = 0;
    let mut has_document_xml = false;
    for idx in 0..archive.len() {
        let entry = archive
            .by_index(idx)
            .map_err(|_| EngineError::invalid_input("读取 .docx 内部条目失败"))?;
        total_uncompressed = total_uncompressed.saturating_add(entry.size());
        if total_uncompressed > MAX_TOTAL_UNCOMPRESSED_BYTES {
            return Err(EngineError::invalid_input("文档解压体积异常超限"));
        }
        if entry.name() == "word/document.xml" {
            if entry.size() > MAX_XML_UNCOMPRESSED_BYTES {
                return Err(EngineError::invalid_input("正文 XML 体积超过安全限制"));
            }
            has_document_xml = true;
        }
    }

    if !has_document_xml {
        return Err(EngineError::invalid_input(
            "未在压缩包内找到 word/document.xml，请确认文件是由 Word/WPS 导出的有效 .docx",
        ));
    }

    Ok(())
}

fn read_zip_entry_bounded(
    archive: &mut ZipArchive<Cursor<&[u8]>>,
    name: &str,
    max_bytes: u64,
) -> Result<Option<Vec<u8>>, EngineError> {
    let entry = match archive.by_name(name) {
        Ok(f) => f,
        Err(zip::result::ZipError::FileNotFound) => return Ok(None),
        Err(_) => return Err(EngineError::invalid_input("读取 .docx 内部文件失败")),
    };
    if entry.size() > max_bytes {
        return Err(EngineError::invalid_input("文档 XML 条目超过安全体积限制"));
    }
    let mut buf = Vec::new();
    entry
        .take(max_bytes.saturating_add(1))
        .read_to_end(&mut buf)
        .map_err(|_| EngineError::invalid_input("解压文档 XML 内容失败"))?;
    if (buf.len() as u64) > max_bytes {
        return Err(EngineError::invalid_input("文档 XML 实际解压体积超限"));
    }
    Ok(Some(buf))
}

pub fn parse_docx_ast(bytes: &[u8]) -> Result<ParsedDocx, EngineError> {
    validate_docx_bytes(bytes)?;
    let mut archive = ZipArchive::new(Cursor::new(bytes))
        .map_err(|_| EngineError::invalid_input("打开 .docx 归档失败"))?;

    let styles = match read_zip_entry_bounded(&mut archive, "word/styles.xml", MAX_XML_UNCOMPRESSED_BYTES)? {
        Some(style_bytes) => parse_styles_xml(&style_bytes),
        None => HashMap::new(),
    };

    let doc_bytes = read_zip_entry_bounded(&mut archive, "word/document.xml", MAX_XML_UNCOMPRESSED_BYTES)?
        .ok_or_else(|| EngineError::invalid_input("缺少 word/document.xml"))?;

    parse_document_xml(&doc_bytes, &styles)
}

fn parse_styles_xml(xml_bytes: &[u8]) -> HashMap<String, StyleMeta> {
    let mut map = HashMap::new();
    let mut reader = Reader::from_reader(xml_bytes);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();

    let mut cur_id: Option<String> = None;
    let mut cur_meta = StyleMeta::default();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                let local = e.local_name();
                match local.as_ref() {
                    b"style" => {
                        cur_id = attr_val(e, b"styleId");
                        cur_meta = StyleMeta::default();
                    }
                    b"name" if cur_id.is_some() => {
                        if let Some(v) = attr_val(e, b"val") {
                            cur_meta.name = v;
                        }
                    }
                    b"outlineLvl" if cur_id.is_some() => {
                        if let Some(v) = attr_val(e, b"val") {
                            if let Ok(lvl) = v.trim().parse::<u8>() {
                                cur_meta.outline_lvl = Some(lvl.saturating_add(1).min(6));
                            }
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::End(ref e)) => {
                if e.local_name().as_ref() == b"style" {
                    if let Some(id) = cur_id.take() {
                        map.insert(id, std::mem::take(&mut cur_meta));
                    }
                }
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
        buf.clear();
    }

    map
}

#[derive(Default)]
struct ParaCollector {
    p_ord: usize,
    para_id: Option<String>,
    bookmark: Option<String>,
    style_id: Option<String>,
    outline_lvl: Option<u8>,
    is_list: bool,
    text: String,
}

#[derive(Default)]
struct TableCollector {
    tbl_ord: usize,
    first_p_ord: Option<usize>,
    rows: Vec<Vec<String>>,
    cur_row: Vec<String>,
    cur_cell_lines: Vec<String>,
    in_cell: bool,
}

fn parse_document_xml(
    xml_bytes: &[u8],
    styles: &HashMap<String, StyleMeta>,
) -> Result<ParsedDocx, EngineError> {
    let mut reader = Reader::from_reader(xml_bytes);
    reader.config_mut().trim_text(false);
    let mut buf = Vec::new();

    let mut nodes: Vec<DocNodeRecord> = Vec::new();
    let mut refs: Vec<DocRefRecord> = Vec::new();
    let mut heading_stack: Vec<(u8, String, String)> = Vec::new();
    let mut anchor_seen: HashMap<String, usize> = HashMap::new();

    let mut in_body = false;
    let mut fallback_depth: usize = 0;
    let mut del_depth: usize = 0;
    let mut in_text_elem = false;
    let mut has_revisions = false;

    let mut next_p_ord: usize = 0;
    let mut next_tbl_ord: usize = 0;
    let mut table_count: usize = 0;
    let mut mandatory_count: usize = 0;

    let mut cur_para: Option<ParaCollector> = None;
    let mut cur_tbl: Option<TableCollector> = None;

    loop {
        let ev = reader
            .read_event_into(&mut buf)
            .map_err(|_| EngineError::invalid_input("word/document.xml 解析失败，XML 结构损坏"))?;

        match ev {
            Event::Eof => break,
            Event::Start(ref e) => {
                let local = e.local_name();
                let name = local.as_ref();

                if name == b"body" {
                    in_body = true;
                    buf.clear();
                    continue;
                }
                if !in_body {
                    buf.clear();
                    continue;
                }

                if name == b"Fallback" {
                    fallback_depth += 1;
                    buf.clear();
                    continue;
                }
                if fallback_depth > 0 {
                    buf.clear();
                    continue;
                }

                if name == b"del" {
                    del_depth += 1;
                    has_revisions = true;
                    buf.clear();
                    continue;
                }
                if name == b"ins" {
                    has_revisions = true;
                    buf.clear();
                    continue;
                }

                match name {
                    b"tbl" if cur_tbl.is_none() => {
                        cur_tbl = Some(TableCollector {
                            tbl_ord: next_tbl_ord,
                            ..TableCollector::default()
                        });
                        next_tbl_ord += 1;
                    }
                    b"tr" if cur_tbl.is_some() => {
                        if let Some(ref mut t) = cur_tbl {
                            t.cur_row.clear();
                        }
                    }
                    b"tc" if cur_tbl.is_some() => {
                        if let Some(ref mut t) = cur_tbl {
                            t.in_cell = true;
                            t.cur_cell_lines.clear();
                        }
                    }
                    b"p" => {
                        let p_ord = next_p_ord;
                        next_p_ord += 1;
                        if let Some(ref mut t) = cur_tbl {
                            if t.first_p_ord.is_none() {
                                t.first_p_ord = Some(p_ord);
                            }
                        }
                        cur_para = Some(ParaCollector {
                            p_ord,
                            para_id: attr_val(e, b"paraId"),
                            ..ParaCollector::default()
                        });
                    }
                    b"pStyle" => {
                        if let Some(ref mut p) = cur_para {
                            p.style_id = attr_val(e, b"val");
                        }
                    }
                    b"outlineLvl" => {
                        if let Some(ref mut p) = cur_para {
                            if let Some(v) = attr_val(e, b"val") {
                                if let Ok(lvl) = v.trim().parse::<u8>() {
                                    p.outline_lvl = Some(lvl.saturating_add(1).min(6));
                                }
                            }
                        }
                    }
                    b"numPr" => {
                        if let Some(ref mut p) = cur_para {
                            p.is_list = true;
                        }
                    }
                    b"bookmarkStart" => {
                        if let Some(ref mut p) = cur_para {
                            if p.bookmark.is_none() {
                                if let Some(bname) = attr_val(e, b"name") {
                                    if !bname.starts_with('_') && !bname.is_empty() {
                                        p.bookmark = Some(bname);
                                    }
                                }
                            }
                        }
                    }
                    b"t" if del_depth == 0 => {
                        in_text_elem = true;
                    }
                    _ => {}
                }
            }
            Event::Empty(ref e) => {
                if !in_body || fallback_depth > 0 {
                    buf.clear();
                    continue;
                }
                let local = e.local_name();
                match local.as_ref() {
                    b"p" => {
                        // empty paragraph keeps ordinal parity with patcher
                        next_p_ord += 1;
                    }
                    b"pStyle" => {
                        if let Some(ref mut p) = cur_para {
                            p.style_id = attr_val(e, b"val");
                        }
                    }
                    b"outlineLvl" => {
                        if let Some(ref mut p) = cur_para {
                            if let Some(v) = attr_val(e, b"val") {
                                if let Ok(lvl) = v.trim().parse::<u8>() {
                                    p.outline_lvl = Some(lvl.saturating_add(1).min(6));
                                }
                            }
                        }
                    }
                    b"bookmarkStart" => {
                        if let Some(ref mut p) = cur_para {
                            if p.bookmark.is_none() {
                                if let Some(bname) = attr_val(e, b"name") {
                                    if !bname.starts_with('_') && !bname.is_empty() {
                                        p.bookmark = Some(bname);
                                    }
                                }
                            }
                        }
                    }
                    b"tab" if del_depth == 0 => {
                        if let Some(ref mut p) = cur_para {
                            p.text.push(' ');
                        }
                    }
                    b"br" | b"cr" if del_depth == 0 => {
                        if let Some(ref mut p) = cur_para {
                            p.text.push('\n');
                        }
                    }
                    _ => {}
                }
            }
            Event::Text(ref e) => {
                if in_body && fallback_depth == 0 && del_depth == 0 && in_text_elem {
                    if let Some(ref mut p) = cur_para {
                        if let Ok(decoded) = e.decode() {
                            p.text.push_str(&decoded);
                        }
                    }
                }
            }
            Event::GeneralRef(ref g) => {
                if in_body && fallback_depth == 0 && del_depth == 0 && in_text_elem {
                    if let Some(ref mut p) = cur_para {
                        if let Ok(entity) = g.decode() {
                            if let Some(resolved) = resolve_xml_entity(&entity) {
                                p.text.push_str(&resolved);
                            }
                        }
                    }
                }
            }
            Event::End(ref e) => {
                let local = e.local_name();
                let name = local.as_ref();

                if name == b"body" {
                    in_body = false;
                    buf.clear();
                    continue;
                }
                if name == b"Fallback" && fallback_depth > 0 {
                    fallback_depth -= 1;
                    buf.clear();
                    continue;
                }
                if fallback_depth > 0 {
                    buf.clear();
                    continue;
                }
                if name == b"del" && del_depth > 0 {
                    del_depth -= 1;
                    buf.clear();
                    continue;
                }

                match name {
                    b"t" => {
                        in_text_elem = false;
                    }
                    b"p" => {
                        if let Some(p) = cur_para.take() {
                            let clean_text = normalize_paragraph_text(&p.text);
                            if let Some(ref mut t) = cur_tbl {
                                if t.in_cell && !clean_text.is_empty() {
                                    t.cur_cell_lines.push(clean_text);
                                }
                            } else if !clean_text.is_empty() {
                                let explicit = p.para_id.or(p.bookmark);
                                let heading_lvl = detect_heading_level(
                                    p.outline_lvl,
                                    p.style_id.as_deref(),
                                    styles,
                                    &clean_text,
                                );

                                let node_kind = if heading_lvl.is_some() {
                                    NodeKind::Heading
                                } else if p.is_list {
                                    NodeKind::ListItem
                                } else {
                                    NodeKind::Paragraph
                                };

                                if let Some(lvl) = heading_lvl {
                                    while heading_stack
                                        .last()
                                        .map(|(l, _, _)| *l >= lvl)
                                        .unwrap_or(false)
                                    {
                                        heading_stack.pop();
                                    }
                                }

                                let parent_anchor = heading_stack
                                    .last()
                                    .map(|(_, anc, _)| anc.clone());
                                let heading_path = if heading_lvl.is_some() {
                                    let mut parts: Vec<String> = heading_stack
                                        .iter()
                                        .map(|(_, _, t)| t.clone())
                                        .collect();
                                    parts.push(clean_text.clone());
                                    parts.join(" / ")
                                } else if heading_stack.is_empty() {
                                    String::from("文档正文")
                                } else {
                                    heading_stack
                                        .iter()
                                        .map(|(_, _, t)| t.clone())
                                        .collect::<Vec<_>>()
                                        .join(" / ")
                                };

                                let base_anchor = explicit
                                    .as_ref()
                                    .map(|id| format!("a_{id}"))
                                    .unwrap_or_else(|| format!("p_{}", p.p_ord));
                                let anchor_id = dedup_anchor(&mut anchor_seen, base_anchor);

                                let mandatory_tags = extract_mandatory_tags(&clean_text);
                                if !mandatory_tags.is_empty() {
                                    mandatory_count += 1;
                                }

                                for target in extract_cross_refs(&clean_text) {
                                    refs.push(DocRefRecord {
                                        source_anchor: anchor_id.clone(),
                                        target_hint: target,
                                    });
                                }

                                let ord = nodes.len();
                                let text_hash = sha256_hex(clean_text.as_bytes());
                                let token_estimate = estimate_tokens(&clean_text);

                                nodes.push(DocNodeRecord {
                                    ord,
                                    anchor_id: anchor_id.clone(),
                                    parent_anchor_id: parent_anchor,
                                    explicit_anchor: explicit,
                                    xml_locator: format!("p:{}", p.p_ord),
                                    node_kind,
                                    heading_path: heading_path.clone(),
                                    text_content: clean_text.clone(),
                                    text_hash,
                                    row_keys: Vec::new(),
                                    mandatory_tags,
                                    locked: false,
                                    token_estimate,
                                    is_search_chunk: false,
                                });

                                // Small-to-Big: emit 200~380 char search sub-chunks pointing to primary node
                                if clean_text.chars().count() > 420 {
                                    for (c_idx, chunk) in split_search_chunks(&clean_text, 340, 60)
                                        .into_iter()
                                        .enumerate()
                                    {
                                        let chunk_ord = nodes.len();
                                        let chunk_anchor = dedup_anchor(
                                            &mut anchor_seen,
                                            format!("{anchor_id}_c{c_idx}"),
                                        );
                                        nodes.push(DocNodeRecord {
                                            ord: chunk_ord,
                                            anchor_id: chunk_anchor,
                                            parent_anchor_id: Some(anchor_id.clone()),
                                            explicit_anchor: None,
                                            xml_locator: format!("p:{}", p.p_ord),
                                            node_kind,
                                            heading_path: heading_path.clone(),
                                            text_hash: sha256_hex(chunk.as_bytes()),
                                            token_estimate: estimate_tokens(&chunk),
                                            text_content: chunk,
                                            row_keys: Vec::new(),
                                            mandatory_tags: Vec::new(),
                                            locked: false,
                                            is_search_chunk: true,
                                        });
                                    }
                                }

                                if let Some(lvl) = heading_lvl {
                                    heading_stack.push((lvl, anchor_id, clean_text));
                                }
                            }
                        }
                    }
                    b"tc" if cur_tbl.is_some() => {
                        if let Some(ref mut t) = cur_tbl {
                            let cell_text = t.cur_cell_lines.join(" ");
                            t.cur_row.push(cell_text);
                            t.in_cell = false;
                            t.cur_cell_lines.clear();
                        }
                    }
                    b"tr" if cur_tbl.is_some() => {
                        if let Some(ref mut t) = cur_tbl {
                            if t.cur_row.iter().any(|c| !c.trim().is_empty()) {
                                t.rows.push(std::mem::take(&mut t.cur_row));
                            } else {
                                t.cur_row.clear();
                            }
                        }
                    }
                    b"tbl" => {
                        if let Some(t) = cur_tbl.take() {
                            if !t.rows.is_empty() {
                                table_count += 1;
                                emit_table_nodes(
                                    t,
                                    &heading_stack,
                                    &mut anchor_seen,
                                    &mut nodes,
                                    &mut refs,
                                    &mut mandatory_count,
                                );
                            }
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
        buf.clear();
    }

    let primary_count = nodes.iter().filter(|n| !n.is_search_chunk).count();
    Ok(ParsedDocx {
        nodes,
        refs,
        node_count: primary_count,
        table_count,
        mandatory_count,
        has_revisions,
    })
}

fn emit_table_nodes(
    tbl: TableCollector,
    heading_stack: &[(u8, String, String)],
    anchor_seen: &mut HashMap<String, usize>,
    nodes: &mut Vec<DocNodeRecord>,
    refs: &mut Vec<DocRefRecord>,
    mandatory_count: &mut usize,
) {
    let parent_anchor = heading_stack.last().map(|(_, anc, _)| anc.clone());
    let heading_path = if heading_stack.is_empty() {
        String::from("文档正文")
    } else {
        heading_stack
            .iter()
            .map(|(_, _, t)| t.clone())
            .collect::<Vec<_>>()
            .join(" / ")
    };

    let format_row = |cells: &[String]| -> String { cells.join(" | ") };
    let extract_row_key = |cells: &[String]| -> String {
        cells
            .iter()
            .map(|c| c.trim())
            .filter(|c| !c.is_empty())
            .take(2)
            .collect::<Vec<_>>()
            .join("|")
    };

    let full_lines: Vec<String> = tbl.rows.iter().map(|r| format_row(r)).collect();
    let full_text = full_lines.join("\n");
    let full_keys: Vec<String> = tbl.rows.iter().map(|r| extract_row_key(r)).collect();
    let main_anchor = dedup_anchor(anchor_seen, format!("tbl_{}", tbl.tbl_ord));
    let locator = tbl
        .first_p_ord
        .map(|p| format!("p:{p}"))
        .unwrap_or_else(|| format!("tbl:{}", tbl.tbl_ord));

    let mandatory_tags = extract_mandatory_tags(&full_text);
    if !mandatory_tags.is_empty() {
        *mandatory_count += 1;
    }
    for target in extract_cross_refs(&full_text) {
        refs.push(DocRefRecord {
            source_anchor: main_anchor.clone(),
            target_hint: target,
        });
    }

    let main_ord = nodes.len();
    nodes.push(DocNodeRecord {
        ord: main_ord,
        anchor_id: main_anchor.clone(),
        parent_anchor_id: parent_anchor,
        explicit_anchor: Some(format!("tbl_{}", tbl.tbl_ord)),
        xml_locator: locator.clone(),
        node_kind: NodeKind::Table,
        heading_path: heading_path.clone(),
        text_hash: sha256_hex(full_text.as_bytes()),
        token_estimate: estimate_tokens(&full_text),
        text_content: full_text,
        row_keys: full_keys,
        mandatory_tags,
        locked: false,
        is_search_chunk: false,
    });

    // Preserve table headers across row-group search chunks when table > 18 rows
    if tbl.rows.len() > 18 {
        let header = &tbl.rows[0];
        let header_line = format_row(header);
        let header_key = extract_row_key(header);
        for (chunk_idx, slice) in tbl.rows[1..].chunks(15).enumerate() {
            let mut lines = Vec::with_capacity(slice.len() + 1);
            let mut keys = Vec::with_capacity(slice.len() + 1);
            lines.push(header_line.clone());
            keys.push(header_key.clone());
            for row in slice {
                lines.push(format_row(row));
                keys.push(extract_row_key(row));
            }
            let chunk_text = lines.join("\n");
            let chunk_anchor = dedup_anchor(anchor_seen, format!("{main_anchor}_part{chunk_idx}"));
            let chunk_ord = nodes.len();
            nodes.push(DocNodeRecord {
                ord: chunk_ord,
                anchor_id: chunk_anchor,
                parent_anchor_id: Some(main_anchor.clone()),
                explicit_anchor: None,
                xml_locator: locator.clone(),
                node_kind: NodeKind::TableChunk,
                heading_path: heading_path.clone(),
                text_hash: sha256_hex(chunk_text.as_bytes()),
                token_estimate: estimate_tokens(&chunk_text),
                text_content: chunk_text,
                row_keys: keys,
                mandatory_tags: Vec::new(),
                locked: false,
                is_search_chunk: true,
            });
        }
    }
}

/// Losslessly patches `word/document.xml` inside an existing `.docx` ZIP archive.
/// All styles, numbering, headers, footers, media, and untouched paragraphs are copied byte-for-byte.
pub fn patch_docx_zip(
    baseline_docx: &[u8],
    patches_by_locator: &HashMap<String, ParagraphPatch>,
) -> Result<Vec<u8>, EngineError> {
    validate_docx_bytes(baseline_docx)?;
    let mut archive = ZipArchive::new(Cursor::new(baseline_docx))
        .map_err(|_| EngineError::invalid_input("打开基线 .docx 失败"))?;

    let doc_bytes = read_zip_entry_bounded(&mut archive, "word/document.xml", MAX_XML_UNCOMPRESSED_BYTES)?
        .ok_or_else(|| EngineError::invalid_input("基线文档缺少 word/document.xml"))?;

    let patched_xml = if patches_by_locator.is_empty() {
        doc_bytes
    } else {
        patch_document_xml(&doc_bytes, patches_by_locator)?
    };

    let mut out_cursor = Cursor::new(Vec::with_capacity(baseline_docx.len()));
    {
        let mut writer = ZipWriter::new(&mut out_cursor);
        let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

        for i in 0..archive.len() {
            let file = archive
                .by_index_raw(i)
                .map_err(|_| EngineError::internal("读取归档条目失败"))?;
            if file.name() == "word/document.xml" {
                drop(file);
                writer
                    .start_file("word/document.xml", opts)
                    .map_err(|_| EngineError::internal("写入 patched document.xml 失败"))?;
                writer
                    .write_all(&patched_xml)
                    .map_err(|_| EngineError::internal("写入 XML 内容失败"))?;
            } else {
                writer
                    .raw_copy_file(file)
                    .map_err(|_| EngineError::internal("无损复制原包资源失败"))?;
            }
        }
        writer
            .finish()
            .map_err(|_| EngineError::internal("完成 .docx 打包失败"))?;
    }

    Ok(out_cursor.into_inner())
}

fn patch_document_xml(
    xml_bytes: &[u8],
    patches_by_locator: &HashMap<String, ParagraphPatch>,
) -> Result<Vec<u8>, EngineError> {
    let mut reader = Reader::from_reader(xml_bytes);
    reader.config_mut().trim_text(false);
    let mut writer = Writer::new(Cursor::new(Vec::with_capacity(xml_bytes.len())));
    let mut buf = Vec::new();

    let mut in_body = false;
    let mut next_p_ord: usize = 0;
    let mut next_rev_id: usize = 1;

    let mut capturing_patch: Option<ParagraphPatch> = None;
    let mut capture_depth: usize = 0;
    let mut captured_p_start: Option<BytesStart<'static>> = None;
    let mut captured_ppr: Option<Vec<u8>> = None;
    let mut captured_rpr: Option<Vec<u8>> = None;
    let mut captured_bookmarks: Vec<u8> = Vec::new();

    let mut sub_capture: Option<(SubCaptureKind, usize, Writer<Cursor<Vec<u8>>>)> = None;

    loop {
        let ev = reader
            .read_event_into(&mut buf)
            .map_err(|_| EngineError::invalid_input("XML 解析失败，无法应用补丁"))?;
        if matches!(ev, Event::Eof) {
            break;
        }

        if let Some(ref patch) = capturing_patch {
            if let Some((kind, mut depth, mut sub_w)) = sub_capture.take() {
                match &ev {
                    Event::Start(_) => {
                        depth += 1;
                        sub_w
                            .write_event(ev.borrow())
                            .map_err(|_| EngineError::internal("复制节点属性失败"))?;
                        sub_capture = Some((kind, depth, sub_w));
                    }
                    Event::End(_) => {
                        sub_w
                            .write_event(ev.borrow())
                            .map_err(|_| EngineError::internal("结束复制节点失败"))?;
                        depth -= 1;
                        if depth == 0 {
                            capture_depth = capture_depth.saturating_sub(1);
                            let bytes = sub_w.into_inner().into_inner();
                            match kind {
                                SubCaptureKind::PPr => {
                                    if captured_ppr.is_none() {
                                        captured_ppr = Some(bytes);
                                    }
                                }
                                SubCaptureKind::RPr => {
                                    if captured_rpr.is_none() {
                                        captured_rpr = Some(bytes);
                                    }
                                }
                            }
                        } else {
                            sub_capture = Some((kind, depth, sub_w));
                        }
                    }
                    _ => {
                        sub_w
                            .write_event(ev.borrow())
                            .map_err(|_| EngineError::internal("写入节点属性失败"))?;
                        sub_capture = Some((kind, depth, sub_w));
                    }
                }
                buf.clear();
                continue;
            }

            match &ev {
                Event::Start(e) => {
                    capture_depth += 1;
                    let local = e.local_name();
                    if capture_depth == 2 && local.as_ref() == b"pPr" && captured_ppr.is_none() {
                        let mut sub_w = Writer::new(Cursor::new(Vec::new()));
                        sub_w
                            .write_event(Event::Start(e.borrow()))
                            .map_err(|_| EngineError::internal("复制 pPr 失败"))?;
                        sub_capture = Some((SubCaptureKind::PPr, 1, sub_w));
                    } else if local.as_ref() == b"rPr" && captured_rpr.is_none() {
                        let mut sub_w = Writer::new(Cursor::new(Vec::new()));
                        sub_w
                            .write_event(Event::Start(e.borrow()))
                            .map_err(|_| EngineError::internal("复制 rPr 失败"))?;
                        sub_capture = Some((SubCaptureKind::RPr, 1, sub_w));
                    } else if matches!(local.as_ref(), b"bookmarkStart" | b"bookmarkEnd") {
                        let mut bw = Writer::new(Cursor::new(Vec::new()));
                        let _ = bw.write_event(Event::Start(e.borrow()));
                        captured_bookmarks.extend_from_slice(&bw.into_inner().into_inner());
                    }
                }
                Event::Empty(e) => {
                    let local = e.local_name();
                    if capture_depth == 1 && local.as_ref() == b"pPr" && captured_ppr.is_none() {
                        let mut bw = Writer::new(Cursor::new(Vec::new()));
                        let _ = bw.write_event(Event::Empty(e.borrow()));
                        captured_ppr = Some(bw.into_inner().into_inner());
                    } else if local.as_ref() == b"rPr" && captured_rpr.is_none() {
                        let mut bw = Writer::new(Cursor::new(Vec::new()));
                        let _ = bw.write_event(Event::Empty(e.borrow()));
                        captured_rpr = Some(bw.into_inner().into_inner());
                    } else if matches!(local.as_ref(), b"bookmarkStart" | b"bookmarkEnd") {
                        let mut bw = Writer::new(Cursor::new(Vec::new()));
                        let _ = bw.write_event(Event::Empty(e.borrow()));
                        captured_bookmarks.extend_from_slice(&bw.into_inner().into_inner());
                    }
                }
                Event::End(e) => {
                    capture_depth = capture_depth.saturating_sub(1);
                    if capture_depth == 0 && e.local_name().as_ref() == b"p" {
                        let start = captured_p_start.take().unwrap_or_else(|| BytesStart::new("w:p"));
                        writer
                            .write_event(Event::Start(start))
                            .map_err(|_| EngineError::internal("写回段落起标签失败"))?;
                        if let Some(ppr) = captured_ppr.take() {
                            writer
                                .get_mut()
                                .write_all(&ppr)
                                .map_err(|_| EngineError::internal("写回 pPr 失败"))?;
                        }
                        if !captured_bookmarks.is_empty() {
                            writer
                                .get_mut()
                                .write_all(&captured_bookmarks)
                                .map_err(|_| EngineError::internal("写回书签失败"))?;
                            captured_bookmarks.clear();
                        }

                        emit_patched_runs(
                            &mut writer,
                            patch,
                            captured_rpr.take().as_deref(),
                            &mut next_rev_id,
                        )?;

                        writer
                            .write_event(Event::End(BytesEnd::new(
                                std::str::from_utf8(e.name().as_ref()).unwrap_or("w:p"),
                            )))
                            .map_err(|_| EngineError::internal("写回段落闭标签失败"))?;

                        capturing_patch = None;
                    }
                }
                _ => {}
            }

            buf.clear();
            continue;
        }

        match &ev {
            Event::Start(e) => {
                let local = e.local_name();
                if local.as_ref() == b"body" {
                    in_body = true;
                } else if in_body && local.as_ref() == b"p" {
                    let loc = format!("p:{next_p_ord}");
                    next_p_ord += 1;
                    if let Some(patch) = patches_by_locator.get(&loc) {
                        capturing_patch = Some(patch.clone());
                        capture_depth = 1;
                        captured_p_start = Some(e.borrow().into_owned());
                        captured_ppr = None;
                        captured_rpr = None;
                        captured_bookmarks.clear();
                        buf.clear();
                        continue;
                    }
                }
                writer
                    .write_event(ev.borrow())
                    .map_err(|_| EngineError::internal("写回 XML 事件失败"))?;
            }
            Event::Empty(e) => {
                let local = e.local_name();
                if in_body && local.as_ref() == b"p" {
                    let loc = format!("p:{next_p_ord}");
                    next_p_ord += 1;
                    if let Some(patch) = patches_by_locator.get(&loc) {
                        let start = e.borrow().into_owned();
                        writer
                            .write_event(Event::Start(start))
                            .map_err(|_| EngineError::internal("展开空段落失败"))?;
                        emit_patched_runs(&mut writer, patch, None, &mut next_rev_id)?;
                        writer
                            .write_event(Event::End(BytesEnd::new(
                                std::str::from_utf8(e.name().as_ref()).unwrap_or("w:p"),
                            )))
                            .map_err(|_| EngineError::internal("关闭段落失败"))?;
                        buf.clear();
                        continue;
                    }
                }
                writer
                    .write_event(ev.borrow())
                    .map_err(|_| EngineError::internal("写回空标签失败"))?;
            }
            Event::End(e) => {
                if e.local_name().as_ref() == b"body" {
                    in_body = false;
                }
                writer
                    .write_event(ev.borrow())
                    .map_err(|_| EngineError::internal("写回闭标签失败"))?;
            }
            _ => {
                writer
                    .write_event(ev.borrow())
                    .map_err(|_| EngineError::internal("写回 XML 节点失败"))?;
            }
        }
        buf.clear();
    }

    Ok(writer.into_inner().into_inner())
}

#[derive(Clone, Copy)]
enum SubCaptureKind {
    PPr,
    RPr,
}

fn emit_patched_runs(
    writer: &mut Writer<Cursor<Vec<u8>>>,
    patch: &ParagraphPatch,
    rpr_bytes: Option<&[u8]>,
    next_rev_id: &mut usize,
) -> Result<(), EngineError> {
    match patch.mode {
        ExportMode::Clean | ExportMode::Raw => {
            write_text_run(writer, rpr_bytes, &patch.new_text, false)?;
        }
        ExportMode::Redline => {
            let (spans, _, _) = diff_text_spans(&patch.before_text, &patch.new_text);
            if spans.is_empty() {
                write_text_run(writer, rpr_bytes, &patch.new_text, false)?;
                return Ok(());
            }
            for span in spans {
                match span.op {
                    SpanOp::Equal => {
                        write_text_run(writer, rpr_bytes, &span.text, false)?;
                    }
                    SpanOp::Delete => {
                        let rev_id = (*next_rev_id).to_string();
                        *next_rev_id += 1;
                        let mut del = BytesStart::new("w:del");
                        del.push_attribute(("w:id", rev_id.as_str()));
                        del.push_attribute(("w:author", patch.author.as_str()));
                        del.push_attribute(("w:date", patch.date_iso.as_str()));
                        writer
                            .write_event(Event::Start(del))
                            .map_err(|_| EngineError::internal("写入 w:del 失败"))?;
                        write_text_run(writer, rpr_bytes, &span.text, true)?;
                        writer
                            .write_event(Event::End(BytesEnd::new("w:del")))
                            .map_err(|_| EngineError::internal("关闭 w:del 失败"))?;
                    }
                    SpanOp::Insert => {
                        let rev_id = (*next_rev_id).to_string();
                        *next_rev_id += 1;
                        let mut ins = BytesStart::new("w:ins");
                        ins.push_attribute(("w:id", rev_id.as_str()));
                        ins.push_attribute(("w:author", patch.author.as_str()));
                        ins.push_attribute(("w:date", patch.date_iso.as_str()));
                        writer
                            .write_event(Event::Start(ins))
                            .map_err(|_| EngineError::internal("写入 w:ins 失败"))?;
                        write_text_run(writer, rpr_bytes, &span.text, false)?;
                        writer
                            .write_event(Event::End(BytesEnd::new("w:ins")))
                            .map_err(|_| EngineError::internal("关闭 w:ins 失败"))?;
                    }
                }
            }
        }
    }
    Ok(())
}

fn write_text_run(
    writer: &mut Writer<Cursor<Vec<u8>>>,
    rpr_bytes: Option<&[u8]>,
    text: &str,
    is_deleted: bool,
) -> Result<(), EngineError> {
    writer
        .write_event(Event::Start(BytesStart::new("w:r")))
        .map_err(|_| EngineError::internal("写入 w:r 失败"))?;
    if let Some(rpr) = rpr_bytes {
        writer
            .get_mut()
            .write_all(rpr)
            .map_err(|_| EngineError::internal("写回 w:rPr 失败"))?;
    }
    let tag = if is_deleted { "w:delText" } else { "w:t" };
    let mut t_start = BytesStart::new(tag);
    t_start.push_attribute(("xml:space", "preserve"));
    writer
        .write_event(Event::Start(t_start))
        .map_err(|_| EngineError::internal("写入文本标签失败"))?;
    writer
        .write_event(Event::Text(BytesText::new(text)))
        .map_err(|_| EngineError::internal("写入文本内容失败"))?;
    writer
        .write_event(Event::End(BytesEnd::new(tag)))
        .map_err(|_| EngineError::internal("关闭文本标签失败"))?;
    writer
        .write_event(Event::End(BytesEnd::new("w:r")))
        .map_err(|_| EngineError::internal("关闭 w:r 失败"))?;
    Ok(())
}

fn attr_val(e: &BytesStart<'_>, target_local: &[u8]) -> Option<String> {
    for attr in e.attributes().flatten() {
        if attr.key.local_name().as_ref() == target_local {
            if let Ok(val) = attr.unescape_value() {
                return Some(val.into_owned());
            }
        }
    }
    None
}

fn resolve_xml_entity(name: &str) -> Option<String> {
    match name {
        "amp" => Some(String::from("&")),
        "lt" => Some(String::from("<")),
        "gt" => Some(String::from(">")),
        "quot" => Some(String::from("\"")),
        "apos" => Some(String::from("'")),
        _ if name.starts_with("#x") || name.starts_with("#X") => {
            u32::from_str_radix(&name[2..], 16)
                .ok()
                .and_then(char::from_u32)
                .map(|c| c.to_string())
        }
        _ if name.starts_with('#') => name[1..]
            .parse::<u32>()
            .ok()
            .and_then(char::from_u32)
            .map(|c| c.to_string()),
        _ => None,
    }
}

fn normalize_paragraph_text(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let mut out = String::with_capacity(trimmed.len());
    let mut prev_space = false;
    for ch in trimmed.chars() {
        if ch == '\n' {
            if !out.ends_with('\n') {
                out.push('\n');
            }
            prev_space = false;
        } else if ch.is_whitespace() {
            if !prev_space {
                out.push(' ');
                prev_space = true;
            }
        } else {
            out.push(ch);
            prev_space = false;
        }
    }
    out.trim().to_string()
}

fn detect_heading_level(
    direct_outline: Option<u8>,
    style_id: Option<&str>,
    styles: &HashMap<String, StyleMeta>,
    text: &str,
) -> Option<u8> {
    if let Some(lvl) = direct_outline {
        return Some(lvl);
    }
    if let Some(sid) = style_id {
        if let Some(meta) = styles.get(sid) {
            if let Some(lvl) = meta.outline_lvl {
                return Some(lvl);
            }
            if let Some(lvl) = heading_level_from_style_label(&meta.name) {
                return Some(lvl);
            }
        }
        if let Some(lvl) = heading_level_from_style_label(sid) {
            return Some(lvl);
        }
    }

    // Fallback structural heading recognition for unstyled Chinese tender documents
    let chars_cnt = text.chars().count();
    if chars_cnt > 48 || text.ends_with('。') || text.ends_with('；') {
        return None;
    }
    if text.starts_with('第') && (text.contains("卷 ") || text.contains("篇 ") || text.contains("章 ")) {
        return Some(1);
    }
    if text.starts_with('第') && text.contains("节 ") {
        return Some(2);
    }
    // Numeric prefix like "3.2 运维响应机制"
    let first_tok = text.split_whitespace().next().unwrap_or("");
    if first_tok.contains('.')
        && first_tok
            .chars()
            .all(|c| c.is_ascii_digit() || c == '.')
        && text.contains(' ')
    {
        let dots = first_tok.matches('.').count();
        return Some((dots as u8 + 1).clamp(2, 5));
    }
    None
}

fn heading_level_from_style_label(raw: &str) -> Option<u8> {
    let lower = raw.trim().to_ascii_lowercase();
    for prefix in ["heading", "标题", "toc"] {
        if let Some(rest) = lower.strip_prefix(prefix) {
            let digits: String = rest.chars().filter(|c| c.is_ascii_digit()).collect();
            if let Ok(n) = digits.parse::<u8>() {
                if (1..=6).contains(&n) {
                    return Some(n);
                }
            }
        }
    }
    None
}

fn extract_mandatory_tags(text: &str) -> Vec<String> {
    let mut tags = Vec::new();
    let checks: [(&str, &str); 7] = [
        ("★", "★星标"),
        ("▲", "▲重要"),
        ("※", "※重点"),
        ("废标", "废标红线"),
        ("无效投标", "无效投标"),
        ("实质性", "实质性要求"),
        ("必须满足", "必须满足"),
    ];
    for (needle, label) in checks {
        if text.contains(needle) {
            tags.push(label.to_string());
        }
    }
    tags
}

fn extract_cross_refs(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    let chars: Vec<char> = text.chars().collect();

    for i in 0..chars.len() {
        if chars[i] == '表' || (i + 1 < chars.len() && chars[i] == '附' && chars[i + 1] == '件')
        {
            let prefix_len = if chars[i] == '表' { 1 } else { 2 };
            let mut j = i + prefix_len;
            while j < chars.len() && chars[j].is_whitespace() {
                j += 1;
            }
            let start_num = j;
            while j < chars.len()
                && (chars[j].is_ascii_digit() || chars[j] == '-' || chars[j] == '.')
            {
                j += 1;
            }
            if j > start_num && chars[start_num..j].iter().any(|c| c.is_ascii_digit()) {
                let candidate: String = chars[i..j].iter().collect();
                let clean = candidate.trim().to_string();
                if seen.insert(clean.clone()) {
                    out.push(clean);
                }
            }
        } else if chars[i] == '第' {
            let mut j = i + 1;
            while j < chars.len() && chars[j].is_whitespace() {
                j += 1;
            }
            let start_num = j;
            while j < chars.len() && (chars[j].is_ascii_digit() || chars[j] == '.') {
                j += 1;
            }
            while j < chars.len() && chars[j].is_whitespace() {
                j += 1;
            }
            if j > start_num
                && j < chars.len()
                && matches!(chars[j], '条' | '章' | '节' | '款')
            {
                let candidate: String = chars[i..=j].iter().collect();
                let clean = candidate.trim().to_string();
                if seen.insert(clean.clone()) {
                    out.push(clean);
                }
            }
        }
    }
    out
}

fn split_search_chunks(text: &str, max_chars: usize, overlap: usize) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= max_chars {
        return vec![text.to_string()];
    }
    let step = max_chars.saturating_sub(overlap).max(80);
    let mut chunks = Vec::new();
    let mut start = 0usize;
    while start < chars.len() {
        let end = (start + max_chars).min(chars.len());
        chunks.push(chars[start..end].iter().collect());
        if end == chars.len() {
            break;
        }
        start += step;
    }
    chunks
}

fn dedup_anchor(seen: &mut HashMap<String, usize>, base: String) -> String {
    let cnt = seen.entry(base.clone()).or_insert(0);
    *cnt += 1;
    if *cnt == 1 {
        base
    } else {
        format!("{base}_{}", *cnt)
    }
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    encode_hex(&h.finalize())
}

pub fn estimate_tokens(text: &str) -> usize {
    let mut cjk = 0usize;
    let mut other = 0usize;
    for ch in text.chars() {
        if !ch.is_ascii() {
            cjk += 1;
        } else {
            other += 1;
        }
    }
    (cjk + (other + 3) / 4).max(1)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::io::{Cursor, Read, Write};

    use zip::write::SimpleFileOptions;
    use zip::{CompressionMethod, ZipArchive, ZipWriter};

    use super::{parse_docx_ast, patch_docx_zip, validate_docx_bytes, ParagraphPatch};
    use crate::core::error::ErrorCode;
    use crate::doc::model::{ExportMode, NodeKind};

    fn build_sample_docx(document_xml: &str) -> Vec<u8> {
        let mut cursor = Cursor::new(Vec::new());
        {
            let mut zw = ZipWriter::new(&mut cursor);
            let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

            zw.start_file("[Content_Types].xml", opts).unwrap();
            zw.write_all(b"<Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\"/>").unwrap();

            zw.start_file("word/styles.xml", opts).unwrap();
            zw.write_all(
                br#"<?xml version="1.0" encoding="UTF-8"?>
<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:style w:type="paragraph" w:styleId="Heading1"><w:name w:val="heading 1"/><w:pPr><w:outlineLvl w:val="0"/></w:pPr></w:style>
</w:styles>"#,
            )
            .unwrap();

            zw.start_file("word/header1.xml", opts).unwrap();
            zw.write_all(b"<w:hdr><w:p><w:r><w:t>CONFIDENTIAL HEADER</w:t></w:r></w:p></w:hdr>")
                .unwrap();

            zw.start_file("word/media/logo.png", opts).unwrap();
            zw.write_all(b"\x89PNG\r\n\x1a\nFAKEPNG").unwrap();

            zw.start_file("word/document.xml", opts).unwrap();
            zw.write_all(document_xml.as_bytes()).unwrap();
            zw.finish().unwrap();
        }
        cursor.into_inner()
    }

    #[test]
    fn rejects_legacy_binary_doc_with_clear_guidance() {
        let mut ole = vec![0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];
        ole.extend_from_slice(&[0u8; 64]);
        let err = validate_docx_bytes(&ole).expect_err("should reject binary .doc");
        assert_eq!(err.code, ErrorCode::InvalidInput);
        assert!(err.message.contains("另存为 .docx"));
    }

    #[test]
    fn parses_wps_split_runs_alternate_content_mandatory_and_large_table_chunks() {
        let mut rows_xml = String::from(
            "<w:tr><w:tc><w:p><w:r><w:t>编号</w:t></w:r></w:p></w:tc><w:tc><w:p><w:r><w:t>指标</w:t></w:r></w:p></w:tc></w:tr>",
        );
        for i in 1..=22 {
            rows_xml.push_str(&format!(
                "<w:tr><w:tc><w:p><w:r><w:t>R{i}</w:t></w:r></w:p></w:tc><w:tc><w:p><w:r><w:t>SLA 项 {i}</w:t></w:r></w:p></w:tc></w:tr>"
            ));
        }

        let doc_xml = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
            xmlns:w14="http://schemas.microsoft.com/office/word/2010/wordml"
            xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006">
  <w:body>
    <w:p w14:paraId="1111AAAA">
      <w:pPr><w:pStyle w:val="Heading1"/></w:pPr>
      <w:r><w:t>第三章 运维服务方案</w:t></w:r>
    </w:p>
    <w:p w14:paraId="2222BBBB">
      <w:r><w:rPr><w:b/></w:rPr><w:t>★ 一般</w:t></w:r>
      <w:proofErr w:type="spellStart"/>
      <w:r><w:t>故障 30 分钟内响应，值班见表 2-3，详见第 3.2 条。</w:t></w:r>
      <w:mc:AlternateContent>
        <w:mc:Choice><w:r><w:t>（含7x24值守）</w:t></w:r></w:mc:Choice>
        <w:mc:Fallback><w:r><w:t>（重复兼容文本应忽略）</w:t></w:r></w:mc:Fallback>
      </w:mc:AlternateContent>
    </w:p>
    <w:tbl>{rows_xml}</w:tbl>
  </w:body>
</w:document>"#
        );

        let bytes = build_sample_docx(&doc_xml);
        let parsed = parse_docx_ast(&bytes).expect("parse docx");
        assert_eq!(parsed.table_count, 1);
        assert_eq!(parsed.mandatory_count, 1);
        assert_eq!(parsed.nodes[0].node_kind, NodeKind::Heading);
        assert!(parsed.nodes[1].text_content.contains("★ 一般故障 30 分钟内响应"));
        assert!(parsed.nodes[1].text_content.contains("（含7x24值守）"));
        assert!(!parsed.nodes[1].text_content.contains("重复兼容文本"));
        assert!(parsed.refs.iter().any(|r| r.target_hint == "表 2-3"));
        assert!(parsed.refs.iter().any(|r| r.target_hint == "第 3.2 条"));

        let chunks: Vec<_> = parsed
            .nodes
            .iter()
            .filter(|n| n.node_kind == NodeKind::TableChunk)
            .collect();
        assert_eq!(chunks.len(), 2);
        assert!(chunks[0].text_content.starts_with("编号 | 指标"));
        assert!(chunks[1].text_content.starts_with("编号 | 指标"));
    }

    #[test]
    fn lossless_patch_supports_clean_and_redline_preserving_package_assets() {
        let doc_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
            xmlns:w14="http://schemas.microsoft.com/office/word/2010/wordml">
  <w:body>
    <w:p w14:paraId="ABCD1234">
      <w:pPr><w:jc w:val="both"/></w:pPr>
      <w:r><w:rPr><w:sz w:val="24"/></w:rPr><w:t>连续两季度无故障可下调班次。</w:t></w:r>
    </w:p>
  </w:body>
</w:document>"#;
        let baseline = build_sample_docx(doc_xml);

        // Clean mode
        let mut clean_map = HashMap::new();
        clean_map.insert(
            String::from("p:0"),
            ParagraphPatch {
                before_text: String::from("连续两季度无故障可下调班次。"),
                new_text: String::from("连续两季度无故障可下调班次，需甲方书面确认。"),
                mode: ExportMode::Clean,
                author: String::from("陈默"),
                date_iso: String::from("2026-11-22T10:00:00Z"),
            },
        );
        let clean_bytes = patch_docx_zip(&baseline, &clean_map).expect("clean patch");
        let reparsed = parse_docx_ast(&clean_bytes).expect("reparse clean");
        assert_eq!(
            reparsed.nodes[0].text_content,
            "连续两季度无故障可下调班次，需甲方书面确认。"
        );

        // Verify untouched header & media preserved identically
        let mut za = ZipArchive::new(Cursor::new(clean_bytes.as_slice())).unwrap();
        let mut png = Vec::new();
        za.by_name("word/media/logo.png").unwrap().read_to_end(&mut png).unwrap();
        assert_eq!(png, b"\x89PNG\r\n\x1a\nFAKEPNG");

        // Redline mode
        let mut red_map = HashMap::new();
        red_map.insert(
            String::from("p:0"),
            ParagraphPatch {
                before_text: String::from("连续两季度无故障可下调班次。"),
                new_text: String::from("连续两季度无故障可下调班次，需甲方书面确认。"),
                mode: ExportMode::Redline,
                author: String::from("陈默"),
                date_iso: String::from("2026-11-22T10:00:00Z"),
            },
        );
        let red_bytes = patch_docx_zip(&baseline, &red_map).expect("redline patch");
        let mut red_za = ZipArchive::new(Cursor::new(red_bytes.as_slice())).unwrap();
        let mut xml_out = String::new();
        red_za.by_name("word/document.xml").unwrap().read_to_string(&mut xml_out).unwrap();
        assert!(xml_out.contains("<w:ins"));
        assert!(xml_out.contains("<w:del"));
        assert!(xml_out.contains("<w:jc w:val=\"both\"/>"));
        assert!(xml_out.contains("<w:sz w:val=\"24\"/>"));
    }
}
