//! RE-176 (probe): how Revit stores a length entered in millimetres, bit for
//! bit (#155).
//!
//! #155 reports that Revit computes internal feet as `mm / 25.4 / 12`, which
//! can differ by one ULP from `mm / 304.8`, so an exact-bit search must try
//! both. This probe checks which form stored values take, and adds a third
//! that also reaches feet: `mm * (1 / 304.8)`.
//!
//! For every whole millimetre value 1 to [`MAX_MM`] that is not a whole inch
//! (a value in whole inches is stored as `inches / 12`) and whose forms do not
//! all give the same bits, it takes the eight bytes of each form and counts
//! every occurrence in every inflated partition, at any alignment. An
//! occurrence supports the forms that give those bits. There is no shape
//! assumption.
//!
//! Each occurrence is also classified by its neighbours, for #155's two record
//! shapes: `id-first` when the eight bytes before it are a negative
//! BuiltInParameter id (`i64 id · f64`), `value-first` when `ff × 8 · i64 id`
//! follows it (`f64 · ff × 8 · i64 id`, RE-93), `both`, or `neither`.
//!
//! Output, per file: the values found, how many occurrences support each form
//! (A: `mm / 25.4 / 12`, B: `mm / 304.8`, C: `mm * (1 / 304.8)`), how many
//! values some form fails to reproduce, and the values found with the forms
//! each occurrence supports.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re176_mm_encodings -- MODEL.rvt ...

use rvt::RevitFile;
use std::collections::{BTreeMap, HashMap};

/// Largest millimetre value tried.
const MAX_MM: u32 = 20_000;
/// Values printed per file.
const SHOWN: usize = 400;

/// The three forms, as `(letter, bit of the support mask, function)`.
type Form = (char, u8, fn(f64) -> f64);
const FORMS: [Form; 3] = [
    ('A', 1, |mm| mm / 25.4 / 12.0),
    ('B', 2, |mm| mm / 304.8),
    ('C', 4, |mm| mm * (1.0 / 304.8)),
];

fn is_whole_inch(mm: u32) -> bool {
    let inches = f64::from(mm) / 25.4;
    (inches - inches.round()).abs() < 1e-9 * inches.max(1.0)
}

/// `bits -> (mm, forms that give them)` for every value the forms tell apart.
fn table() -> HashMap<u64, (u32, u8)> {
    let mut out: HashMap<u64, (u32, u8)> = HashMap::new();
    for mm in 1..=MAX_MM {
        let bits: Vec<u64> = FORMS
            .iter()
            .map(|(_, _, f)| f(f64::from(mm)).to_bits())
            .collect();
        if bits.iter().all(|b| *b == bits[0]) || is_whole_inch(mm) {
            continue;
        }
        for ((_, mask, _), bits) in FORMS.iter().zip(&bits) {
            out.entry(*bits).or_insert((mm, 0)).1 |= mask;
        }
    }
    out
}

