use crate::types::StructLayout;
use serde::Serialize;
use std::collections::{BTreeMap, HashMap};

/// Penalty for matching structs with mismatched source locations.
/// Large enough to dominate all other scoring factors, preventing cross-location matching.
const LOCATION_MISMATCH_PENALTY: i64 = i64::MIN / 4;

#[derive(Debug, Clone, Serialize)]
pub struct DiffResult {
    pub added: Vec<StructSummary>,
    pub removed: Vec<StructSummary>,
    pub changed: Vec<StructChange>,
    pub unchanged_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct StructSummary {
    pub name: String,
    pub size: u64,
    pub padding_bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct StructChange {
    pub name: String,
    pub old_size: u64,
    pub new_size: u64,
    pub size_delta: i64,
    pub old_padding: u64,
    pub new_padding: u64,
    pub padding_delta: i64,
    pub member_changes: Vec<MemberChange>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MemberChange {
    pub kind: MemberChangeKind,
    pub name: String,
    pub details: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub enum MemberChangeKind {
    Added,
    Removed,
    OffsetChanged,
    SizeChanged,
    TypeChanged,
}

impl DiffResult {
    pub fn has_changes(&self) -> bool {
        !self.added.is_empty() || !self.removed.is_empty() || !self.changed.is_empty()
    }

    pub fn has_regressions(&self) -> bool {
        self.changed.iter().any(|c| c.size_delta > 0 || c.padding_delta > 0)
    }
}

pub fn diff_layouts(old: &[StructLayout], new: &[StructLayout]) -> DiffResult {
    let mut added = Vec::new();
    let mut removed = Vec::new();
    let mut changed = Vec::new();
    let mut unchanged_count = 0;

    // Group by display name; allow duplicates and match them deterministically.
    // IMPORTANT: Use BTreeMap (not HashMap) for deterministic iteration order.
    // This ensures stable JSON output across runs. See AUDIT_FINDINGS.md Finding #1.
    let mut old_by_name: BTreeMap<String, Vec<&StructLayout>> = BTreeMap::new();
    let mut new_by_name: BTreeMap<String, Vec<&StructLayout>> = BTreeMap::new();

    for s in old {
        old_by_name.entry(s.name.clone()).or_default().push(s);
    }
    for s in new {
        new_by_name.entry(s.name.clone()).or_default().push(s);
    }

    // Iterate names in a deterministic order.
    let mut all_names: BTreeMap<String, ()> = BTreeMap::new();
    for name in old_by_name.keys() {
        all_names.insert(name.clone(), ());
    }
    for name in new_by_name.keys() {
        all_names.insert(name.clone(), ());
    }

    for (name, _) in all_names {
        let old_group = old_by_name.get(&name).map(Vec::as_slice).unwrap_or(&[]);
        let new_group = new_by_name.get(&name).map(Vec::as_slice).unwrap_or(&[]);

        if old_group.is_empty() {
            for s in new_group {
                added.push(StructSummary {
                    name: name.clone(),
                    size: s.size,
                    padding_bytes: s.metrics.padding_bytes,
                });
            }
            continue;
        }

        if new_group.is_empty() {
            for s in old_group {
                removed.push(StructSummary {
                    name: name.clone(),
                    size: s.size,
                    padding_bytes: s.metrics.padding_bytes,
                });
            }
            continue;
        }

        let (pairs, old_unmatched, new_unmatched) = match_structs(old_group, new_group);

        for (old_s, new_s) in pairs {
            if let Some(change) = diff_struct(old_s, new_s) {
                changed.push(change);
            } else {
                unchanged_count += 1;
            }
        }

        for s in old_unmatched {
            removed.push(StructSummary {
                name: name.clone(),
                size: s.size,
                padding_bytes: s.metrics.padding_bytes,
            });
        }

        for s in new_unmatched {
            added.push(StructSummary {
                name: name.clone(),
                size: s.size,
                padding_bytes: s.metrics.padding_bytes,
            });
        }
    }

    added.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.size.cmp(&b.size)));
    removed.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.size.cmp(&b.size)));
    changed.sort_by(|a, b| {
        a.name
            .cmp(&b.name)
            .then_with(|| a.old_size.cmp(&b.old_size))
            .then_with(|| a.new_size.cmp(&b.new_size))
    });

    DiffResult { added, removed, changed, unchanged_count }
}

fn location_key(s: &StructLayout) -> Option<(&str, u64)> {
    s.source_location.as_ref().map(|loc| (loc.file.as_str(), loc.line))
}

