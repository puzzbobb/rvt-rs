//! RE-174 (probe): what the symmetric `(N,N,N)` tuples of `Global/ElemTable`
//! mark (#85).
//!
//! Each `ElemTable` record holds a triple of `u32`s between its ids and its
//! owner: bytes `8..20` on both record sizes. Most are symmetric, `(N,N,N)`.
//! 0.2.0 read Level `(8,8,8)`, Material `(20,20,20)` and Space `(25,25,25)`
//! as class markers. The schema names the three words: they are
//! `ElemRec.m_history`'s `m_creationDate`, `m_lastModificationDate` and
//! `m_lastUserModificationDate`, save-episode numbers that
//! `native_index::parse` already reads. `(N,N,N)` is an element created in
//! episode `N` and not modified since.
//!
//! The probe tests the class-marker reading without that parse. The oracle
//! is the element's own native record: its body names its class
//! (`native_document`, Revit 2023 to 2027). It prints:
//!
//! - per file, how many records carry the exact words
//!   `native_index` calls the history (`triple_is_history`);
//! - per symmetric tuple of at least [`MIN_RECORDS`] records, its classes and
//!   the purity of the commonest;
//! - per class of at least [`MIN_RECORDS`] records, how many distinct tuples
//!   it has and how much of it the commonest tuple holds.
//!
//! A class marker would give both purity and concentration near 1.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re174_symmetric_tuples -- MODEL.rvt ...

use rvt::{RevitFile, elem_table, native_document};
use std::collections::BTreeMap;

/// Smallest tuple or class that is printed.
const MIN_RECORDS: usize = 20;

fn word(raw: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(raw.get(at..at + 4)?.try_into().ok()?))
}

fn probe(path: &str) -> anyhow::Result<()> {
    let file = std::path::Path::new(path)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let mut rf = RevitFile::open(path)?;
    let revit = rf.basic_file_info()?.version;
    let records = elem_table::parse_records(&mut rf)?;
    // Class and history of every native record, by ElementId.
    let mut native: BTreeMap<u64, (String, [u32; 3])> = BTreeMap::new();
    let options = native_document::Options::default();
    let native_error = native_document::extract_graphs(&mut rf, &options, |record| {
        let identity = &record.identity;
        let history = [
            identity.creation_episode,
            identity.stored_revision,
            identity.other_revision,
        ];
        if let Some(class) = record.class_name {
            native.insert(identity.element_id, (class, history));
        }
        Ok(())
    })
    .err()
    .map(|e| e.to_string());
    let mut tuples: BTreeMap<[u32; 3], BTreeMap<String, usize>> = BTreeMap::new();
    let mut classes: BTreeMap<String, BTreeMap<[u32; 3], usize>> = BTreeMap::new();
    let (mut joined, mut is_history) = (0usize, 0usize);
    for record in &records {
        let (Some(a), Some(b), Some(c)) = (
            word(&record.raw, 8),
            word(&record.raw, 12),
            word(&record.raw, 16),
        ) else {
            continue;
        };
        let tuple = [a, b, c];
        let id = u64::from(if record.id_secondary != record.id_primary {
            record.id_secondary
        } else {
            record.id_primary
        });
        let class = match native.get(&id) {
            Some((class, history)) => {
                joined += 1;
                is_history += usize::from(*history == tuple);
                class.clone()
            }
            None => "<no native record>".into(),
        };
        *tuples
            .entry(tuple)
            .or_default()
            .entry(class.clone())
            .or_default() += 1;
        *classes.entry(class).or_default().entry(tuple).or_default() += 1;
    }
    println!(
        "{{\"file\":{file:?},\"revit\":{revit},\"records\":{},\"native_records\":{},\"native_error\":{},\"triple_is_history\":{is_history},\"of\":{joined},\"distinct_tuples\":{},\"symmetric_tuples\":{}}}",
        records.len(),
        native.len(),
        native_error
            .as_ref()
            .map_or("null".to_string(), |e| format!("{e:?}")),
        tuples.len(),
        tuples
            .keys()
            .filter(|t| t[0] == t[1] && t[1] == t[2])
            .count()
    );
    if native.is_empty() {
        return Ok(());
    }
    for (tuple, by_class) in &tuples {
        let total: usize = by_class.values().sum();
        if total < MIN_RECORDS || tuple[0] != tuple[1] || tuple[1] != tuple[2] {
            continue;
        }
        let mut by: Vec<(&String, &usize)> = by_class.iter().collect();
        by.sort_by(|x, y| y.1.cmp(x.1).then(x.0.cmp(y.0)));
        let top: Vec<String> = by
            .iter()
            .take(4)
            .map(|(c, n)| format!("{c:?}:{n}"))
            .collect();
        println!(
            "{{\"tuple\":{},\"records\":{total},\"purity\":{:.3},\"top_class\":{:?},\"classes\":{{{}}}}}",
            tuple[0],
            *by[0].1 as f64 / total as f64,
            by[0].0,
            top.join(",")
        );
    }
    for (class, by_tuple) in &classes {
        let total: usize = by_tuple.values().sum();
        if total < MIN_RECORDS || class.starts_with('<') {
            continue;
        }
        let mut by: Vec<(&[u32; 3], &usize)> = by_tuple.iter().collect();
        by.sort_by(|x, y| y.1.cmp(x.1).then(x.0.cmp(y.0)));
        println!(
            "{{\"class\":{class:?},\"records\":{total},\"distinct_tuples\":{},\"top_tuple\":[{},{},{}],\"concentration\":{:.3}}}",
            by_tuple.len(),
            by[0].0[0],
            by[0].0[1],
            by[0].0[2],
            *by[0].1 as f64 / total as f64
        );
    }
    Ok(())
}

fn main() {
    let paths: Vec<String> = std::env::args().skip(1).collect();
    if paths.is_empty() {
        eprintln!("usage: probe_re174_symmetric_tuples MODEL.rvt ...");
        std::process::exit(2);
    }
    for path in &paths {
        if let Err(err) = probe(path) {
            println!("{{\"file\":{path:?},\"error\":{:?}}}", err.to_string());
        }
    }
}
