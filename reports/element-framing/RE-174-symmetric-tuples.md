# RE-174: an ElemTable record's `(N,N,N)` is its history, three save-episode numbers, not a class marker

**Date:** 2026-10-05
**Issues:** #85
**Artefacts:** 47 distinct local models, Revit 2019 to 2027: 32 of Revit 2023 to 2027 (3, 3, 18, 2 and 6 per release) and 15 of 2019 to 2022, which have no class oracle. Measured locally.
**Probe:** `examples/probe_re174_symmetric_tuples.rs`.
**Status:** negative for the issue's premise. The triples are the schema's `ElemRec.m_history`. They do not mark classes, and none of the issue's resource classes is found by them. The 0.2.0 readings (Level `(8,8,8)`, Material `(20,20,20)`, Space `(25,25,25)`) occur on no Level, Material or Room of any model measured.

## 1. The question

#85 says symmetric `(N,N,N)` tuples in `Global/ElemTable` mark project-level resource classes: Level, Material and Space were confirmed at 98 to 100% through `(8,8,8)`, `(20,20,20)` and `(25,25,25)`, and Sheets, Views, Phases, Design Options, Worksets, Parameters, Groups, Links and Revisions should have tuples of their own. Which classes do the tuples mark?

## 2. Method

An ElemTable record holds three `u32`s between its ids and its owner: bytes `8..20` of a 28-byte or a 40-byte record. Most are symmetric.

The oracle is the element's own native record. Its body begins with its class tag, which names the class (`native_document`, Revit 2023 to 2027). Each ElemTable record is joined to the native record of its ElementId. The probe reports, for every symmetric tuple of at least 20 records, how pure its commonest class is, and for every class of at least 20 records, how many tuples it has and how much of it the commonest holds. A class marker would give both near 1.

## 3. Result

### 3.1 What the three words are

`ElemRec`, the table's record class, has four members: `m_history`, `m_id`, `m_OwningElementId`, `m_partitionId`. `native_index::parse` reads `m_history`'s `m_creationDate`, `m_lastModificationDate` and `m_lastUserModificationDate` from this table as the element's `creation_episode`, `stored_revision` and `other_revision`. The triple is those three words: on 651,264 of 651,264 records of the 32 models of Revit 2023 to 2027, it equals them. This agreement is by construction, since both read the same bytes, and says what the words are, not whether a class reading holds.

They are save-episode numbers, indexes into the file's save history (`native_index` indexes its list of the file's creation episodes with `creation_episode`). So `(N,N,N)` is an element created in episode `N` and not modified since.

### 3.2 No tuple marks a class

Over the 32 models, 1,072 symmetric tuples have at least 20 records.

- **Purity.** Of the 243 with at least 100 records, purity is 0.11 to 1.0 with median 0.27, and 14 reach 0.9. Of all 1,072, 132 reach 0.9, 106 of them `GStyleElem` (graphic styles). The rest are `PostedWarningElem` (8), `PropertySetElement` (6), `CurveElem` (4), `AppearanceAssetElem` (3), `StairsPathElement` (2), and one each of `IndependentTag`, `ParamBinding` and `BuildingOperatingYearSchedule`.
- **What a pure tuple is.** A class created in bulk in one save: Snowdon Architectural's 154 `StairsPathElement` are all at `(1903,1903,1903)`, and 98% of the 651 records at `サンプル設備`'s `(5821,5821,5821)` are `BuildingOperatingYearSchedule`. No class of the issue's list (Sheet, View, Phase, DesignOption, Workset, ParameterElement, Group, LinkInstance, Revision) is among them.
- **The same class has many tuples.** Core Interior (Revit 2024), tuple `(0,0,0)` has 2,051 records of four classes: `GStyleElem` 1,242, `HVACLoadSpaceTypeElem` 125, `CategoryElem` 74, `ConceptualConstructionType` 36, all created in episode 0 and not modified since. `GStyleElem` itself has six tuples.

### 3.3 The three 0.2.0 readings

| class | models with 20 or more | tuples per model (median, range) | the commonest tuple holds (median, range) | commonest tuple |
|---|---:|---|---|---|
| `MaterialElem` | 32 | 38 (1 to 120) | 0.21 (0.03 to 1.0) | 20 different ones |
| `RoomElem` | 16 | 10.5 (3 to 63) | 0.37 (0.11 to 0.97) | 15 different ones |
| `Level` | 6 | 16 (5 to 37) | 0.32 (0.14 to 0.61) | 6 different ones |

Levels on the models that have 20 or more:

| model | levels | tuples | commonest |
|---|---:|---:|---|
| Core Interior 2024 | 74 | 5 | `(19,19,19)` 0.61 |
| Golden Nugget Architektur | 100 | 37 | `(423,2766,1349)` 0.14 |
| Snowdon Architectural | 32 | 18 | `(919,2277,1765)` 0.19 |
| `サンプル意匠` | 44 | 16 | `(17678,19354,19287)` 0.43 |

Checked directly on six models (Einhoven 2023, Core Interior 2024, `rac_basic` 2023, `rst_basic` 2024, RE1 2025, `rme_basic` 2027): `(8,8,8)` is on 0 of the symmetric Levels there (Einhoven has 1, Core Interior 70, the others none). `(20,20,20)` is on 0 of the symmetric Materials (36, 82, 17, 17, 28 and 17). `(25,25,25)` is on 0 of the symmetric Rooms and Spaces (`RoomElem`: Core Interior has 161, the others none). The numbers of the first reading are not reproduced by any of these. The tool that read them is not in this repository, so how they were derived is not known here.

### 3.4 Why a class can look pure

Resources a template creates, such as Levels, Materials and Spaces in a fresh project, are created in the same save, so on one model they can share one `N`. A model made the other way, or saved at another time, has another `N` for them. That is a property of the file's history, not of the class.

## 4. What this changes

- **#85's premise is refuted.** Its acceptance criteria ask for 2 more classes at 90% purity: none of the 14 tuples with 100 or more records that reach it is a resource class, and the 132 that reach it over 20 records are bulk-created styles, warnings and sets.
- **No `ProbableClass` mapping is added.** There is no `elem_table_class.rs` any more, and a tuple-to-class table would be wrong on every model but the one it was read from.
- **The correct reading is already in the code.** `native_index::Identity` has `creation_episode`, `stored_revision` and `other_revision` for each element. A class is read from its record body's tag, which `native_document` already reports.
- **The per-class table is the useful output.** The probe prints, for every class of 20 or more records, its tuples. The sweep is one pass over the native records: 7 s on Core Interior, 20 s on Snowdon Architectural, on a Raspberry Pi 5.

## 5. Not done

- **Models before Revit 2023.** 15 models of 2019 to 2022 are read, with their tuples, but the native record path does not read them, so no class is joined. 10 files of 2008 to 2013 do not open (`Malformed BasicFileInfo: no 4-digit Revit version found`).
- **What an episode is.** Whether `N` is a workshared central file's save counter or a local one is not looked at. The report uses only that it is the schema's creation and modification date.
- **The meaning of the 28-byte layout's fourth word.** The 28-byte record has one more word after the triple.

## 6. Reproduce

```text
cargo run --profile ci --example probe_re174_symmetric_tuples -- MODEL.rvt ...
```

One JSON line per file, then a line per tuple and a line per class. Six large 2025 models are read only with the layout fix of RE-175.