fn member_similarity_score(old: &StructLayout, new: &StructLayout) -> i64 {
    // If both have source locations and they disagree, heavily penalize to avoid cross-matching.
    if let (Some(ol), Some(nl)) = (location_key(old), location_key(new)) {
        if ol != nl {
            return LOCATION_MISMATCH_PENALTY;
        }
    }

    let mut score: i64 = 0;

    // Prefer similar overall sizes/padding.
    // Use abs_diff() to avoid overflow when converting large u64 to i64.
    let size_delta = old.size.abs_diff(new.size).min(i64::MAX as u64) as i64;
    score = score.saturating_sub(size_delta);

    let pad_delta = old.metrics.padding_bytes.abs_diff(new.metrics.padding_bytes).min(i64::MAX as u64) as i64;
    score = score.saturating_sub(pad_delta / 2);

    // Member-name overlap drives matching for same-name duplicates.
    let old_members: HashMap<&str, _> = old.members.iter().map(|m| (m.name.as_str(), m)).collect();
    let new_members: HashMap<&str, _> = new.members.iter().map(|m| (m.name.as_str(), m)).collect();

    let mut intersection: i64 = 0;
    for (name, om) in &old_members {
        if let Some(nm) = new_members.get(name) {
            intersection += 1;
            if om.type_name == nm.type_name {
                score += 5;
            }
            if om.size == nm.size {
                score += 2;
            }
            if om.offset == nm.offset {
                score += 1;
            }
        }
    }

    score += intersection * 10;

    // Light preference for similar member count.
    let count_delta = (old.members.len() as i64 - new.members.len() as i64).unsigned_abs() as i64;
    score -= count_delta;

    score
}

fn match_structs<'a>(
    old_group: &[&'a StructLayout],
    new_group: &[&'a StructLayout],
) -> (Vec<(&'a StructLayout, &'a StructLayout)>, Vec<&'a StructLayout>, Vec<&'a StructLayout>) {
    let mut pairs: Vec<(&StructLayout, &StructLayout)> = Vec::new();
    let mut old_used = vec![false; old_group.len()];
    let mut new_used = vec![false; new_group.len()];

    // 1) Exact match by (file,line) when available.
    let mut new_by_loc: BTreeMap<(&str, u64), Vec<usize>> = BTreeMap::new();
    for (j, s) in new_group.iter().enumerate() {
        if let Some(loc) = location_key(s) {
            new_by_loc.entry(loc).or_default().push(j);
        }
    }

    for (i, s) in old_group.iter().enumerate() {
        let Some(loc) = location_key(s) else { continue };
        let Some(candidates) = new_by_loc.get(&loc) else { continue };

        if let Some(&j) = candidates.iter().find(|&&j| !new_used[j]) {
            old_used[i] = true;
            new_used[j] = true;
            pairs.push((*s, new_group[j]));
        }
    }

    // 2) Deterministic greedy matching by similarity score.
    // First check if exactly one unpaired remains on each side - if so, pair them directly
    // to preserve baseline semantics for non-duplicate names (avoids O(n*m) similarity calc).
    let remaining_old: Vec<usize> =
        old_used.iter().enumerate().filter_map(|(i, used)| (!*used).then_some(i)).collect();
    let remaining_new: Vec<usize> =
        new_used.iter().enumerate().filter_map(|(j, used)| (!*used).then_some(j)).collect();

    if remaining_old.len() == 1 && remaining_new.len() == 1 {
        let i = remaining_old[0];
        let j = remaining_new[0];
        old_used[i] = true;
        new_used[j] = true;
        pairs.push((old_group[i], new_group[j]));
    } else if !remaining_old.is_empty() && !remaining_new.is_empty() {
        // Build similarity scores only when needed.
        let mut scored: Vec<(i64, usize, usize)> = Vec::new();
        for &i in &remaining_old {
            for &j in &remaining_new {
                let score = member_similarity_score(old_group[i], new_group[j]);
                scored.push((score, i, j));
            }
        }

        scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)).then_with(|| a.2.cmp(&b.2)));

        // Require a positive score to avoid pairing completely unrelated duplicates.
        for (score, i, j) in scored {
            if score <= 0 {
                break;
            }
            if old_used[i] || new_used[j] {
                continue;
            }
            old_used[i] = true;
            new_used[j] = true;
            pairs.push((old_group[i], new_group[j]));
        }
    }

    // Deterministic ordering of produced pairs.
    pairs.sort_by(|(a_old, a_new), (b_old, b_new)| {
        location_key(a_old)
            .cmp(&location_key(b_old))
            .then_with(|| a_old.size.cmp(&b_old.size))
            .then_with(|| a_new.size.cmp(&b_new.size))
    });

    let old_unmatched: Vec<&StructLayout> =
        old_group.iter().enumerate().filter_map(|(i, s)| (!old_used[i]).then_some(*s)).collect();
    let new_unmatched: Vec<&StructLayout> =
        new_group.iter().enumerate().filter_map(|(j, s)| (!new_used[j]).then_some(*s)).collect();

    (pairs, old_unmatched, new_unmatched)
}

