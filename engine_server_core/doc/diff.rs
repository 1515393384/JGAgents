use std::collections::{HashMap, HashSet};

use crate::doc::model::{
    DiffBlock, DiffKind, DiffSpan, DocDiffReport, DocNodeRecord, NodeKind, SpanOp, TableRowDiff,
};

#[derive(Clone, Debug)]
struct MatchPair {
    old_idx: usize,
    new_idx: usize,
    kind: DiffKind,
}

pub fn diff_text_spans(before: &str, after: &str) -> (Vec<DiffSpan>, usize, usize) {
    if before == after {
        if before.is_empty() {
            return (Vec::new(), 0, 0);
        }
        return (
            vec![DiffSpan {
                op: SpanOp::Equal,
                text: before.to_string(),
            }],
            0,
            0,
        );
    }

    let a = tokenize_diff_units(before);
    let b = tokenize_diff_units(after);

    if a.len() * b.len() > 200_000 {
        let deleted = before.chars().count();
        let added = after.chars().count();
        let mut spans = Vec::new();
        if !before.is_empty() {
            spans.push(DiffSpan {
                op: SpanOp::Delete,
                text: before.to_string(),
            });
        }
        if !after.is_empty() {
            spans.push(DiffSpan {
                op: SpanOp::Insert,
                text: after.to_string(),
            });
        }
        return (spans, added, deleted);
    }

    let lcs_pairs = lcs_indices(&a, &b);
    let mut raw: Vec<DiffSpan> = Vec::new();
    let mut i = 0usize;
    let mut j = 0usize;

    for (ai, bj) in lcs_pairs {
        if i < ai {
            raw.push(DiffSpan {
                op: SpanOp::Delete,
                text: a[i..ai].concat(),
            });
        }
        if j < bj {
            raw.push(DiffSpan {
                op: SpanOp::Insert,
                text: b[j..bj].concat(),
            });
        }
        raw.push(DiffSpan {
            op: SpanOp::Equal,
            text: a[ai].clone(),
        });
        i = ai + 1;
        j = bj + 1;
    }

    if i < a.len() {
        raw.push(DiffSpan {
            op: SpanOp::Delete,
            text: a[i..].concat(),
        });
    }
    if j < b.len() {
        raw.push(DiffSpan {
            op: SpanOp::Insert,
            text: b[j..].concat(),
        });
    }

    let merged = merge_spans(raw);
    let mut added_chars = 0usize;
    let mut deleted_chars = 0usize;
    for s in &merged {
        match s.op {
            SpanOp::Insert => added_chars += s.text.chars().count(),
            SpanOp::Delete => deleted_chars += s.text.chars().count(),
            SpanOp::Equal => {}
        }
    }
    (merged, added_chars, deleted_chars)
}

