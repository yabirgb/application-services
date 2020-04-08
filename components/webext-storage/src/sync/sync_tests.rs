/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use crate::api::set;
use crate::db::test::new_mem_db;
use crate::error::*;
use crate::sync::incoming::{apply_actions, get_incoming, plan_incoming, stage_incoming};
use crate::sync::outgoing::{get_outgoing, record_uploaded};
use crate::sync::ServerPayload;
use crate::ServerTimestamp;
use interrupt::NeverInterrupts;
use rusqlite::{Connection, Row};
use serde_json::json;
use sql_support::ConnExt;
use sync_guid::Guid;

// Here we try and simulate everything done by a "full sync", just minus the
// engine.
fn do_sync(conn: &Connection, incoming_bsos: Vec<ServerPayload>) -> Result<()> {
    // First we stage the incoming in the temp tables.
    stage_incoming(conn, incoming_bsos, &NeverInterrupts)?;
    // Then we process them getting a Vec of (item, state), which we turn into
    // a Vec of (item, action)
    let actions = get_incoming(conn)?
        .into_iter()
        .map(|(item, state)| (item, plan_incoming(state)))
        .collect();
    apply_actions(&conn, actions, &NeverInterrupts)?;
    // So we've done incoming - do outgoing.
    let outgoing = get_outgoing(conn, &NeverInterrupts)?;
    record_uploaded(conn, &outgoing, &NeverInterrupts)?;
    Ok(())
}

fn get_mirror_data(conn: &Connection, expected_extid: &str) -> Result<Option<String>> {
    let sql = "SELECT ext_id, data FROM moz_extension_data_mirror";

    fn from_row(row: &Row<'_>) -> Result<(String, Option<String>)> {
        Ok((row.get("ext_id")?, row.get("data")?))
    }
    let mut items = conn.conn().query_rows_and_then_named(sql, &[], from_row)?;
    assert_eq!(items.len(), 1);
    let item = items.pop().expect("it exists");
    assert_eq!(item.0, expected_extid);
    Ok(item.1)
}

#[test]
fn test_simple_outgoing_sync() -> Result<()> {
    // So we are starting with an empty local store and empty server store.
    let db = new_mem_db();
    let conn = db.writer.lock().unwrap();
    let data = json!({"key1": "key1-value", "key2": "key2-value"});
    let expected = data.to_string();
    set(&conn, "ext-id", data)?;
    do_sync(&conn, vec![])?;
    let data = get_mirror_data(&conn, "ext-id")?;
    assert_eq!(data, Some(expected));
    Ok(())
}

#[test]
fn test_conflicting_incoming() -> Result<()> {
    let db = new_mem_db();
    let conn = db.writer.lock().unwrap();
    let data = json!({"key1": "key1-value", "key2": "key2-value"});
    set(&conn, "ext-id", data)?;
    // Incoming payload without 'key1' and conflicting for 'key2'
    let payload = ServerPayload {
        guid: Guid::from("guid"),
        ext_id: "ext-id".to_string(),
        data: Some(json!({"key2": "key2-incoming"}).to_string()),
        deleted: false,
        last_modified: ServerTimestamp(0),
    };
    do_sync(&conn, vec![payload])?;
    let data = get_mirror_data(&conn, "ext-id")?;
    let expected = json!({"key1": "key1-value", "key2": "key2-incoming"});
    assert_eq!(data, Some(expected.to_string()));
    Ok(())
}
