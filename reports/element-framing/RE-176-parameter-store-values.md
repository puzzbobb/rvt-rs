# RE-176: parameter values are stored id-first in an element's own data, a height checks against Revit's export, and a millimetre length takes three float paths

**Date:** 2026-10-06
**Issues:** #155 (and #35)
**Artefacts:** `02_2025.zip` from #356 (Apache-2.0): 23 purpose-made metric Revit 2025 projects, 21 with walls and Revit's own IFC4 export; plus the Revit samples named in §3.4 and §3.5. Measured locally.
**Probe:** `examples/probe_re176_mm_encodings.rs`.
**Status:** partial. Three of #155's five checks are answered and two are not.
- **Answered.** The id-first shape, with one `BuiltInParameter` verified against Revit's export (85 of 85 walls); the bit-exact encoding of a length entered in millimetres; and the definition side of the shared-parameter GUID.
- **Not answered.** The extrusion and revolution ids have no oracle here, and the `PartAtom` half of the GUID chain cannot be tested on these files.

## 1. The questions

#155 reports, from Discussion #112, that the parameter store has two record shapes, that `BuiltInParameter` ids identify extrusion and revolution values, that shared parameters resolve by `PartAtom` GUID, and that Revit computes feet as `mm / 25.4 / 12`, one ULP from `mm / 304.8`. It asks for each to be reproduced or refuted from allowed byte evidence.

## 2. Method

- **Encodings.** For every whole millimetre from 1 to 20,000 that is not a whole inch (a length in whole inches is stored as `inches / 12`), the probe takes the eight bytes of three forms: **A** `mm / 25.4 / 12`, **B** `mm / 304.8` and **C** `mm * (1 / 304.8)`. It counts every occurrence of each pattern in every inflated partition, at any alignment, for the values on which the three forms do not all agree. There is no shape assumption.
- **Shape.** Each occurrence is classified by its neighbours: id-first when the eight bytes before it are a negative `BuiltInParameter` id (-1,000,000 to -2,000,000), value-first when `ff × 8 · i64 id` follows (RE-93), neither otherwise.
- **Oracle for an id.** A parameter's value is compared with Revit's own IFC4 export of the same element.

## 3. Result

### 3.1 The two shapes

| shape | layout | where | evidence |
|---|---|---|---|
| id-first | `i64 BuiltInParameter · f64 value` | an element's own data object | §3.2: 85 of 85 walls' Unconnected Height |
| value-first | `f64 value · ff × 8 · i64 parameter` | a family type's value block | RE-93: 66 of 66 windows' Width and Height on Snowdon |