pub fn compute_doc_diff(
    doc_id: &str,
    from_rev: i64,
    to_rev: i64,
    old_all: &[DocNodeRecord],
    new_all: &[DocNodeRecord],
) -> DocDiffReport {
    let old_nodes: Vec<&DocNodeRecord> = old_all.iter().filter(|n| !n.is_search_chunk).collect();
    let new_nodes: Vec<&DocNodeRecord> = new_all.iter().filter(|n| !n.is_search_chunk).collect();

    let mut old_matched = vec![false; old_nodes.len()];
    let mut new_matched = vec![false; new_nodes.len()];
    let mut pairs: Vec<MatchPair> = Vec::new();

    // Level 1: Explicit anchor match (w14:paraId / bookmarkStart)
    let mut old_by_explicit: HashMap<&str, usize> = HashMap::new();
    for (idx, n) in old_nodes.iter().enumerate() {
        if let Some(ref exp) = n.explicit_anchor {
            if !exp.is_empty() {
                old_by_explicit.entry(exp.as_str()).or_insert(idx);
            }
        }
    }
    for (j, n) in new_nodes.iter().enumerate() {
        if let Some(ref exp) = n.explicit_anchor {
            if let Some(&i) = old_by_explicit.get(exp.as_str()) {
                if !old_matched[i] {
                    old_matched[i] = true;
                    new_matched[j] = true;
                    let kind = if old_nodes[i].text_hash == n.text_hash {
                        DiffKind::Equal
                    } else {
                        DiffKind::Modified
                    };
                    pairs.push(MatchPair {
                        old_idx: i,
                        new_idx: j,
                        kind,
                    });
                }
            }
        }
    }

    // Level 2: Unique / ordered (heading_path + text_hash) anchor alignment
    let mut old_hash_map: HashMap<(&str, &str), Vec<usize>> = HashMap::new();
    for (i, n) in old_nodes.iter().enumerate() {
        if !old_matched[i] {
            old_hash_map
                .entry((n.heading_path.as_str(), n.text_hash.as_str()))
                .or_default()
                .push(i);
        }
    }
    for (j, n) in new_nodes.iter().enumerate() {
        if new_matched[j] {
            continue;
        }
        if let Some(candidates) = old_hash_map.get(&(n.heading_path.as_str(), n.text_hash.as_str()))
        {
            if let Some(&i) = candidates.iter().find(|&&idx| !old_matched[idx]) {
                old_matched[i] = true;
                new_matched[j] = true;
                let moved = i.abs_diff(j) > 2 && pairs.is_empty();
                pairs.push(MatchPair {
                    old_idx: i,
                    new_idx: j,
                    kind: if moved {
                        DiffKind::Moved
                    } else {
                        DiffKind::Equal
                    },
                });
            }
        }
    }

    // Level 2b: Same text_hash even if heading moved
    let mut old_text_hash_map: HashMap<&str, Vec<usize>> = HashMap::new();
    for (i, n) in old_nodes.iter().enumerate() {
        if !old_matched[i] && !n.text_content.trim().is_empty() {
            old_text_hash_map
                .entry(n.text_hash.as_str())
                .or_default()
                .push(i);
        }
    }
    for (j, n) in new_nodes.iter().enumerate() {
        if new_matched[j] {
            continue;
        }
        if let Some(candidates) = old_text_hash_map.get(n.text_hash.as_str()) {
            if let Some(&i) = candidates.iter().find(|&&idx| !old_matched[idx]) {
                old_matched[i] = true;
                new_matched[j] = true;
                let kind = if old_nodes[i].heading_path != n.heading_path {
                    DiffKind::Moved
                } else {
                    DiffKind::Equal
                };
                pairs.push(MatchPair {
                    old_idx: i,
                    new_idx: j,
                    kind,
                });
            }
        }
    }

    // Level 3: Local fuzzy bigram similarity pairing for modified paragraphs/tables
    let unmatched_old: Vec<usize> = (0..old_nodes.len()).filter(|&i| !old_matched[i]).collect();
    let unmatched_new: Vec<usize> = (0..new_nodes.len()).filter(|&j| !new_matched[j]).collect();

    for &j in &unmatched_new {
        let mut best: Option<(usize, f64)> = None;
        for &i in &unmatched_old {
            if old_matched[i] {
                continue;
            }
            if !compatible_kinds(old_nodes[i].node_kind, new_nodes[j].node_kind) {
                continue;
            }
            let mut score = dice_similarity(&old_nodes[i].text_content, &new_nodes[j].text_content);
            if old_nodes[i].heading_path == new_nodes[j].heading_path {
                score += 0.12;
            }
            if score >= 0.28 {
                if best.map(|(_, b)| score > b).unwrap_or(true) {
                    best = Some((i, score));
                }
            }
        }
        if let Some((i, _)) = best {
            old_matched[i] = true;
            new_matched[j] = true;
            pairs.push(MatchPair {
                old_idx: i,
                new_idx: j,
                kind: DiffKind::Modified,
            });
        }
    }

    // Build ordered diff blocks preserving document reading order
    let mut pair_by_new: HashMap<usize, MatchPair> = HashMap::new();
    for p in &pairs {
        pair_by_new.insert(p.new_idx, p.clone());
    }

    let mut blocks: Vec<DiffBlock> = Vec::new();
    let mut emitted_old: HashSet<usize> = HashSet::new();
    let mut next_old_cursor = 0usize;

    let mut total_added = 0usize;
    let mut total_deleted = 0usize;
    let mut changed_blocks = 0usize;
    let mut locked_conflicts: Vec<String> = Vec::new();

    for (j, new_node) in new_nodes.iter().enumerate() {
        while next_old_cursor < old_nodes.len() {
            if old_matched[next_old_cursor] {
                next_old_cursor += 1;
                continue;
            }
            // Emit deleted old nodes whose natural position precedes current new_node
            if let Some(p) = pair_by_new.get(&j) {
                if next_old_cursor < p.old_idx {
                    let old_n = old_nodes[next_old_cursor];
                    if emitted_old.insert(next_old_cursor) {
                        let del_chars = old_n.text_content.chars().count();
                        total_deleted += del_chars;
                        changed_blocks += 1;
                        if old_n.locked {
                            locked_conflicts.push(old_n.anchor_id.clone());
                        }
                        blocks.push(DiffBlock {
                            anchor_id: old_n.anchor_id.clone(),
                            heading_path: old_n.heading_path.clone(),
                            node_kind: old_n.node_kind,
                            diff_kind: DiffKind::Deleted,
                            before_text: old_n.text_content.clone(),
                            after_text: String::new(),
                            spans: vec![DiffSpan {
                                op: SpanOp::Delete,
                                text: old_n.text_content.clone(),
                            }],
                            added_chars: 0,
                            deleted_chars: del_chars,
                            locked: old_n.locked,
                            table_row_diffs: Vec::new(),
                        });
                    }
                    next_old_cursor += 1;
                    continue;
                }
            }
            break;
        }

        if let Some(p) = pair_by_new.get(&j) {
            let old_n = old_nodes[p.old_idx];
            emitted_old.insert(p.old_idx);
            while next_old_cursor <= p.old_idx && next_old_cursor < old_nodes.len() {
                if old_matched[next_old_cursor] || emitted_old.contains(&next_old_cursor) {
                    next_old_cursor += 1;
                } else {
                    break;
                }
            }

            if p.kind == DiffKind::Equal {
                blocks.push(DiffBlock {
                    anchor_id: new_node.anchor_id.clone(),
                    heading_path: new_node.heading_path.clone(),
                    node_kind: new_node.node_kind,
                    diff_kind: DiffKind::Equal,
                    before_text: old_n.text_content.clone(),
                    after_text: new_node.text_content.clone(),
                    spans: vec![DiffSpan {
                        op: SpanOp::Equal,
                        text: new_node.text_content.clone(),
                    }],
                    added_chars: 0,
                    deleted_chars: 0,
                    locked: old_n.locked || new_node.locked,
                    table_row_diffs: Vec::new(),
                });
            } else {
                let (spans, add_c, del_c) =
                    diff_text_spans(&old_n.text_content, &new_node.text_content);
                let row_diffs = if matches!(new_node.node_kind, NodeKind::Table | NodeKind::TableChunk)
                    || matches!(old_n.node_kind, NodeKind::Table | NodeKind::TableChunk)
                {
                    diff_table_rows(old_n, new_node)
                } else {
                    Vec::new()
                };

                total_added += add_c;
                total_deleted += del_c;
                if add_c > 0 || del_c > 0 || p.kind == DiffKind::Moved {
                    changed_blocks += 1;
                }
                let is_locked = old_n.locked || new_node.locked;
                if is_locked && (add_c > 0 || del_c > 0) {
                    locked_conflicts.push(new_node.anchor_id.clone());
                }

                blocks.push(DiffBlock {
                    anchor_id: new_node.anchor_id.clone(),
                    heading_path: new_node.heading_path.clone(),
                    node_kind: new_node.node_kind,
                    diff_kind: p.kind,
                    before_text: old_n.text_content.clone(),
                    after_text: new_node.text_content.clone(),
                    spans,
                    added_chars: add_c,
                    deleted_chars: del_c,
                    locked: is_locked,
                    table_row_diffs: row_diffs,
                });
            }
        } else {
            let add_chars = new_node.text_content.chars().count();
            total_added += add_chars;
            changed_blocks += 1;
            blocks.push(DiffBlock {
                anchor_id: new_node.anchor_id.clone(),
                heading_path: new_node.heading_path.clone(),
                node_kind: new_node.node_kind,
                diff_kind: DiffKind::Inserted,
                before_text: String::new(),
                after_text: new_node.text_content.clone(),
                spans: vec![DiffSpan {
                    op: SpanOp::Insert,
                    text: new_node.text_content.clone(),
                }],
                added_chars: add_chars,
                deleted_chars: 0,
                locked: new_node.locked,
                table_row_diffs: Vec::new(),
            });
        }
    }

    for (i, old_n) in old_nodes.iter().enumerate() {
        if !old_matched[i] && !emitted_old.contains(&i) {
            let del_chars = old_n.text_content.chars().count();
            total_deleted += del_chars;
            changed_blocks += 1;
            if old_n.locked {
                locked_conflicts.push(old_n.anchor_id.clone());
            }
            blocks.push(DiffBlock {
                anchor_id: old_n.anchor_id.clone(),
                heading_path: old_n.heading_path.clone(),
                node_kind: old_n.node_kind,
                diff_kind: DiffKind::Deleted,
                before_text: old_n.text_content.clone(),
                after_text: String::new(),
                spans: vec![DiffSpan {
                    op: SpanOp::Delete,
                    text: old_n.text_content.clone(),
                }],
                added_chars: 0,
                deleted_chars: del_chars,
                locked: old_n.locked,
                table_row_diffs: Vec::new(),
            });
        }
    }

    DocDiffReport {
        doc_id: doc_id.to_string(),
        from_rev,
        to_rev,
        added_chars: total_added,
        deleted_chars: total_deleted,
        changed_blocks,
        locked_conflicts,
        blocks,
    }
}

