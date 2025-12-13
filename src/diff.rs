use crate::types::StructLayout;
use serde::Serialize;
use std::collections::HashMap;

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
    let old_map: HashMap<&str, &StructLayout> = old.iter().map(|s| (s.name.as_str(), s)).collect();
    let new_map: HashMap<&str, &StructLayout> = new.iter().map(|s| (s.name.as_str(), s)).collect();

    let mut added = Vec::new();
    let mut removed = Vec::new();
    let mut changed = Vec::new();
    let mut unchanged_count = 0;

    for (name, old_struct) in &old_map {
        if !new_map.contains_key(name) {
            removed.push(StructSummary {
                name: name.to_string(),
                size: old_struct.size,
                padding_bytes: old_struct.metrics.padding_bytes,
            });
        }
    }

    for (name, new_struct) in &new_map {
        match old_map.get(name) {
            None => {
                added.push(StructSummary {
                    name: name.to_string(),
                    size: new_struct.size,
                    padding_bytes: new_struct.metrics.padding_bytes,
                });
            }
            Some(old_struct) => {
                if let Some(change) = diff_struct(old_struct, new_struct) {
                    changed.push(change);
                } else {
                    unchanged_count += 1;
                }
            }
        }
    }

    added.sort_by(|a, b| a.name.cmp(&b.name));
    removed.sort_by(|a, b| a.name.cmp(&b.name));
    changed.sort_by(|a, b| a.name.cmp(&b.name));

    DiffResult { added, removed, changed, unchanged_count }
}

fn diff_struct(old: &StructLayout, new: &StructLayout) -> Option<StructChange> {
    let size_delta = i64::try_from(new.size)
        .unwrap_or(i64::MAX)
        .saturating_sub(i64::try_from(old.size).unwrap_or(i64::MAX));
    let padding_delta = i64::try_from(new.metrics.padding_bytes)
        .unwrap_or(i64::MAX)
        .saturating_sub(i64::try_from(old.metrics.padding_bytes).unwrap_or(i64::MAX));

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