(RE-77 and RE-117 read the same id-first form for text: `i64 id · u32 n · UTF-16 × n`. `Element`'s first four members in `Formats/Latest` are `m_pParamValueSetDouble`, `m_pParamValueSetInt`, `m_pParamValueSetAString` and `m_pParamValueSetElementId`.)

The two do not overlap: in the 23 metric models 206 millimetre values are id-first, none value-first (these models hold no family type block), and the other 17,709 are in neither shape. They are geometry, such as sketch coordinates. A float found at a bare offset is therefore usually not a parameter.

### 3.2 One id verified against Revit's export

`-1001105` (`WALL_USER_HEIGHT_PARAM`, Unconnected Height in the UI) is an id-first entry in each wall's own data object. Against Revit's IFC4 export of the same wall, on the 21 models of `02_2025.zip` that have walls:

| walls | stored height equals the height of Revit's body |
|---|---|
| 85 free-standing | 85 equal, 0 differ |
| 32 attached to a roof | not compared: their top follows the roof |

The stored values are 4.000 m (13.1234 ft) on most walls and 6.222 m (20.4134 ft) on `2789`. The attached walls store the same 4.000 m, and Revit's body is shorter (wall `2892`: 5.949 m on `02_1Slope`), as the `JoinToRoofGStep` of RE-172 predicts.

The other parameter-shaped lengths in these models, `-1007259` (`STAIRS_ATTR_STAIRS_CUT_OFFSET`, 600 mm) and `-1007203` (`STAIRS_ATTR_MINIMUM_TREAD_DEPTH`, 180 mm), are in 23 models each. They are not compared with an export here.

### 3.3 Extrusion and revolution ids

Of the ids #155 names, `-1001800`, `-1001801` (extrusion start and end), `-1001802` and `-1001803` (revolution start and end angle) are present on `rac_basic_sample_project` 2024, 593, 609, 31 and 31 times. They are id-first: `-1001801` is followed by an `f64` (0.0860 ft and 0.025 ft among them) and `-1001800` and `-1001802` by a `u32` and the next id. None is in the value-first shape on any of 12 models. **No oracle holds an extrusion or a revolution**, so none of these ids is verified here.

### 3.4 Shared parameters

- **Definition.** A shared parameter's definition holds the string `revit.local.shared:<32 hex digits>-1.0.0`: a GUID without dashes, then a version. Family parameters carry `revit.local.family:` and the same shape; `revit.local.project:` and `revit.local.classification:` also occur. The 10 models measured hold 564 such strings (RE1 Architecture 230, `rac_basic` 2025 290, the rest 1 to 11).
- **Across models.** 21 of 535 distinct GUIDs are in two or more models. The name found beside the definition is the same in every model for each of the 19 that resolve one, and no GUID has two names. The name finder reads the entry before the GUID and is a heuristic.
- **`PartAtom`.** Only 3 of the 118 readable models carry a `PartAtom` (the family samples). Their parameters are of type `system` (32), `custom` (45) and `user` (12); none is `shared` and none holds a GUID. So **the `PartAtom` link is not tested.** No chain is resolved.

### 3.5 Millimetres to feet

The reported one-ULP difference between the two expressions is real, and larger than reported:

- **How often.** `mm / 25.4 / 12` and `mm / 304.8` differ for **17,121 of the 30,000** whole millimetres (57.1%), by one ULP for 16,375 and by two for 746.
- **No single form.** In the 23 models, 1,584 (model, value) pairs have a value the forms tell apart. One form reproduces every occurrence of a value as follows:

| form | reproduces all occurrences of | |
|---|---:|---|
| A `mm / 25.4 / 12` | 414 | 26.1% |
| B `mm / 304.8` | 933 | 58.9% |
| **C `mm * (1 / 304.8)`** | **1,489** | **94.0%** |
| none (the value is stored two ways) | 76 | 4.8% |

  Of the 17,915 occurrences, the support is B and C 67.7%, A and C 25.2%, A alone 5.8%, C alone 1.0% and B alone 0.2%. C equals B on 84.3% of whole millimetres, and A on 53.2%.
- **By value.** The form a value takes is a property of the number: 85 of the 88 millimetre values found are reproduced by the same forms in every model they occur in. 6222 is B and C (not A), 600 is A and C (not B), 180 is B and C.
- **In the parameter store.** The three genuine length parameters of §3.2 give three values: Unconnected Height 6222 mm is stored as B and C, Extend Below Base 600 mm as A and C, and Minimum Tread Depth 180 mm as B and C. **Only C reproduces all three.** A reproduces one and B two.

So no form alone finds every stored length. C is the best single form (it supports 93.9% of the occurrences), and the two forms #155 names, together, support 98.9%: the remaining 1.0% occur only as C. This is a result on 23 related models, not a statement about how Revit works.

## 4. What this changes

- **Exact searches.** A search for a length entered in millimetres should try all three forms. `mm / 25.4 / 12` and `mm / 304.8` together miss the 1.0% of occurrences that only the reciprocal product `mm * (1.0 / 304.8)` reproduces, and each alone leaves 6% to 74% of values unreproduced. The three agree on a number of values, so a search with one still hits those.
- **#35.** An id-first `f64` entry is a parameter value in the element's own data, joined to its element by the Adler-32-verified object that holds it (RE-153). `-1001105` is an Unconnected Height that checks against Revit's export.
- **#155.** Three of its checks are answered, as above; no code change.

## 5. Not done

- **Extrusion and revolution ids** need a model whose extrusion start and end, or revolution angles, are known. A family with an extrusion of a stated depth, with its IFC, would settle it.
- **Shared-parameter name resolution through `PartAtom`** needs a family saved with a shared parameter and its `PartAtom`.
- **Imperial models** are not covered: a whole-inch value was left out, and no oracle was run for lengths entered in feet and inches.
- **A discriminating model.** Only 3 values are genuine lengths in the parameter store here. Walls whose Unconnected Height is one of the 518 whole millimetres on which A, B and C all differ would test it directly. Round ones include 2150, 2250, 2350, 4300, 4500, 4700, 8600, 9000 and 9400 mm.
- **Integration into the parameter model of #35** is not done.

## 6. Reproduce

```text
cargo run --profile ci --example probe_re176_mm_encodings -- MODEL.rvt ...
```

One JSON line per file with the support of each form, a `shapes` line, the parameter-shaped values, then every millimetre value found. `aggregate.py` in the package of this PR turns them into the tables above.