/// Level 4: Table Row-Key LCS alignment prevents row insertions/deletions from cascading diffs
fn diff_table_rows(old_node: &DocNodeRecord, new_node: &DocNodeRecord) -> Vec<TableRowDiff> {
    let old_rows: Vec<&str> = old_node.text_content.lines().collect();
    let new_rows: Vec<&str> = new_node.text_content.lines().collect();
    let old_keys = if old_node.row_keys.len() == old_rows.len() {
        old_node.row_keys.clone()
    } else {
        old_rows.iter().map(|r| extract_row_key(r)).collect()
    };
    let new_keys = if new_node.row_keys.len() == new_rows.len() {
        new_node.row_keys.clone()
    } else {
        new_rows.iter().map(|r| extract_row_key(r)).collect()
    };

    let lcs = lcs_indices(&old_keys, &new_keys);
    let mut diffs = Vec::new();
    let mut i = 0usize;
    let mut j = 0usize;

    for (oi, nj) in lcs {
        while i < oi {
            diffs.push(TableRowDiff {
                row_key: old_keys[i].clone(),
                op: SpanOp::Delete,
                before_row: Some(old_rows[i].to_string()),
                after_row: None,
            });
            i += 1;
        }
        while j < nj {
            diffs.push(TableRowDiff {
                row_key: new_keys[j].clone(),
                op: SpanOp::Insert,
                before_row: None,
                after_row: Some(new_rows[j].to_string()),
            });
            j += 1;
        }
        if old_rows[oi] != new_rows[nj] {
            diffs.push(TableRowDiff {
                row_key: new_keys[nj].clone(),
                op: SpanOp::Insert,
                before_row: Some(old_rows[oi].to_string()),
                after_row: Some(new_rows[nj].to_string()),
            });
        }
        i = oi + 1;
        j = nj + 1;
    }

    while i < old_rows.len() {
        diffs.push(TableRowDiff {
            row_key: old_keys[i].clone(),
            op: SpanOp::Delete,
            before_row: Some(old_rows[i].to_string()),
            after_row: None,
        });
        i += 1;
    }
    while j < new_rows.len() {
        diffs.push(TableRowDiff {
            row_key: new_keys[j].clone(),
            op: SpanOp::Insert,
            before_row: None,
            after_row: Some(new_rows[j].to_string()),
        });
        j += 1;
    }

    diffs
}

