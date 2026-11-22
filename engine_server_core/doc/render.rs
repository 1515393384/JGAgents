//! 渲染层：把 .docx 解析成「前端能按 Word 的样子画出来」的块序列。
//!
//! 这一层与 `ooxml.rs` 的语义层（patch / diff / 检索用）是**两遍扫描**：
//! `ooxml.rs` 产出便于改写和检索的纯文本节点，本模块产出保住外观的渲染块。
//! 两者的段落口径一致：都用 `p:{n}` 作为段落序号（locator），n 只统计 `<w:p>`，
//! 空段落同样占号，所以 `anchor` 可以互相对齐。
//!
//! 不在这里做的事：改写、diff、检索、图片字节提取。

use std::collections::HashMap;
use std::io::{Cursor, Read};

use quick_xml::events::{BytesStart, Event};
use quick_xml::reader::Reader;
use serde::{Deserialize, Serialize};
use zip::ZipArchive;

use crate::core::error::EngineError;
use crate::doc::model::{MAX_TOTAL_UNCOMPRESSED_BYTES, MAX_XML_UNCOMPRESSED_BYTES};
use crate::doc::ooxml::validate_docx_bytes;

/// 默认字号：中文 Word 的「五号」= 10.5pt = 21 half-point。
const DEFAULT_SIZE_HALF_PT: u16 = 21;
const DEFAULT_FONT: &str = "宋体";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PageSetup {
    pub width_twips: i32,
    pub height_twips: i32,
    pub margin_top_twips: i32,
    pub margin_right_twips: i32,
    pub margin_bottom_twips: i32,
    pub margin_left_twips: i32,
    pub header_twips: i32,
    pub footer_twips: i32,
    pub gutter_twips: i32,
    pub landscape: bool,
}

impl Default for PageSetup {
    fn default() -> Self {
        // A4，上下 2.54cm / 左右 3.17cm，Word 中文默认版式
        Self {
            width_twips: 11906,
            height_twips: 16838,
            margin_top_twips: 1440,
            margin_right_twips: 1800,
            margin_bottom_twips: 1440,
            margin_left_twips: 1800,
            header_twips: 851,
            footer_twips: 992,
            gutter_twips: 0,
            landscape: false,
        }
    }
}

/// 解析后落到前端的 run 外观，字段都是「已解析出的确定值」。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunFmt {
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strike: bool,
    /// half-point，24 = 12pt
    pub size_half_pt: u16,
    /// "RRGGBB"，空串表示继承默认色
    pub color: String,
    pub font: String,
    /// "" | "superscript" | "subscript"
    pub vert_align: String,
    /// "" 表示无底纹
    pub highlight: String,
}