fn probe(path: &str, table: &HashMap<u64, (u32, u8)>) -> rvt::Result<()> {
    let file = std::path::Path::new(path)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let mut rf = RevitFile::open(path)?;
    let revit = rf.basic_file_info()?.version;
    // mm -> occurrences by support mask.
    let mut found: BTreeMap<u32, BTreeMap<u8, usize>> = BTreeMap::new();
    let mut shapes: BTreeMap<&str, usize> = BTreeMap::new();
    let mut ids: BTreeMap<(&str, i64), usize> = BTreeMap::new();
    // (parameter, mm, support mask) -> occurrences, for parameter-shaped values.
    let mut parameter_values: BTreeMap<(i64, u32, u8), usize> = BTreeMap::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
        for (at, window) in buf.windows(8).enumerate() {
            if !(window[7] == 0x3f || window[7] == 0x40) {
                continue;
            }
            let bits = u64::from_le_bytes(window.try_into().expect("8 bytes"));
            if let Some(&(mm, mask)) = table.get(&bits) {
                *found.entry(mm).or_default().entry(mask).or_default() += 1;
                let id_at = |from: usize| -> Option<i64> {
                    let id = i64::from_le_bytes(buf.get(from..from + 8)?.try_into().ok()?);
                    (-2_000_000..=-1_000_000).contains(&id).then_some(id)
                };
                let before = at.checked_sub(8).and_then(id_at);
                let after = (buf.get(at + 8..at + 16) == Some(&[0xff; 8]))
                    .then(|| id_at(at + 16))
                    .flatten();
                let shape = match (before, after) {
                    (Some(_), None) => "id-first",
                    (None, Some(_)) => "value-first",
                    (Some(_), Some(_)) => "both",
                    (None, None) => "neither",
                };
                *shapes.entry(shape).or_default() += 1;
                if let Some(id) = before.or(after) {
                    *ids.entry((shape, id)).or_default() += 1;
                    *parameter_values.entry((id, mm, mask)).or_default() += 1;
                }
            }
        }
    }
    let mut support = [0usize; 3];
    let mut unreached = [0usize; 3];
    for masks in found.values() {
        for (k, (_, bit, _)) in FORMS.iter().enumerate() {
            let n: usize = masks
                .iter()
                .filter(|(m, _)| *m & bit != 0)
                .map(|(_, n)| n)
                .sum();
            support[k] += n;
            if masks.keys().any(|m| m & bit == 0) {
                unreached[k] += 1;
            }
        }
    }
    println!(
        "{{\"file\":{file:?},\"revit\":{revit},\"values_found\":{},\"occurrences_supporting\":{{\"A\":{},\"B\":{},\"C\":{}}},\"values_a_form_does_not_reproduce\":{{\"A\":{},\"B\":{},\"C\":{}}}}}",
        found.len(),
        support[0],
        support[1],
        support[2],
        unreached[0],
        unreached[1],
        unreached[2]
    );
    let shape_parts: Vec<String> = shapes.iter().map(|(k, n)| format!("\"{k}\":{n}")).collect();
    println!("{{\"shapes\":{{{}}}}}", shape_parts.join(","));
    let mut top: Vec<(&(&str, i64), &usize)> = ids.iter().collect();
    top.sort_by(|x, y| y.1.cmp(x.1).then(x.0.cmp(y.0)));
    for ((shape, id), n) in top.into_iter().take(10) {
        println!("{{\"shape\":\"{shape}\",\"parameter\":{id},\"occurrences\":{n}}}");
    }
    for ((id, mm, mask), n) in &parameter_values {
        let letters: String = FORMS
            .iter()
            .filter(|(_, bit, _)| mask & bit != 0)
            .map(|(l, _, _)| *l)
            .collect();
        println!(
            "{{\"parameter_value\":{{\"parameter\":{id},\"mm\":{mm},\"supported_by\":\"{letters}\",\"occurrences\":{n}}}}}"
        );
    }
    for (mm, masks) in found.iter().take(SHOWN) {
        let by: Vec<String> = masks
            .iter()
            .map(|(m, n)| {
                let letters: String = FORMS
                    .iter()
                    .filter(|(_, bit, _)| m & bit != 0)
                    .map(|(l, _, _)| *l)
                    .collect();
                format!("\"{letters}\":{n}")
            })
            .collect();
        println!("{{\"mm\":{mm},\"supported_by\":{{{}}}}}", by.join(","));
    }
    Ok(())
}

fn main() {
    let paths: Vec<String> = std::env::args().skip(1).collect();
    if paths.is_empty() {
        eprintln!("usage: probe_re176_mm_encodings MODEL.rvt ...");
        std::process::exit(2);
    }
    let table = table();
    eprintln!("{} bit patterns", table.len());
    for path in &paths {
        if let Err(err) = probe(path, &table) {
            println!("{{\"file\":{path:?},\"error\":{:?}}}", err.to_string());
        }
    }
}