fn extract_row_key(line: &str) -> String {
    line.split('|')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .take(2)
        .collect::<Vec<_>>()
        .join("|")
}

fn compatible_kinds(a: NodeKind, b: NodeKind) -> bool {
    match (a, b) {
        (NodeKind::Table | NodeKind::TableChunk, NodeKind::Table | NodeKind::TableChunk) => true,
        (
            NodeKind::Heading | NodeKind::Paragraph | NodeKind::ListItem,
            NodeKind::Heading | NodeKind::Paragraph | NodeKind::ListItem,
        ) => true,
        _ => false,
    }
}

fn tokenize_diff_units(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut ascii_buf = String::new();
    let mut ws_buf = String::new();

    for ch in text.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' || ch == '.' {
            if !ws_buf.is_empty() {
                tokens.push(std::mem::take(&mut ws_buf));
            }
            ascii_buf.push(ch);
        } else if ch.is_whitespace() {
            if !ascii_buf.is_empty() {
                tokens.push(std::mem::take(&mut ascii_buf));
            }
            ws_buf.push(ch);
        } else {
            if !ascii_buf.is_empty() {
                tokens.push(std::mem::take(&mut ascii_buf));
            }
            if !ws_buf.is_empty() {
                tokens.push(std::mem::take(&mut ws_buf));
            }
            tokens.push(ch.to_string());
        }
    }
    if !ascii_buf.is_empty() {
        tokens.push(ascii_buf);
    }
    if !ws_buf.is_empty() {
        tokens.push(ws_buf);
    }
    tokens
}