fn kind_rank(kind: &MemberChangeKind) -> u8 {
    match kind {
        MemberChangeKind::Removed => 0,
        MemberChangeKind::Added => 1,
        MemberChangeKind::TypeChanged => 2,
        MemberChangeKind::SizeChanged => 3,
        MemberChangeKind::OffsetChanged => 4,
    }
}

fn diff_struct(old: &StructLayout, new: &StructLayout) -> Option<StructChange> {
    // Use saturating signed subtraction to handle large u64 values safely.
    let size_delta = (new.size as i128 - old.size as i128).clamp(i64::MIN as i128, i64::MAX as i128) as i64;
    let padding_delta = (new.metrics.padding_bytes as i128 - old.metrics.padding_bytes as i128)
        .clamp(i64::MIN as i128, i64::MAX as i128) as i64;

    let mut member_changes = Vec::new();

    let old_members: HashMap<&str, _> = old.members.iter().map(|m| (m.name.as_str(), m)).collect();
    let new_members: HashMap<&str, _> = new.members.iter().map(|m| (m.name.as_str(), m)).collect();

    for (name, old_member) in &old_members {
        if !new_members.contains_key(name) {
            member_changes.push(MemberChange {
                kind: MemberChangeKind::Removed,
                name: name.to_string(),
                details: format!("offset {:?}, size {:?}", old_member.offset, old_member.size),
            });
        }
    }

    for (name, new_member) in &new_members {
        match old_members.get(name) {
            None => {
                member_changes.push(MemberChange {
                    kind: MemberChangeKind::Added,
                    name: name.to_string(),
                    details: format!("offset {:?}, size {:?}", new_member.offset, new_member.size),
                });
            }
            Some(old_member) => {
                if old_member.offset != new_member.offset {
                    member_changes.push(MemberChange {
                        kind: MemberChangeKind::OffsetChanged,
                        name: name.to_string(),
                        details: format!("{:?} -> {:?}", old_member.offset, new_member.offset),
                    });
                }
                if old_member.size != new_member.size {
                    member_changes.push(MemberChange {
                        kind: MemberChangeKind::SizeChanged,
                        name: name.to_string(),
                        details: format!("{:?} -> {:?}", old_member.size, new_member.size),
                    });
                }
                if old_member.type_name != new_member.type_name {
                    member_changes.push(MemberChange {
                        kind: MemberChangeKind::TypeChanged,
                        name: name.to_string(),
                        details: format!("{} -> {}", old_member.type_name, new_member.type_name),
                    });
                }
            }
        }
    }

    member_changes.sort_by(|a, b| {
        kind_rank(&a.kind)
            .cmp(&kind_rank(&b.kind))
            .then_with(|| a.name.cmp(&b.name))
            .then_with(|| a.details.cmp(&b.details))
    });

    if size_delta == 0 && padding_delta == 0 && member_changes.is_empty() {
        return None;
    }

    Some(StructChange {
        name: old.name.clone(),
        old_size: old.size,
        new_size: new.size,
        size_delta,
        old_padding: old.metrics.padding_bytes,
        new_padding: new.metrics.padding_bytes,
        padding_delta,
        member_changes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{LayoutMetrics, MemberLayout, StructLayout};

    fn layout(
        name: &str,
        size: u64,
        padding_bytes: u64,
        members: Vec<MemberLayout>,
    ) -> StructLayout {
        let mut s = StructLayout::new(name.to_string(), size, Some(8));
        s.members = members;
        s.metrics = LayoutMetrics { padding_bytes, total_size: size, ..LayoutMetrics::default() };
        s
    }

    #[test]
    fn member_changes_are_sorted_deterministically() {
        let mut old = layout(
            "X",
            16,
            0,
            vec![
                MemberLayout::new("a".to_string(), "u32".to_string(), Some(0), Some(4)),
                MemberLayout::new("b".to_string(), "u32".to_string(), Some(4), Some(4)),
            ],
        );
        let mut new = layout(
            "X",
            20,
            4,
            vec![
                MemberLayout::new("a".to_string(), "u64".to_string(), Some(0), Some(8)), // type/size change
                MemberLayout::new("c".to_string(), "u32".to_string(), Some(8), Some(4)), // added
            ],
        );

        // Ensure metrics exist to make diff meaningful.
        old.metrics.padding_bytes = 0;
        new.metrics.padding_bytes = 4;

        let diff = diff_layouts(&[old], &[new]);
        assert_eq!(diff.changed.len(), 1);
        let changes = &diff.changed[0].member_changes;

        // Expected ordering: Removed, Added, TypeChanged, SizeChanged, OffsetChanged (then name)
        let kinds: Vec<_> = changes.iter().map(|c| &c.kind).collect();
        assert!(kinds.windows(2).all(|w| kind_rank(w[0]) <= kind_rank(w[1])));
    }
}
