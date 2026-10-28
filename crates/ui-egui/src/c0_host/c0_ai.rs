//! `c0.selection` and `c0.place`: what the AI actions of the host read and write.
//!
//! Both go through the engine's own commands. A placement is one engine undo group, so one Undo
//! removes all of it and the document is dirty afterwards like after any user edit.

use gridcraft_engine::{
    core::{CellRef, RangeRef, Value as CellValue},
    display,
};
use serde_json::{Value, json};

use crate::SheetApp;

/// A selection read returns at most this many cells (whole rows; a very wide one keeps one row).
pub const MAX_SELECTION_CELLS: u32 = 2_000;
/// A placement writes at most this many cells.
pub const MAX_PLACE_CELLS: usize = 100_000;

pub(super) fn selection(app: &SheetApp) -> Result<Value, String> {
    let d = app.session.active().ok_or("no workbook open")?;
    let sh = d.wb.active().ok_or("no active sheet")?;
    let selected = d.selection.current();
    let used = sh.used_range();
    let (rows, cols) = (selected.end.row - selected.start.row + 1, selected.end.col - selected.start.col + 1);
    // A whole column or row would otherwise be a million empty cells: keep to the used range.
    let mut end = selected.end;
    if u64::from(rows) * u64::from(cols) > u64::from(MAX_SELECTION_CELLS)
        && let Some(used) = used
    {
        end = CellRef::new(selected.end.row.min(used.end.row.max(selected.start.row)), selected.end.col.min(used.end.col.max(selected.start.col)));
    }
    let (rows, cols) = (end.row - selected.start.row + 1, end.col - selected.start.col + 1);
    let keep_cols = cols.min(MAX_SELECTION_CELLS);
    let keep_rows = rows.min((MAX_SELECTION_CELLS / keep_cols).max(1));
    let truncated = keep_rows < rows || keep_cols < cols;
    let last = CellRef::new(selected.start.row + keep_rows - 1, selected.start.col + keep_cols - 1);
    let range = RangeRef::new(selected.start, last);
    let header_row = used.map(|u| u.start.row).unwrap_or(0);
    let mut values = Vec::new();
    let mut formulas = Vec::new();
    for r in range.start.row..=range.end.row {
        let (mut vrow, mut frow) = (Vec::new(), Vec::new());
        for c in range.start.col..=range.end.col {
            let at = CellRef::new(r, c);
            vrow.push(Value::String(display::cell_text(&d.wb, sh, at)));
            frow.push(sh.cell(at).and_then(|x| x.formula.as_ref()).map(|f| Value::String(format!("={}", f.text))).unwrap_or(Value::Null));
        }
        values.push(vrow);
        formulas.push(frow);
    }
    let headers: Vec<String> = (range.start.col..=range.end.col)
        .map(|c| match sh.value(CellRef::new(header_row, c)) {
            CellValue::Text(text) => text.to_string(),
            _ => String::new(),
        })
        .collect();
    Ok(json!({
        "kind": "cells", "sheet": sh.name, "range": range.a1(), "active": d.selection.active.a1(),
        "values": values, "formulas": formulas, "headers": headers,
        "selection": selected.a1(), "truncated": truncated,
    }))
}

/// One cell of a placement: the engine input text, or `None` to leave the cell alone.
fn input(value: Option<&Value>, formula: Option<&Value>) -> Result<Option<String>, String> {
    match formula {
        None | Some(Value::Null) => {}
        Some(Value::String(f)) if f.starts_with('=') && f.len() > 1 => return Ok(Some(f.clone())),
        Some(_) => return Err("every formula must be a string that starts with '='".into()),
    }
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(text)) => Ok(Some(text.clone())),
        Some(v @ (Value::Number(_) | Value::Bool(_))) => Ok(Some(match v {
            Value::Bool(true) => "TRUE".into(),
            Value::Bool(false) => "FALSE".into(),
            number => number.to_string(),
        })),
        Some(_) => Err("every value must be a string, number or null".into()),
    }
}

fn grid(params: &Value, key: &str) -> Result<Vec<Vec<Value>>, String> {
    match params.get(key) {
        None | Some(Value::Null) => Ok(Vec::new()),
        Some(Value::Array(rows)) => rows
            .iter()
            .map(|row| row.as_array().cloned().ok_or_else(|| format!("{key} must be an array of rows (arrays)")))
            .collect(),
        Some(_) => Err(format!("{key} must be an array of rows (arrays)")),
    }
}

pub(super) fn place(app: &mut SheetApp, params: &Value) -> Result<Value, String> {
    let (at_text, values, formulas) = match (params.get("cells"), params.get("formula")) {
        (Some(cells), None) if cells.is_object() => {
            let at = cells.get("at").and_then(Value::as_str).ok_or("cells.at must be an A1 cell such as \"B2\"")?;
            (at.to_owned(), grid(cells, "values")?, grid(cells, "formulas")?)
        }
        (None, Some(Value::String(f))) => {
            let at = match params.get("at") {
                None | Some(Value::Null) => None,
                Some(Value::String(at)) => Some(at.clone()),
                Some(_) => return Err("at must be an A1 cell such as \"B2\"".into()),
            };
            let at = match at {
                Some(at) => at,
                None => app.session.active().ok_or("no workbook open")?.selection.active.a1(),
            };
            (at, Vec::new(), vec![vec![Value::String(f.clone())]])
        }
        (Some(_), Some(_)) => return Err("give either cells or formula, not both".into()),
        _ => return Err("place needs {cells: {at, values?, formulas?}} or {formula, at?}".into()),
    };
    let at = CellRef::parse(at_text.trim_start_matches('$')).ok_or("at must be an A1 cell such as \"B2\"")?;
    let height = values.len().max(formulas.len());
    let width = values.iter().chain(formulas.iter()).map(|r| r.len()).max().unwrap_or(0);
    if height == 0 || width == 0 {
        return Err("nothing to place: values and formulas are empty".into());
    }
    if height.saturating_mul(width) > MAX_PLACE_CELLS {
        return Err(format!("too many cells: a placement is limited to {MAX_PLACE_CELLS}"));
    }
    let far = at.offset(height as i64 - 1, width as i64 - 1).ok_or("the block does not fit on the sheet from that cell")?;
    let mut writes = Vec::new();
    for r in 0..height {
        for c in 0..width {
            let value = values.get(r).and_then(|row| row.get(c));
            let formula = formulas.get(r).and_then(|row| row.get(c));
            if let Some(text) = input(value, formula).map_err(|e| format!("{} {e}", at.offset(r as i64, c as i64).map(|x| x.a1()).unwrap_or_default()))? {
                writes.push((at.offset(r as i64, c as i64).ok_or("the block does not fit on the sheet")?.a1(), text));
            }
        }
    }
    super::commit_edit(app)?;
    let sheet = app.session.active().ok_or("no workbook open")?.wb.active_sheet.get();
    let name = app.session.active().and_then(|d| d.wb.sheet(sheet).map(|s| s.name.clone())).unwrap_or_default();
    let formula_count = writes.iter().filter(|(_, t)| t.starts_with('=')).count();
    app.session
        .grouped("C0 place", |s| {
            for (cell, input) in &writes {
                s.execute("cell.set", json!({"cell": cell, "input": input}))?;
            }
            Ok(())
        })
        .map_err(|e| e.to_string())?;
    let range = RangeRef::new(at, far);
    app.run("selection.set", json!({"sheet": sheet, "range": range.a1()}))?;
    app.grid.ensure_visible = true;
    Ok(json!({"placed": range.a1(), "sheet": name, "cells": writes.len(), "formulas": formula_count}))
}