fn lcs_indices<T: PartialEq>(a: &[T], b: &[T]) -> Vec<(usize, usize)> {
    let m = a.len();
    let n = b.len();
    if m == 0 || n == 0 {
        return Vec::new();
    }
    let mut dp = vec![vec![0usize; n + 1]; m + 1];
    for i in (0..m).rev() {
        for j in (0..n).rev() {
            dp[i][j] = if a[i] == b[j] {
                dp[i + 1][j + 1] + 1
            } else {
                dp[i + 1][j].max(dp[i][j + 1])
            };
        }
    }

    let mut pairs = Vec::new();
    let mut i = 0usize;
    let mut j = 0usize;
    while i < m && j < n {
        if a[i] == b[j] {
            pairs.push((i, j));
            i += 1;
            j += 1;
        } else if dp[i + 1][j] >= dp[i][j + 1] {
            i += 1;
        } else {
            j += 1;
        }
    }
    pairs
}

fn merge_spans(raw: Vec<DiffSpan>) -> Vec<DiffSpan> {
    let mut merged: Vec<DiffSpan> = Vec::new();
    for s in raw {
        if s.text.is_empty() {
            continue;
        }
        if let Some(last) = merged.last_mut() {
            if last.op == s.op {
                last.text.push_str(&s.text);
                continue;
            }
        }
        merged.push(s);
    }
    merged
}

pub fn dice_similarity(a: &str, b: &str) -> f64 {
    let ca: Vec<char> = a.chars().filter(|c| !c.is_whitespace()).collect();
    let cb: Vec<char> = b.chars().filter(|c| !c.is_whitespace()).collect();
    if ca.is_empty() || cb.is_empty() {
        return 0.0;
    }
    if ca.len() == 1 || cb.len() == 1 {
        return if ca == cb { 1.0 } else { 0.0 };
    }
    let mut bigrams_a: HashMap<(char, char), usize> = HashMap::new();
    for w in ca.windows(2) {
        *bigrams_a.entry((w[0], w[1])).or_insert(0) += 1;
    }
    let mut matches = 0usize;
    let mut total_b = 0usize;
    for w in cb.windows(2) {
        total_b += 1;
        if let Some(cnt) = bigrams_a.get_mut(&(w[0], w[1])) {
            if *cnt > 0 {
                *cnt -= 1;
                matches += 1;
            }
        }
    }
    let total_a = ca.len().saturating_sub(1);
    (2.0 * matches as f64) / ((total_a + total_b) as f64)
}