impl Default for RunFmt {
    fn default() -> Self {
        Self {
            bold: false,
            italic: false,
            underline: false,
            strike: false,
            size_half_pt: DEFAULT_SIZE_HALF_PT,
            color: String::new(),
            font: String::from(DEFAULT_FONT),
            vert_align: String::new(),
            highlight: String::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunSpan {
    pub text: String,
    pub fmt: RunFmt,
    /// 图片 run：`text` 为空，前端按 `path` 走媒体接口取图
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub image: Option<ImageRef>,
}

/// 正文里的一张图。`path` 是 docx 包内路径，尺寸为 0 表示原图尺寸。
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageRef {
    pub path: String,
    pub width_px: u32,
    pub height_px: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParaFmt {
    pub style_id: String,
    /// "" | "left" | "center" | "right" | "justify" | "distribute"
    pub align: String,
    pub indent_left_twips: i32,
    pub indent_right_twips: i32,
    /// 可为负（悬挂缩进）
    pub indent_first_line_twips: i32,
    pub space_before_twips: i32,
    pub space_after_twips: i32,
    /// 0 表示未指定
    pub line_twips: i32,
    /// "auto" | "exact" | "atLeast"
    pub line_rule: String,
    pub num_id: String,
    pub num_level: u8,
    /// 0 = 正文；1..6 = 标题级
    pub heading_level: u8,
    pub keep_next: bool,
    pub page_break_before: bool,
}

impl Default for ParaFmt {
    fn default() -> Self {
        Self {
            style_id: String::new(),
            align: String::new(),
            indent_left_twips: 0,
            indent_right_twips: 0,
            indent_first_line_twips: 0,
            space_before_twips: 0,
            space_after_twips: 0,
            line_twips: 0,
            line_rule: String::from("auto"),
            num_id: String::new(),
            num_level: 0,
            heading_level: 0,
            keep_next: false,
            page_break_before: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RPara {
    /// 段落序号，与 `xml_locator` 的 `p:{n}` 同口径
    pub ord: usize,
    pub fmt: ParaFmt,
    pub runs: Vec<RunSpan>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TableCell {
    pub grid_span: u16,
    /// "" | "restart" | "continue"
    pub v_merge: String,
    pub width_twips: i32,
    pub paras: Vec<RPara>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TableRow {
    pub height_twips: i32,
    pub header: bool,
    pub cells: Vec<TableCell>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RTable {
    pub first_ord: usize,
    pub grid: Vec<i32>,
    pub rows: Vec<TableRow>,
    pub width_twips: i32,
    /// "" | "left" | "center" | "right"
    pub align: String,
}

/// 统一块结构（`kind` 判别），前端一个 switch 就能画。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderBlock {
    /// "para" | "table" | "page_break"
    pub kind: String,
    pub ord: usize,
    /// 与语义层对齐的位置标识：`p:{ord}`
    pub anchor: String,
    pub heading_level: u8,
    pub heading_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub para: Option<RPara>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub table: Option<RTable>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderedDoc {
    pub page: PageSetup,
    pub blocks: Vec<RenderBlock>,
    pub para_count: usize,
    pub table_count: usize,
    pub page_break_count: usize,
}

/// 按 ord 区间切出的一段可渲染内容，供前端分章/分段懒加载。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderSlice {
    pub doc_id: String,
    pub rev: i64,
    pub page: PageSetup,
    pub blocks: Vec<RenderBlock>,
    /// 该版本块总数（未裁剪前）
    pub total: usize,
    /// 切片首块/末块的段落序号（定位锚点用）
    pub first_ord: usize,
    pub last_ord: usize,
    /// 切片首块/末块的块序号（续接加载用 `last_seq + 1`）
    pub first_seq: usize,
    pub last_seq: usize,
    pub has_more: bool,
    pub para_count: usize,
    pub table_count: usize,
    pub page_break_count: usize,
}

// ── 样式表：把 styles.xml 的继承链展开成可覆盖的补丁 ──

#[derive(Clone, Debug, Default)]
struct RunPatch {
    bold: Option<bool>,
    italic: Option<bool>,
    underline: Option<bool>,
    strike: Option<bool>,
    size: Option<u16>,
    color: Option<String>,
    font: Option<String>,
    vert: Option<String>,
    highlight: Option<String>,
}

impl RunPatch {
    fn overlay(&mut self, top: &RunPatch) {
        if top.bold.is_some() {
            self.bold = top.bold;
        }
        if top.italic.is_some() {
            self.italic = top.italic;
        }
        if top.underline.is_some() {
            self.underline = top.underline;
        }
        if top.strike.is_some() {
            self.strike = top.strike;
        }
        if top.size.is_some() {
            self.size = top.size;
        }
        if top.color.is_some() {
            self.color = top.color.clone();
        }
        if top.font.is_some() {
            self.font = top.font.clone();
        }
        if top.vert.is_some() {
            self.vert = top.vert.clone();
        }
        if top.highlight.is_some() {
            self.highlight = top.highlight.clone();
        }
    }

    fn resolve(&self) -> RunFmt {
        let base = RunFmt::default();
        RunFmt {
            bold: self.bold.unwrap_or(base.bold),
            italic: self.italic.unwrap_or(base.italic),
            underline: self.underline.unwrap_or(base.underline),
            strike: self.strike.unwrap_or(base.strike),
            size_half_pt: self.size.unwrap_or(base.size_half_pt),
            color: self.color.clone().unwrap_or(base.color),
            font: self.font.clone().unwrap_or(base.font),
            vert_align: self.vert.clone().unwrap_or(base.vert_align),
            highlight: self.highlight.clone().unwrap_or(base.highlight),
        }
    }
}

#[derive(Clone, Debug, Default)]
struct ParaPatch {
    align: Option<String>,
    indent_left: Option<i32>,
    indent_right: Option<i32>,
    indent_first: Option<i32>,
    space_before: Option<i32>,
    space_after: Option<i32>,
    line: Option<i32>,
    line_rule: Option<String>,
    num_id: Option<String>,
    num_level: Option<u8>,
    heading_level: Option<u8>,
    keep_next: Option<bool>,
    page_break_before: Option<bool>,
    run: RunPatch,
}

impl ParaPatch {
    fn overlay(&mut self, top: &ParaPatch) {
        if top.align.is_some() {
            self.align = top.align.clone();
        }
        if top.indent_left.is_some() {
            self.indent_left = top.indent_left;
        }
        if top.indent_right.is_some() {
            self.indent_right = top.indent_right;
        }
        if top.indent_first.is_some() {
            self.indent_first = top.indent_first;
        }
        if top.space_before.is_some() {
            self.space_before = top.space_before;
        }
        if top.space_after.is_some() {
            self.space_after = top.space_after;
        }
        if top.line.is_some() {
            self.line = top.line;
        }
        if top.line_rule.is_some() {
            self.line_rule = top.line_rule.clone();
        }
        if top.num_id.is_some() {
            self.num_id = top.num_id.clone();
        }
        if top.num_level.is_some() {
            self.num_level = top.num_level;
        }
        if top.heading_level.is_some() {
            self.heading_level = top.heading_level;
        }
        if top.keep_next.is_some() {
            self.keep_next = top.keep_next;
        }
        if top.page_break_before.is_some() {
            self.page_break_before = top.page_break_before;
        }
        self.run.overlay(&top.run);
    }
}

#[derive(Clone, Debug, Default)]
struct StyleDef {
    based_on: Option<String>,
    para: ParaPatch,
    run: RunPatch,
}

#[derive(Clone, Debug, Default)]
struct StyleSheet {
    def_para: ParaPatch,
    def_run: RunPatch,
    styles: HashMap<String, StyleDef>,
}

impl StyleSheet {
    /// 沿 basedOn 链从根到叶叠加，再叠题目自己的补丁。
    fn resolve_style(&self, style_id: &str) -> ParaPatch {
        let mut chain: Vec<&StyleDef> = Vec::new();
        let mut cursor = Some(style_id.to_string());
        let mut guard = 0;
        while let Some(id) = cursor {
            guard += 1;
            if guard > 32 {
                break;
            }
            let Some(def) = self.styles.get(&id) else {
                break;
            };
            chain.push(def);
            cursor = def.based_on.clone();
        }

        let mut merged = ParaPatch::default();
        merged.run.overlay(&self.def_run);
        merged.overlay(&ParaPatch {
            run: self.def_run.clone(),
            ..ParaPatch::default()
        });
        for def in chain.into_iter().rev() {
            merged.overlay(&def.para);
            merged.run.overlay(&def.run);
        }
        merged
    }
}

// ── 解析入口 ──

pub fn parse_rendered_doc(docx: &[u8]) -> Result<RenderedDoc, EngineError> {
    validate_docx_bytes(docx)?;

    let mut archive = ZipArchive::new(Cursor::new(docx))
        .map_err(|_| EngineError::invalid_input("打开 .docx 归档失败"))?;

    let styles_bytes = read_entry(&mut archive, "word/styles.xml")?;
    let doc_bytes = read_entry(&mut archive, "word/document.xml")?
        .ok_or_else(|| EngineError::invalid_input("缺少 word/document.xml"))?;

    let sheet = styles_bytes
        .as_deref()
        .map(parse_styles)
        .unwrap_or_default();

    // 图片关系表：拿不到就空表，正文照常渲染（图不显示）
    let rels = crate::doc::media::load_document_rels(docx);

    let mut walker = Walker::new(&sheet, &rels);
    walker.run(&doc_bytes)?;
    Ok(walker.finish())
}

fn read_entry(
    archive: &mut ZipArchive<Cursor<&[u8]>>,
    name: &str,
) -> Result<Option<Vec<u8>>, EngineError> {
    let entry = match archive.by_name(name) {
        Ok(f) => f,
        Err(zip::result::ZipError::FileNotFound) => return Ok(None),
        Err(_) => return Err(EngineError::invalid_input("读取 .docx 内部文件失败")),
    };
    if entry.size() > MAX_XML_UNCOMPRESSED_BYTES
        || entry.size() > MAX_TOTAL_UNCOMPRESSED_BYTES
    {
        return Err(EngineError::invalid_input("文档 XML 条目超过安全体积限制"));
    }
    let mut buf = Vec::new();
    entry
        .take(MAX_XML_UNCOMPRESSED_BYTES.saturating_add(1))
        .read_to_end(&mut buf)
        .map_err(|_| EngineError::invalid_input("解压文档 XML 内容失败"))?;
    if (buf.len() as u64) > MAX_XML_UNCOMPRESSED_BYTES {
        return Err(EngineError::invalid_input("文档 XML 实际解压体积超限"));
    }
    Ok(Some(buf))
}

fn parse_styles(xml: &[u8]) -> StyleSheet {
    let mut sheet = StyleSheet::default();
    let mut reader = Reader::from_reader(xml);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();

    let mut cur_id: Option<String> = None;
    let mut cur_def = StyleDef::default();
    let mut scope: Vec<Scope> = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                apply_style_event(e, true, &mut sheet, &mut cur_id, &mut cur_def, &mut scope);
            }
            Ok(Event::Empty(ref e)) => {
                apply_style_event(e, false, &mut sheet, &mut cur_id, &mut cur_def, &mut scope);
            }
            Ok(Event::End(ref e)) => {
                let name = e.local_name();
                match name.as_ref() {
                    b"style" => {
                        if let Some(id) = cur_id.take() {
                            sheet.styles.insert(id, std::mem::take(&mut cur_def));
                        }
                        scope.clear();
                    }
                    b"pPr" | b"rPr" => {
                        scope.pop();
                    }
                    _ => {}
                }
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
        buf.clear();
    }

    sheet
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)]
enum Scope {
    Style,
    ParaProps,
    RunProps,
}

fn apply_style_event(
    e: &BytesStart<'_>,
    is_start: bool,
    sheet: &mut StyleSheet,
    cur_id: &mut Option<String>,
    cur_def: &mut StyleDef,
    scope: &mut Vec<Scope>,
) {
    let name = e.local_name();
    let local = name.as_ref();
    match local {
        b"style" => {
            *cur_id = attr_val(e, b"styleId");
            *cur_def = StyleDef::default();
            scope.clear();
            if is_start {
                scope.push(Scope::Style);
            }
        }
        b"basedOn" if cur_id.is_some() => {
            if let Some(v) = attr_val(e, b"val") {
                cur_def.based_on = Some(v);
            }
        }
        b"pPr" => {
            if is_start {
                scope.push(Scope::ParaProps);
            }
        }
        // <w:style><w:name w:val="heading 1"/>：WPS/Word 常把 styleId 写成 "1"，级次只在 name 里
        b"name" if cur_id.is_some() && scope.last() == Some(&Scope::Style) => {
            if cur_def.para.heading_level.is_none() {
                if let Some(v) = attr_val(e, b"val") {
                    if let Some(lvl) = heading_from_name(&v) {
                        cur_def.para.heading_level = Some(lvl);
                    }
                }
            }
        }
        b"rPr" => {
            if is_start {
                scope.push(Scope::RunProps);
            }
        }
        _ => {
            let in_para = matches!(scope.last(), Some(Scope::ParaProps));
            let in_run = matches!(scope.last(), Some(Scope::RunProps));
            if in_para {
                if let Some(patch) = take_para_prop(e, local) {
                    if cur_id.is_some() {
                        cur_def.para.overlay(&patch);
                    } else {
                        sheet.def_para.overlay(&patch);
                    }
                } else if let Some(run) = take_run_prop(e, local) {
                    if cur_id.is_some() {
                        cur_def.run.overlay(&run);
                    } else {
                        sheet.def_run.overlay(&run);
                    }
                }
            } else if in_run {
                if let Some(run) = take_run_prop(e, local) {
                    if cur_id.is_some() {
                        cur_def.run.overlay(&run);
                    } else {
                        sheet.def_run.overlay(&run);
                    }
                }
            }
        }
    }
}

fn take_para_prop(e: &BytesStart<'_>, local: &[u8]) -> Option<ParaPatch> {
    let mut patch = ParaPatch::default();
    match local {
        b"jc" => {
            patch.align = attr_val(e, b"val").map(|v| normalize_align(&v));
        }
        b"ind" => {
            if let Some(v) = attr_val(e, b"left").and_then(|v| v.parse::<i32>().ok()) {
                patch.indent_left = Some(v);
            }
            if let Some(v) = attr_val(e, b"start").and_then(|v| v.parse::<i32>().ok()) {
                patch.indent_left = Some(v);
            }
            if let Some(v) = attr_val(e, b"right").and_then(|v| v.parse::<i32>().ok()) {
                patch.indent_right = Some(v);
            }
            if let Some(v) = attr_val(e, b"end").and_then(|v| v.parse::<i32>().ok()) {
                patch.indent_right = Some(v);
            }
            if let Some(v) = attr_val(e, b"firstLine").and_then(|v| v.parse::<i32>().ok()) {
                patch.indent_first = Some(v);
            }
            if let Some(v) = attr_val(e, b"hanging").and_then(|v| v.parse::<i32>().ok()) {
                patch.indent_first = Some(-v);
            }
        }
        b"spacing" => {
            if let Some(v) = attr_val(e, b"before").and_then(|v| v.parse::<i32>().ok()) {
                patch.space_before = Some(v);
            }
            if let Some(v) = attr_val(e, b"after").and_then(|v| v.parse::<i32>().ok()) {
                patch.space_after = Some(v);
            }
            if let Some(v) = attr_val(e, b"line").and_then(|v| v.parse::<i32>().ok()) {
                patch.line = Some(v);
            }
            if let Some(v) = attr_val(e, b"lineRule") {
                patch.line_rule = Some(normalize_line_rule(&v));
            }
        }
        b"outlineLvl" => {
            if let Some(v) = attr_val(e, b"val").and_then(|v| v.parse::<i32>().ok()) {
                if (0..9).contains(&v) {
                    patch.heading_level = Some((v + 1).min(9) as u8);
                } else {
                    patch.heading_level = Some(0);
                }
            }
        }
        b"keepNext" => {
            patch.keep_next = Some(flag_on(e));
        }
        b"pageBreakBefore" => {
            patch.page_break_before = Some(flag_on(e));
        }
        _ => return None,
    }
    Some(patch)
}

fn take_run_prop(e: &BytesStart<'_>, local: &[u8]) -> Option<RunPatch> {
    let mut patch = RunPatch::default();
    match local {
        b"b" => patch.bold = Some(flag_on(e)),
        b"i" => patch.italic = Some(flag_on(e)),
        b"u" => patch.underline = Some(attr_val(e, b"val").map(|v| v != "none").unwrap_or(true)),
        b"strike" => patch.strike = Some(flag_on(e)),
        b"sz" => {
            if let Some(v) = attr_val(e, b"val").and_then(|v| v.parse::<u16>().ok()) {
                if v > 0 {
                    patch.size = Some(v);
                }
            }
        }
        b"color" => {
            if let Some(v) = attr_val(e, b"val") {
                let clean = v.trim().trim_start_matches('#').to_ascii_uppercase();
                if clean.len() == 6 && clean.chars().all(|c| c.is_ascii_hexdigit()) {
                    patch.color = Some(clean);
                } else {
                    patch.color = None;
                }
            }
        }
        b"rFonts" => {
            let name = attr_val(e, b"eastAsia")
                .or_else(|| attr_val(e, b"ascii"))
                .or_else(|| attr_val(e, b"hAnsi"));
            if let Some(name) = name {
                if !name.is_empty() {
                    patch.font = Some(name);
                }
            }
        }
        b"vertAlign" => {
            if let Some(v) = attr_val(e, b"val") {
                let v = v.to_ascii_lowercase();
                if v == "superscript" || v == "subscript" {
                    patch.vert = Some(v);
                } else {
                    patch.vert = Some(String::new());
                }
            }
        }
        b"highlight" => {
            if let Some(v) = attr_val(e, b"val") {
                patch.highlight = Some(v.to_ascii_lowercase());
            }
        }
        b"shd" => {
            if let Some(v) = attr_val(e, b"fill") {
                let clean = v.trim().trim_start_matches('#').to_ascii_uppercase();
                if clean.len() == 6 && clean != "FFFFFF" {
                    patch.highlight = Some(clean);
                }
            }
        }
        _ => return None,
    }
    Some(patch)
}

fn flag_on(e: &BytesStart<'_>) -> bool {
    match attr_val(e, b"val") {
        None => true,
        Some(v) => {
            let v = v.trim().to_ascii_lowercase();
            !(v == "0" || v == "false" || v == "off" || v == "none")
        }
    }
}

fn normalize_align(raw: &str) -> String {
    match raw.trim().to_ascii_lowercase().as_str() {
        "center" => "center",
        "right" | "end" => "right",
        "both" | "justify" => "justify",
        "distribute" => "distribute",
        _ => "left",
    }
    .to_string()
}

/// EMU -> CSS px（1px = 9525 EMU）
fn emu_to_px(emu: i64) -> u32 {
    if emu <= 0 {
        return 0;
    }
    (emu as f64 / 9525.0).round() as u32
}

fn normalize_line_rule(raw: &str) -> String {
    match raw.trim().to_ascii_lowercase().as_str() {
        "exact" => "exact",
        "atleast" => "atLeast",
        _ => "auto",
    }
    .to_string()
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

// ── 走 document.xml ──

struct Walker<'a> {
    sheet: &'a StyleSheet,
    /// rId -> 包内图片路径
    rels: &'a HashMap<String, String>,
    page: PageSetup,
    blocks: Vec<RenderBlock>,
    para_count: usize,
    table_count: usize,
    page_break_count: usize,

    p_ord: usize,
    heading_stack: Vec<(u8, String)>,

    cur_para: Option<ParaBuilder>,
    cur_table: Option<TableBuilder>,
    cur_row: Option<TableRow>,
    cur_cell: Option<TableCell>,
    cur_run: Option<RunSpan>,

    in_body: bool,
    p_depth: usize,
    tbl_depth: usize,
    tc_depth: usize,
    r_depth: usize,
    tblpr_depth: usize,
    ppr_depth: usize,
    rpr_depth: usize,
    del_depth: usize,
    fallback_depth: usize,
    in_text: bool,
    /// 段内出现 `<w:br w:type="page"/>`，flush 时补一个 page_break 块
    pending_page_break: bool,
    /// drawing / pict 深度；>0 时正文解析暂停
    drawing_depth: usize,
    /// 正在收的图：(rId, 宽 px, 高 px)
    cur_image: Option<(String, u32, u32)>,
}

struct ParaBuilder {
    ord: usize,
    style_id: String,
    direct: ParaPatch,
    runs: Vec<RunSpan>,
}

struct TableBuilder {
    first_ord: usize,
    grid: Vec<i32>,
    rows: Vec<TableRow>,
    width: i32,
    align: String,
}

impl<'a> Walker<'a> {
    fn new(sheet: &'a StyleSheet, rels: &'a HashMap<String, String>) -> Self {
        Self {
            sheet,
            rels,
            page: PageSetup::default(),
            blocks: Vec::new(),
            para_count: 0,
            table_count: 0,
            page_break_count: 0,
            p_ord: 0,
            heading_stack: Vec::new(),
            cur_para: None,
            cur_table: None,
            cur_row: None,
            cur_cell: None,
            cur_run: None,
            in_body: false,
            p_depth: 0,
            tbl_depth: 0,
            tc_depth: 0,
            r_depth: 0,
            tblpr_depth: 0,
            ppr_depth: 0,
            rpr_depth: 0,
            del_depth: 0,
            fallback_depth: 0,
            in_text: false,
            pending_page_break: false,
            drawing_depth: 0,
            cur_image: None,
        }
    }

    fn run(&mut self, xml: &[u8]) -> Result<(), EngineError> {
        let mut reader = Reader::from_reader(xml);
        reader.config_mut().trim_text(false);
        let mut buf = Vec::new();

        loop {
            let ev = reader
                .read_event_into(&mut buf)
                .map_err(|_| EngineError::invalid_input("word/document.xml 解析失败，XML 结构损坏"))?;

            match ev {
                Event::Eof => break,
                Event::Start(ref e) => self.on_start(e),
                Event::Empty(ref e) => self.on_empty(e),
                Event::Text(ref e) => self.on_text(e),
                Event::End(ref e) => self.on_end(e),
                _ => {}
            }
            buf.clear();
        }
        Ok(())
    }

    fn on_start(&mut self, e: &BytesStart<'_>) {
        let name = e.local_name();
        let local = name.as_ref();

        if local == b"body" {
            self.in_body = true;
            return;
        }
        if !self.in_body {
            return;
        }
        if local == b"Fallback" {
            self.fallback_depth += 1;
            return;
        }
        if self.fallback_depth > 0 {
            return;
        }
        if local == b"del" {
            self.del_depth += 1;
            return;
        }
        if self.del_depth > 0 {
            return;
        }

        // 图片：drawing / pict 子树整体接管，不参与正文解析
        if self.on_image_tag(local, e) {
            return;
        }

        // 表格级对齐：<w:tblPr><w:jc w:val="center"/></w:tblPr>
        if local == b"jc" && self.tblpr_depth > 0 {
            if let Some(ref mut t) = self.cur_table {
                t.align = normalize_align(&attr_val(e, b"val").unwrap_or_default());
            }
            return;
        }

        match local {
            b"p" => {
                self.p_ord += 1;
                self.p_depth += 1;
                let ord = self.p_ord - 1;
                if self.cur_table.is_some() && self.cur_row.is_none() {
                    // 表格前导段落，忽略
                }
                self.cur_para = Some(ParaBuilder {
                    ord,
                    style_id: String::new(),
                    direct: ParaPatch::default(),
                    runs: Vec::new(),
                });
            }
            b"pPr" if self.p_depth > 0 => {
                self.ppr_depth += 1;
            }
            b"rPr" if self.ppr_depth > 0 || self.p_depth > 0 => {
                self.rpr_depth += 1;
            }
            b"r" if self.p_depth > 0 => {
                self.r_depth += 1;
                self.cur_run = Some(RunSpan {
                    text: String::new(),
                    fmt: RunFmt::default(),
                    image: None,
                });
            }
            b"t" if self.r_depth > 0 => {
                self.in_text = true;
            }
            b"pStyle" if self.ppr_depth > 0 => {
                if let Some(ref mut p) = self.cur_para {
                    if let Some(v) = attr_val(e, b"val") {
                        p.style_id = v;
                    }
                }
            }
            b"tbl" if self.tbl_depth == 0 => {
                self.tbl_depth += 1;
                self.cur_table = Some(TableBuilder {
                    first_ord: self.p_ord,
                    grid: Vec::new(),
                    rows: Vec::new(),
                    width: 0,
                    align: String::new(),
                });
            }
            b"tblGrid" => {}
            b"tr" if self.tbl_depth > 0 => {
                self.cur_row = Some(TableRow::default());
            }
            b"tc" if self.cur_row.is_some() => {
                self.tc_depth += 1;
                self.cur_cell = Some(TableCell::default());
            }
            _ => {
                self.on_prop_start(e, local);
            }
        }
    }

    fn on_prop_start(&mut self, e: &BytesStart<'_>, local: &[u8]) {
        // 段落属性
        if self.ppr_depth > 0 && self.rpr_depth == 0 {
            if let Some(patch) = take_para_prop(e, local) {
                if let Some(ref mut p) = self.cur_para {
                    p.direct.overlay(&patch);
                }
                return;
            }
            if let Some(run) = take_run_prop(e, local) {
                if let Some(ref mut p) = self.cur_para {
                    p.direct.run.overlay(&run);
                }
                return;
            }
        }
        // run 属性
        if self.rpr_depth > 0 {
            if let Some(run) = take_run_prop(e, local) {
                if let Some(ref mut span) = self.cur_run {
                    let mut patch = RunPatch::default();
                    patch.overlay(&run);
                    apply_run_patch(&mut span.fmt, &patch);
                }
                return;
            }
        }
        // 列表编号
        if local == b"numPr" {
            if let Some(ref mut p) = self.cur_para {
                p.direct.num_id = Some(String::new());
            }
            return;
        }
        if local == b"ilvl" && self.ppr_depth > 0 {
            if let Some(v) = attr_val(e, b"val").and_then(|v| v.parse::<u8>().ok()) {
                if let Some(ref mut p) = self.cur_para {
                    p.direct.num_level = Some(v);
                }
            }
            return;
        }
        if local == b"numId" && self.ppr_depth > 0 {
            if let Some(v) = attr_val(e, b"val") {
                if let Some(ref mut p) = self.cur_para {
                    p.direct.num_id = Some(v);
                }
            }
            return;
        }
        // 表格结构
        if local == b"gridCol" && self.tbl_depth > 0 {
            if let Some(w) = attr_val(e, b"w").and_then(|v| v.parse::<i32>().ok()) {
                if let Some(ref mut t) = self.cur_table {
                    t.grid.push(w);
                }
            }
            return;
        }
        if local == b"trHeight" && self.cur_row.is_some() {
            if let Some(v) = attr_val(e, b"val").and_then(|v| v.parse::<i32>().ok()) {
                if let Some(ref mut row) = self.cur_row {
                    row.height_twips = v;
                }
            }
            return;
        }
        if local == b"tblHeader" && self.cur_row.is_some() {
            if let Some(ref mut row) = self.cur_row {
                row.header = flag_on(e);
            }
            return;
        }
        if local == b"gridSpan" && self.cur_cell.is_some() {
            if let Some(v) = attr_val(e, b"val").and_then(|v| v.parse::<u16>().ok()) {
                if let Some(ref mut c) = self.cur_cell {
                    c.grid_span = v.max(1);
                }
            }
            return;
        }
        if local == b"vMerge" && self.cur_cell.is_some() {
            let val = attr_val(e, b"val").unwrap_or_else(|| String::from("continue"));
            if let Some(ref mut c) = self.cur_cell {
                c.v_merge = if val == "restart" {
                    String::from("restart")
                } else {
                    String::from("continue")
                };
            }
            return;
        }
        if local == b"tcW" && self.cur_cell.is_some() {
            if let Some(w) = attr_val(e, b"w").and_then(|v| v.parse::<i32>().ok()) {
                if let Some(ref mut c) = self.cur_cell {
                    c.width_twips = w;
                }
            }
            return;
        }
        if local == b"tblW" && self.tbl_depth > 0 {
            if let Some(w) = attr_val(e, b"w").and_then(|v| v.parse::<i32>().ok()) {
                if let Some(ref mut t) = self.cur_table {
                    t.width = w;
                }
            }
            return;
        }
        if local == b"tblPr" {
            self.tblpr_depth += 1;
            self.probe_table_props(e);
            return;
        }
        if local == b"sectPr" {
            self.read_section(e);
            return;
        }
        if local == b"pgSz" {
            self.read_section(e);
            return;
        }
        if local == b"pgMar" {
            self.read_section(e);
            return;
        }
        if local == b"br" && self.r_depth > 0 {
            let kind = attr_val(e, b"type").unwrap_or_default();
            if kind == "page" {
                // 段落内分页符：等本段 flush 时统一产出 page_break 块
                self.pending_page_break = true;
            } else if let Some(ref mut span) = self.cur_run {
                span.text.push('\n');
            }
            return;
        }
        if local == b"tab" && self.r_depth > 0 {
            if let Some(ref mut span) = self.cur_run {
                span.text.push('\t');
            }
            return;
        }
        if local == b"cr" && self.r_depth > 0 {
            if let Some(ref mut span) = self.cur_run {
                span.text.push('\n');
            }
        }
    }

    fn probe_table_props(&mut self, e: &BytesStart<'_>) {
        // tblPr 自身可能带 jc
        if let Some(v) = attr_val(e, b"jc") {
            if let Some(ref mut t) = self.cur_table {
                t.align = normalize_align(&v);
            }
        }
    }

    /// 图片相关标签统一入口。返回 true 表示已消费，正文解析跳过该标签。
    fn on_image_tag(&mut self, local: &[u8], e: &BytesStart<'_>) -> bool {
        if local == b"drawing" || local == b"pict" {
            self.drawing_depth += 1;
            self.cur_image = None;
            return true;
        }
        if self.drawing_depth == 0 {
            return false;
        }
        match local {
            b"extent" => {
                let cx = attr_val(e, b"cx").and_then(|v| v.parse::<i64>().ok()).unwrap_or(0);
                let cy = attr_val(e, b"cy").and_then(|v| v.parse::<i64>().ok()).unwrap_or(0);
                let rel = self.cur_image.take().map(|(r, _, _)| r).unwrap_or_default();
                let (w, h) = if cx > 0 && cy > 0 {
                    (emu_to_px(cx), emu_to_px(cy))
                } else {
                    (0, 0)
                };
                self.cur_image = Some((rel, w, h));
            }
            b"blip" | b"imagedata" => {
                let mut rid = String::new();
                for attr in e.attributes().flatten() {
                    let k = attr.key.local_name();
                    let k = k.as_ref();
                    if k == b"embed" || k == b"id" {
                        rid = attr.unescape_value().unwrap_or_default().to_string();
                        break;
                    }
                }
                if !rid.is_empty() {
                    let (_, w, h) = self.cur_image.take().unwrap_or_default();
                    self.cur_image = Some((rid, w, h));
                }
            }
            _ => {}
        }
        true
    }

    /// 把已收齐的 (rId, 尺寸) 换成可直出的图片引用；关系表里找不到就丢弃。
    fn take_image(&mut self) -> Option<ImageRef> {
        let (rid, w, h) = self.cur_image.take()?;
        let path = self.rels.get(&rid)?.clone();
        Some(ImageRef {
            path,
            width_px: w,
            height_px: h,
        })
    }

    fn on_empty(&mut self, e: &BytesStart<'_>) {
        let name = e.local_name();
        let local = name.as_ref();

        if !self.in_body || self.fallback_depth > 0 {
            return;
        }
        if local == b"p" {
            self.p_ord += 1;
            self.push_blank_para(self.p_ord - 1);
            return;
        }
        if self.del_depth > 0 {
            return;
        }

        // 图片：extent / blip / imagedata 多是 Empty 形态
        if self.on_image_tag(local, e) {
            return;
        }

        // 表格级对齐（Empty 形态）
        if local == b"jc" && self.tblpr_depth > 0 {
            if let Some(ref mut t) = self.cur_table {
                t.align = normalize_align(&attr_val(e, b"val").unwrap_or_default());
            }
            return;
        }
        // pStyle 只会以 Empty 形态出现，且它不在 ParaPatch 里，单独取
        if local == b"pStyle" && self.ppr_depth > 0 {
            if let Some(ref mut p) = self.cur_para {
                if let Some(v) = attr_val(e, b"val") {
                    p.style_id = v;
                }
            }
            return;
        }
        // Empty 形态的段落属性统一走 take_para_prop，避免逐标签列举漏掉（numId/ilvl 等）
        if self.ppr_depth > 0 && self.rpr_depth == 0 {
            if let Some(patch) = take_para_prop(e, local) {
                if let Some(ref mut p) = self.cur_para {
                    p.direct.overlay(&patch);
                }
                return;
            }
        }
        if let Some(run) = take_run_prop(e, local) {
            if self.rpr_depth > 0 {
                if let Some(ref mut span) = self.cur_run {
                    apply_run_patch(&mut span.fmt, &run);
                }
            } else if self.ppr_depth > 0 {
                if let Some(ref mut p) = self.cur_para {
                    p.direct.run.overlay(&run);
                }
            }
            return;
        }
        self.on_prop_start(e, local);
    }

    fn on_text(&mut self, e: &quick_xml::events::BytesText<'_>) {
        if !self.in_body || self.fallback_depth > 0 || self.del_depth > 0 || self.drawing_depth > 0
        {
            return;
        }
        if !self.in_text {
            return;
        }
        if let Some(ref mut span) = self.cur_run {
            if let Ok(text) = e.unescape() {
                span.text.push_str(&text);
            }
        }
    }

    fn on_end(&mut self, e: &quick_xml::events::BytesEnd<'_>) {
        let name = e.local_name();
        let local = name.as_ref();

        if local == b"body" {
            self.in_body = false;
            return;
        }
        if local == b"Fallback" {
            self.fallback_depth = self.fallback_depth.saturating_sub(1);
            return;
        }
        if self.fallback_depth > 0 {
            return;
        }
        if local == b"del" {
            self.del_depth = self.del_depth.saturating_sub(1);
            return;
        }
        if local == b"drawing" || local == b"pict" {
            self.drawing_depth = self.drawing_depth.saturating_sub(1);
            if self.drawing_depth == 0 {
                if let Some(img) = self.take_image() {
                    if let Some(ref mut run) = self.cur_run {
                        if run.image.is_none() {
                            run.image = Some(img);
                        }
                    }
                }
            }
            return;
        }

        match local {
            b"t" => self.in_text = false,
            b"rPr" => {
                self.rpr_depth = self.rpr_depth.saturating_sub(1);
            }
            b"pPr" => {
                self.ppr_depth = self.ppr_depth.saturating_sub(1);
            }
            b"tblPr" => {
                self.tblpr_depth = self.tblpr_depth.saturating_sub(1);
            }
            b"r" => {
                self.r_depth = self.r_depth.saturating_sub(1);
                if let Some(span) = self.cur_run.take() {
                    if let Some(ref mut p) = self.cur_para {
                        if !span.text.is_empty() || span.image.is_some() {
                            p.runs.push(span);
                        }
                    }
                }
            }
            b"p" => {
                self.p_depth = self.p_depth.saturating_sub(1);
                self.flush_para();
            }
            b"tc" => {
                self.tc_depth = self.tc_depth.saturating_sub(1);
                if let Some(mut cell) = self.cur_cell.take() {
                    if cell.grid_span == 0 {
                        cell.grid_span = 1;
                    }
                    if let Some(ref mut row) = self.cur_row {
                        row.cells.push(cell);
                    }
                }
            }
            b"tr" => {
                if let Some(row) = self.cur_row.take() {
                    if let Some(ref mut t) = self.cur_table {
                        if !row.cells.is_empty() {
                            t.rows.push(row);
                        }
                    }
                }
            }
            b"tblGrid" => {}
            b"tbl" => {
                self.tbl_depth = self.tbl_depth.saturating_sub(1);
                self.flush_table();
            }
            _ => {}
        }
    }

    fn push_blank_para(&mut self, ord: usize) {
        if let Some(ref mut cell) = self.cur_cell {
            cell.paras.push(RPara {
                ord,
                fmt: ParaFmt::default(),
                runs: Vec::new(),
            });
            return;
        }
        if self.cur_table.is_some() {
            return;
        }
        self.blocks.push(RenderBlock {
            kind: String::from("para"),
            ord,
            anchor: format!("p:{ord}"),
            heading_level: 0,
            heading_path: self.heading_path_text(),
            para: Some(RPara {
                ord,
                fmt: ParaFmt::default(),
                runs: Vec::new(),
            }),
            table: None,
        });
        self.para_count += 1;
    }

    fn flush_para(&mut self) {
        let Some(para) = self.cur_para.take() else {
            return;
        };

        let mut merged = self.sheet.resolve_style(&para.style_id);
        merged.overlay(&para.direct);

        let mut fmt = ParaFmt {
            style_id: para.style_id.clone(),
            ..ParaFmt::default()
        };
        fmt.align = merged.align.clone().unwrap_or_default();
        fmt.indent_left_twips = merged.indent_left.unwrap_or(0);
        fmt.indent_right_twips = merged.indent_right.unwrap_or(0);
        fmt.indent_first_line_twips = merged.indent_first.unwrap_or(0);
        fmt.space_before_twips = merged.space_before.unwrap_or(0);
        fmt.space_after_twips = merged.space_after.unwrap_or(0);
        fmt.line_twips = merged.line.unwrap_or(0);
        fmt.line_rule = merged
            .line_rule
            .clone()
            .unwrap_or_else(|| String::from("auto"));
        fmt.num_id = merged.num_id.clone().unwrap_or_default();
        fmt.num_level = merged.num_level.unwrap_or(0);
        fmt.heading_level = merged.heading_level.unwrap_or(0);
        fmt.keep_next = merged.keep_next.unwrap_or(false);
        fmt.page_break_before = merged.page_break_before.unwrap_or(false);

        if fmt.heading_level == 0 {
            if let Some(lvl) = heading_from_style(&para.style_id) {
                fmt.heading_level = lvl;
            }
        }

        let base_run = merged.run.resolve();
        let mut runs: Vec<RunSpan> = Vec::with_capacity(para.runs.len());
        for span in para.runs {
            let raw = span.text;
            let image = span.image;
            // run 的直接格式已在收集阶段写进 span.fmt（以默认值起步），
            // 这里再与段落基准做一次合并：段落基准优先补空缺，已有值保留。
            let merged_fmt = merge_run(&base_run, &span.fmt);
            runs.push(RunSpan {
                text: raw,
                fmt: merged_fmt,
                image,
            });
        }

        let rpara = RPara {
            ord: para.ord,
            fmt: fmt.clone(),
            runs,
        };

        if let Some(ref mut cell) = self.cur_cell {
            cell.paras.push(rpara);
            return;
        }
        if self.cur_table.is_some() {
            // 表格间的游离段落，直接丢弃（表格结构内不混排）
            return;
        }

        if fmt.heading_level > 0 {
            let text = rpara
                .runs
                .iter()
                .map(|r| r.text.as_str())
                .collect::<String>();
            let text = text.trim().to_string();
            while self
                .heading_stack
                .last()
                .map(|(l, _)| *l >= fmt.heading_level)
                .unwrap_or(false)
            {
                self.heading_stack.pop();
            }
            self.heading_stack.push((fmt.heading_level, text));
        }

        let emit_break = if self.pending_page_break {
            self.pending_page_break = false;
            true
        } else {
            fmt.page_break_before
        };

        if emit_break {
            self.page_break_count += 1;
            self.blocks.push(RenderBlock {
                kind: String::from("page_break"),
                ord: para.ord,
                anchor: format!("p:{}", para.ord),
                heading_level: 0,
                heading_path: self.heading_path_text(),
                para: None,
                table: None,
            });
        }

        // 纯分页段落（只有 br，没有文字）不再额外输出空行
        let has_text = rpara.runs.iter().any(|r| !r.text.is_empty());
        if !has_text && emit_break {
            return;
        }

        let level = fmt.heading_level;
        self.blocks.push(RenderBlock {
            kind: String::from("para"),
            ord: para.ord,
            anchor: format!("p:{}", para.ord),
            heading_level: level,
            heading_path: self.heading_path_text(),
            para: Some(rpara),
            table: None,
        });
        self.para_count += 1;
    }

    fn flush_table(&mut self) {
        let Some(tbl) = self.cur_table.take() else {
            return;
        };
        if tbl.rows.is_empty() {
            return;
        }
        self.table_count += 1;
        self.blocks.push(RenderBlock {
            kind: String::from("table"),
            ord: tbl.first_ord,
            anchor: format!("p:{}", tbl.first_ord),
            heading_level: 0,
            heading_path: self.heading_path_text(),
            para: None,
            table: Some(RTable {
                first_ord: tbl.first_ord,
                grid: tbl.grid,
                rows: tbl.rows,
                width_twips: tbl.width,
                align: tbl.align,
            }),
        });
    }

    fn heading_path_text(&self) -> String {
        if self.heading_stack.is_empty() {
            return String::from("文档正文");
        }
        self.heading_stack
            .iter()
            .map(|(_, t)| t.clone())
            .collect::<Vec<_>>()
            .join(" / ")
    }

    fn read_section(&mut self, e: &BytesStart<'_>) {
        let local = e.local_name();
        match local.as_ref() {
            b"pgSz" => {
                if let Some(w) = attr_val(e, b"w").and_then(|v| v.parse::<i32>().ok()) {
                    self.page.width_twips = w;
                }
                if let Some(h) = attr_val(e, b"h").and_then(|v| v.parse::<i32>().ok()) {
                    self.page.height_twips = h;
                }
                let orient = attr_val(e, b"orient").unwrap_or_default();
                self.page.landscape = orient == "landscape";
            }
            b"pgMar" => {
                if let Some(v) = attr_val(e, b"top").and_then(|v| v.parse::<i32>().ok()) {
                    self.page.margin_top_twips = v;
                }
                if let Some(v) = attr_val(e, b"right").and_then(|v| v.parse::<i32>().ok()) {
                    self.page.margin_right_twips = v;
                }
                if let Some(v) = attr_val(e, b"bottom").and_then(|v| v.parse::<i32>().ok()) {
                    self.page.margin_bottom_twips = v;
                }
                if let Some(v) = attr_val(e, b"left").and_then(|v| v.parse::<i32>().ok()) {
                    self.page.margin_left_twips = v;
                }
                if let Some(v) = attr_val(e, b"header").and_then(|v| v.parse::<i32>().ok()) {
                    self.page.header_twips = v;
                }
                if let Some(v) = attr_val(e, b"footer").and_then(|v| v.parse::<i32>().ok()) {
                    self.page.footer_twips = v;
                }
                if let Some(v) = attr_val(e, b"gutter").and_then(|v| v.parse::<i32>().ok()) {
                    self.page.gutter_twips = v;
                }
            }
            _ => {}
        }
    }

    fn finish(mut self) -> RenderedDoc {
        if self.cur_para.is_some() {
            self.flush_para();
        }
        if self.cur_table.is_some() {
            self.flush_table();
        }
        RenderedDoc {
            page: self.page,
            blocks: self.blocks,
            para_count: self.para_count,
            table_count: self.table_count,
            page_break_count: self.page_break_count,
        }
    }
}

fn apply_run_patch(fmt: &mut RunFmt, patch: &RunPatch) {
    if let Some(v) = patch.bold {
        fmt.bold = v;
    }
    if let Some(v) = patch.italic {
        fmt.italic = v;
    }
    if let Some(v) = patch.underline {
        fmt.underline = v;
    }
    if let Some(v) = patch.strike {
        fmt.strike = v;
    }
    if let Some(v) = patch.size {
        fmt.size_half_pt = v;
    }
    if let Some(v) = patch.color.clone() {
        fmt.color = v;
    }
    if let Some(v) = patch.font.clone() {
        fmt.font = v;
    }
    if let Some(v) = patch.vert.clone() {
        fmt.vert_align = v;
    }
    if let Some(v) = patch.highlight.clone() {
        fmt.highlight = v;
    }
}

/// 段落基准与 run 直接格式合并：run 若仍是默认占位，则采用段落基准。
fn merge_run(base: &RunFmt, run: &RunFmt) -> RunFmt {
    let default = RunFmt::default();
    RunFmt {
        bold: if run.bold != default.bold {
            run.bold
        } else {
            base.bold
        },
        italic: if run.italic != default.italic {
            run.italic
        } else {
            base.italic
        },
        underline: if run.underline != default.underline {
            run.underline
        } else {
            base.underline
        },
        strike: if run.strike != default.strike {
            run.strike
        } else {
            base.strike
        },
        size_half_pt: if run.size_half_pt != default.size_half_pt {
            run.size_half_pt
        } else {
            base.size_half_pt
        },
        color: if run.color != default.color {
            run.color.clone()
        } else {
            base.color.clone()
        },
        font: if run.font != default.font {
            run.font.clone()
        } else {
            base.font.clone()
        },
        vert_align: if run.vert_align != default.vert_align {
            run.vert_align.clone()
        } else {
            base.vert_align.clone()
        },
        highlight: if run.highlight != default.highlight {
            run.highlight.clone()
        } else {
            base.highlight.clone()
        },
    }
}

/// 样式显示名 → 标题级次。`heading 1` / `Heading1` / `标题 1` 都算。
fn heading_from_name(name: &str) -> Option<u8> {
    let lower = name.trim().to_ascii_lowercase();
    if !(lower.starts_with("heading") || lower.starts_with("标题")) {
        return None;
    }
    let digits: String = lower.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.len() != 1 {
        return None;
    }
    digits
        .parse::<u8>()
        .ok()
        .filter(|n| (1..=6).contains(n))
}

fn heading_from_style(style_id: &str) -> Option<u8> {
    let id = style_id.trim().to_ascii_lowercase();
    if id.is_empty() {
        return None;
    }
    let digits: String = id.chars().filter(|c| c.is_ascii_digit()).collect();
    if id.starts_with("heading") || id.starts_with("标题") || id.starts_with('t') && digits.len() == 1 {
        if let Ok(n) = digits.parse::<u8>() {
            if (1..=6).contains(&n) {
                return Some(n);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stylesheet() -> StyleSheet {
        let mut sheet = StyleSheet::default();
        sheet.def_run = RunPatch {
            size: Some(21),
            font: Some(String::from("宋体")),
            ..RunPatch::default()
        };
        sheet.styles.insert(
            String::from("Heading1"),
            StyleDef {
                based_on: None,
                para: ParaPatch {
                    align: Some(String::from("center")),
                    space_before: Some(240),
                    heading_level: Some(1),
                    ..ParaPatch::default()
                },
                run: RunPatch {
                    bold: Some(true),
                    size: Some(32),
                    font: Some(String::from("黑体")),
                    ..RunPatch::default()
                },
            },
        );
        sheet.styles.insert(
            String::from("QuoteBody"),
            StyleDef {
                based_on: Some(String::from("Heading1")),
                para: ParaPatch {
                    align: Some(String::from("left")),
                    ..ParaPatch::default()
                },
                run: RunPatch::default(),
            },
        );
        sheet
    }

    #[test]
    fn style_inheritance_walks_based_on_chain() {
        let sheet = stylesheet();
        let resolved = sheet.resolve_style("QuoteBody");
        assert_eq!(resolved.align.as_deref(), Some("left"));
        assert_eq!(resolved.space_before, Some(240));
        assert_eq!(resolved.heading_level, Some(1));
        assert_eq!(resolved.run.bold, Some(true));
        assert_eq!(resolved.run.size, Some(32));
        assert_eq!(resolved.run.font.as_deref(), Some("黑体"));
    }

    #[test]
    fn style_unknown_id_falls_back_to_doc_defaults() {
        let sheet = stylesheet();
        let resolved = sheet.resolve_style("NotThere");
        assert_eq!(resolved.align, None);
        assert_eq!(resolved.run.size, Some(21));
        assert_eq!(resolved.run.font.as_deref(), Some("宋体"));
    }

    #[test]
    fn heading_level_read_from_style_id() {
        assert_eq!(heading_from_style("Heading2"), Some(2));
        assert_eq!(heading_from_style("heading6"), Some(6));
        assert_eq!(heading_from_style("Normal"), None);
        assert_eq!(heading_from_style(""), None);
    }

    #[test]
    fn run_merge_prefers_explicit_run_props() {
        let base = RunFmt {
            bold: true,
            size_half_pt: 32,
            font: String::from("黑体"),
            ..RunFmt::default()
        };
        let run = RunFmt {
            size_half_pt: 24,
            ..RunFmt::default()
        };
        let merged = merge_run(&base, &run);
        assert!(merged.bold);
        assert_eq!(merged.size_half_pt, 24);
        assert_eq!(merged.font, "黑体");
    }

    fn zipped(document: &str, styles: &str) -> Vec<u8> {
        use std::io::{Cursor, Write};

        use zip::write::SimpleFileOptions;
        use zip::{CompressionMethod, ZipWriter};

        let mut cursor = Cursor::new(Vec::new());
        {
            let mut zw = ZipWriter::new(&mut cursor);
            let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
            zw.start_file("word/document.xml", opts).unwrap();
            zw.write_all(document.as_bytes()).unwrap();
            zw.start_file("word/styles.xml", opts).unwrap();
            zw.write_all(styles.as_bytes()).unwrap();
            zw.finish().unwrap();
        }
        cursor.into_inner()
    }

    const RELS_XML: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId5" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="media/image1.png"/>
</Relationships>"#;

    fn zipped_with_media(document: &str, styles: &str) -> Vec<u8> {
        use std::io::{Cursor, Write};

        use zip::write::SimpleFileOptions;
        use zip::{CompressionMethod, ZipWriter};

        let mut cursor = Cursor::new(Vec::new());
        {
            let mut zw = ZipWriter::new(&mut cursor);
            let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
            zw.start_file("word/document.xml", opts).unwrap();
            zw.write_all(document.as_bytes()).unwrap();
            zw.start_file("word/styles.xml", opts).unwrap();
            zw.write_all(styles.as_bytes()).unwrap();
            zw.start_file("word/_rels/document.xml.rels", opts).unwrap();
            zw.write_all(RELS_XML.as_bytes()).unwrap();
            zw.start_file("word/media/image1.png", opts).unwrap();
            zw.write_all(b"\x89PNG fake").unwrap();
            zw.finish().unwrap();
        }
        cursor.into_inner()
    }

    #[test]
    fn drawing_run_becomes_image_ref_with_emu_size() {
        let document = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
            xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing"
            xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
            xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <w:body>
    <w:p><w:r><w:t>图前</w:t></w:r></w:p>
    <w:p><w:r><w:drawing>
      <wp:inline><wp:extent cx="952500" cy="476250"/>
        <a:graphic><a:graphicData><a:blip r:embed="rId5"/></a:graphicData></a:graphic>
      </wp:inline>
    </w:drawing></w:r></w:p>
    <w:p><w:r><w:t>图后</w:t></w:r></w:p>
  </w:body>
</w:document>"#;

        let doc = parse_rendered_doc(&zipped_with_media(document, "")).expect("parse");
        assert_eq!(doc.blocks.len(), 3, "图片段落不应被吞掉");
        let p = doc.blocks[1].para.as_ref().expect("图片段落");
        let img = p.runs[0].image.as_ref().expect("应解析出图片");
        assert_eq!(img.path, "word/media/image1.png");
        assert_eq!((img.width_px, img.height_px), (100, 50), "EMU 换算成 px");
        assert_eq!(p.runs[0].text, "");
        let after = doc.blocks[2].para.as_ref().expect("图后段落");
        assert_eq!(after.runs[0].text, "图后");
    }

    #[test]
    fn drawing_without_rels_is_dropped_silently() {
        let document = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
            xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing"
            xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
            xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <w:body>
    <w:p><w:r><w:drawing><wp:inline><a:blip r:embed="rIdX"/></wp:inline></w:drawing></w:r></w:p>
    <w:p><w:r><w:t>后文</w:t></w:r></w:p>
  </w:body>
</w:document>"#;

        let doc = parse_rendered_doc(&zipped(document, "")).expect("parse");
        assert!(
            doc.blocks.iter().all(|b| b
                .para
                .as_ref()
                .map(|p| p.runs.iter().all(|r| r.image.is_none()))
                .unwrap_or(true)),
            "关系表缺失时不应产出图片 run"
        );
        assert_eq!(doc.blocks.last().unwrap().para.as_ref().unwrap().runs[0].text, "后文");
    }

    const STYLES_XML: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:docDefaults>
    <w:rPrDefault><w:rPr><w:rFonts w:ascii="Times New Roman" w:eastAsia="宋体"/><w:sz w:val="24"/></w:rPr></w:rPrDefault>
  </w:docDefaults>
  <w:style w:type="paragraph" w:styleId="Normal"><w:pPr><w:spacing w:after="120"/></w:pPr></w:style>
  <w:style w:type="paragraph" w:styleId="Heading1">
    <w:basedOn w:val="Normal"/>
    <w:pPr><w:outlineLvl w:val="0"/><w:spacing w:before="240" w:after="120"/><w:jc w:val="center"/></w:pPr>
    <w:rPr><w:b/><w:sz w:val="32"/></w:rPr>
  </w:style>
</w:styles>"#;

    #[test]
    fn parses_styles_xml_into_heading_level_and_run_format() {
        let document = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p><w:pPr><w:pStyle w:val="Heading1"/></w:pPr><w:r><w:t>第一章 总体方案</w:t></w:r></w:p>
    <w:p><w:pPr><w:jc w:val="both"/><w:ind w:firstLine="480"/></w:pPr>
      <w:r><w:rPr><w:b/></w:rPr><w:t>加粗导语</w:t></w:r>
      <w:r><w:t>正文</w:t></w:r>
    </w:p>
  </w:body>
</w:document>"#;

        let doc = parse_rendered_doc(&zipped(document, STYLES_XML)).expect("parse");
        assert_eq!(doc.blocks.len(), 2);

        let head = &doc.blocks[0];
        let hp = head.para.as_ref().expect("heading para");
        assert_eq!(hp.fmt.style_id, "Heading1", "pStyle 应解析进 styleId");
        assert_eq!(head.heading_level, 1, "outlineLvl/Heading1 应识别为一级标题");
        assert_eq!(hp.fmt.align, "center", "样式里的 jc 应继承");
        assert_eq!(hp.fmt.space_before_twips, 240);
        assert_eq!(hp.fmt.space_after_twips, 120, "Normal 的段后应沿 basedOn 继承");
        assert!(hp.runs[0].fmt.bold, "样式 rPr 的 bold 应生效");
        assert_eq!(hp.runs[0].fmt.size_half_pt, 32, "样式 rPr 的字号应生效");
        assert_eq!(hp.runs[0].fmt.font, "宋体", "docDefaults 字体应生效");

        let body = &doc.blocks[1];
        let bp = body.para.as_ref().expect("body para");
        assert_eq!(bp.fmt.align, "justify", "w:jc=both 归一化为 justify");
        assert_eq!(bp.fmt.indent_first_line_twips, 480);
        assert_eq!(body.heading_level, 0);
        assert!(bp.runs[0].fmt.bold, "run 直接格式的 bold");
        assert!(!bp.runs[1].fmt.bold, "同段第二个 run 不应被带 bold");
    }

    #[test]
    fn heading_level_from_style_name_when_style_id_is_numeric() {
        // WPS 常见：styleId="1"，级次只写在 <w:name w:val="heading 1"/>
        let styles = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:style w:type="paragraph" w:styleId="1"><w:name w:val="heading 1"/></w:style>
  <w:style w:type="paragraph" w:styleId="2"><w:name w:val="heading 2"/></w:style>
  <w:style w:type="paragraph" w:styleId="a"><w:name w:val="Normal"/></w:style>
</w:styles>"#;
        let document = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p><w:pPr><w:pStyle w:val="1"/></w:pPr><w:r><w:t>第一章</w:t></w:r></w:p>
    <w:p><w:pPr><w:pStyle w:val="2"/></w:pPr><w:r><w:t>第一节</w:t></w:r></w:p>
    <w:p><w:pPr><w:pStyle w:val="a"/></w:pPr><w:r><w:t>正文</w:t></w:r></w:p>
  </w:body>
</w:document>"#;

        let doc = parse_rendered_doc(&zipped(document, styles)).expect("parse");
        assert_eq!(doc.blocks[0].heading_level, 1);
        assert_eq!(doc.blocks[1].heading_level, 2);
        assert_eq!(doc.blocks[2].heading_level, 0, "Normal 不应被判成标题");
    }

    #[test]
    fn table_alignment_and_grid_span_defaults() {
        let document = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:tbl>
      <w:tblPr><w:jc w:val="center"/><w:tblW w:w="5000" w:type="dxa"/></w:tblPr>
      <w:tblGrid><w:gridCol w:w="2000"/><w:gridCol w:w="3000"/></w:tblGrid>
      <w:tr>
        <w:tc><w:tcPr><w:gridSpan w:val="2"/></w:tcPr><w:p><w:r><w:t>合并</w:t></w:r></w:p></w:tc>
      </w:tr>
      <w:tr>
        <w:tc><w:p><w:r><w:t>甲</w:t></w:r></w:p></w:tc>
        <w:tc><w:p><w:r><w:t>乙</w:t></w:r></w:p></w:tc>
      </w:tr>
    </w:tbl>
  </w:body>
</w:document>"#;

        let doc = parse_rendered_doc(&zipped(document, "")).expect("parse");
        let t = doc
            .blocks
            .iter()
            .find_map(|b| b.table.as_ref())
            .expect("表格块");
        assert_eq!(t.align, "center", "tblPr/jc 是表格级对齐");
        assert_eq!(t.grid, vec![2000, 3000]);
        assert_eq!(t.rows[0].cells[0].grid_span, 2);
        assert_eq!(t.rows[1].cells[0].grid_span, 1, "未声明时默认 1");
    }

    #[test]
    fn run_level_page_break_becomes_break_block() {
        let document = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p><w:r><w:t>甲段</w:t></w:r></w:p>
    <w:p><w:r><w:br w:type="page"/></w:r></w:p>
    <w:p><w:r><w:t>乙段</w:t></w:r></w:p>
  </w:body>
</w:document>"#;

        let doc = parse_rendered_doc(&zipped(document, "")).expect("parse");
        let kinds: Vec<&str> = doc.blocks.iter().map(|b| b.kind.as_str()).collect();
        assert_eq!(kinds, vec!["para", "page_break", "para"], "分页符应产出独立块");
        assert_eq!(doc.page_break_count, 1);
        assert_eq!(doc.blocks[1].ord, 1, "分页块沿用其所在段落序");
    }
}
