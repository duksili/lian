//! Generic row -> JSON mapping so repository reads stay compact and the
//! frontend receives snake_case objects mirroring the schema contracts.

use rusqlite::types::ValueRef;
use rusqlite::{Connection, Params, Statement};
use serde_json::{Map, Value};

use crate::Result;

/// Columns whose TEXT content is stored JSON and must be surfaced as JSON.
const JSON_COLUMNS: &[&str] = &[
    "subtypes",
    "context_tags",
    "tags",
    "weekdays",
    "device_metadata",
    "pre_test",
    "validity_reasons",
    "derived_metrics",
    "payload",
    "exposure_definition",
    "outcome_definition",
    "time_window",
    "caveats",
    "source_data_scope",
    "values_json",
    "primary_outcome_definition",
    "prior_values",
    "value",
];

fn cell_to_json(v: ValueRef<'_>, col: &str) -> Value {
    match v {
        ValueRef::Null => Value::Null,
        ValueRef::Integer(i) => Value::from(i),
        ValueRef::Real(f) => Value::from(f),
        ValueRef::Text(t) => {
            let s = String::from_utf8_lossy(t).to_string();
            if JSON_COLUMNS.contains(&col) {
                serde_json::from_str(&s).unwrap_or(Value::String(s))
            } else {
                Value::String(s)
            }
        }
        ValueRef::Blob(_) => Value::Null,
    }
}

fn stmt_rows_to_json<P: Params>(stmt: &mut Statement<'_>, params: P) -> Result<Vec<Value>> {
    let names: Vec<String> = stmt.column_names().iter().map(|s| s.to_string()).collect();
    let mut rows = stmt.query(params)?;
    let mut out = Vec::new();
    while let Some(row) = rows.next()? {
        let mut obj = Map::new();
        for (i, name) in names.iter().enumerate() {
            obj.insert(name.clone(), cell_to_json(row.get_ref(i)?, name));
        }
        out.push(Value::Object(obj));
    }
    Ok(out)
}

pub fn query_json<P: Params>(conn: &Connection, sql: &str, params: P) -> Result<Vec<Value>> {
    let mut stmt = conn.prepare(sql)?;
    stmt_rows_to_json(&mut stmt, params)
}

pub fn query_one<P: Params>(conn: &Connection, sql: &str, params: P) -> Result<Option<Value>> {
    Ok(query_json(conn, sql, params)?.into_iter().next())
}

/// Snapshot a row as JSON for the audit trail; returns None when missing.
pub fn snapshot(conn: &Connection, table: &str, id: &str) -> Result<Option<Value>> {
    // Table names are internal constants, never user input.
    let sql = format!("SELECT * FROM {table} WHERE id = ?1");
    query_one(conn, &sql, [id])
}