#[cfg(test)]
mod tests {
    use super::{compute_doc_diff, diff_text_spans};
    use crate::doc::model::{DiffKind, DocNodeRecord, NodeKind, SpanOp};

    fn node(ord: usize, anchor: &str, exp: Option<&str>, text: &str, locked: bool) -> DocNodeRecord {
        DocNodeRecord {
            ord,
            anchor_id: anchor.to_string(),
            parent_anchor_id: None,
            explicit_anchor: exp.map(str::to_string),
            xml_locator: format!("p:{ord}"),
            node_kind: NodeKind::Paragraph,
            heading_path: String::from("第三章 运维服务方案"),
            text_content: text.to_string(),
            text_hash: format!("h_{text}"),
            row_keys: Vec::new(),
            mandatory_tags: Vec::new(),
            locked,
            token_estimate: 10,
            is_search_chunk: false,
        }
    }

    #[test]
    fn hybrid_anchor_diff_tracks_modified_even_when_wps_drops_paraid() {
        let old_nodes = vec![
            node(
                0,
                "a1",
                Some("11AA22BB"),
                "一般故障 30 分钟内响应、2 小时内恢复。",
                true,
            ),
            node(
                1,
                "a2",
                Some("33CC44DD"),
                "连续两个季度无重大故障可下调值守班次为 5x8 小时。",
                false,
            ),
        ];
        // WPS dropped explicit paraId on node 1 and modified text
        let new_nodes = vec![
            node(
                0,
                "b1",
                None,
                "一般故障 30 分钟内响应、2 小时内恢复。",
                true,
            ),
            node(
                1,
                "b2",
                None,
                "连续两个季度无重大故障可下调值守班次为 5x8 小时，需甲方书面确认。",
                false,
            ),
        ];

        let report = compute_doc_diff("doc1", 1, 2, &old_nodes, &new_nodes);
        assert_eq!(report.changed_blocks, 1);
        assert!(report.locked_conflicts.is_empty());
        assert_eq!(report.blocks[0].diff_kind, DiffKind::Equal);
        assert_eq!(report.blocks[1].diff_kind, DiffKind::Modified);
        assert!(report.blocks[1]
            .spans
            .iter()
            .any(|s| s.op == SpanOp::Insert && s.text.contains("需甲方书面确认")));
    }

    #[test]
    fn table_row_key_lcs_isolates_inserted_row() {
        let mut old_t = node(0, "t1", None, "项目 | 指标\n响应时限 | 15分钟\n备件库 | 1:1", false);
        old_t.node_kind = NodeKind::Table;
        old_t.row_keys = vec![
            "项目|指标".into(),
            "响应时限|15分钟".into(),
            "备件库|1:1".into(),
        ];

        let mut new_t = node(
            0,
            "t1_new",
            None,
            "项目 | 指标\n驻场人数 | 4人\n响应时限 | 15分钟\n备件库 | 1:1",
            false,
        );
        new_t.node_kind = NodeKind::Table;
        new_t.row_keys = vec![
            "项目|指标".into(),
            "驻场人数|4人".into(),
            "响应时限|15分钟".into(),
            "备件库|1:1".into(),
        ];

        let report = compute_doc_diff("doc2", 1, 2, &[old_t], &[new_t]);
        assert_eq!(report.blocks.len(), 1);
        assert_eq!(report.blocks[0].table_row_diffs.len(), 1);
        assert_eq!(report.blocks[0].table_row_diffs[0].op, SpanOp::Insert);
        assert!(report.blocks[0].table_row_diffs[0]
            .row_key
            .contains("驻场人数"));
    }

    #[test]
    fn token_diff_handles_cjk_and_numbers_cleanly() {
        let (spans, add, del) = diff_text_spans("运维响应 4 个工作小时", "运维响应 4 小时");
        assert_eq!(add, 0);
        assert_eq!(del, 3);
        assert!(spans.iter().any(|s| s.op == SpanOp::Delete && s.text == "个工作"));
    }
}
