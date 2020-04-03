/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

// The "outgoing" part of syncing - building the payloads to upload and
// managing the sync state of the local DB.

use interrupt::Interruptee;
use rusqlite::{Connection, Row};
use sql_support::ConnExt;
use sync15::ServerTimestamp;
use sync_guid::Guid as SyncGuid;

use crate::error::*;

use super::{ServerPayload, SyncStatus};

// This is the "state" for outgoing items - it's so that after we POST the
// outgoing records we can update the local DB.
pub struct OutgoingStateHolder {
    ext_id: String,
    change_counter: i32,
}

pub struct OutgoingInfo {
    state: OutgoingStateHolder,
    payload: ServerPayload,
}

impl OutgoingInfo {
    fn from_row(row: &Row<'_>) -> Result<Self> {
        let guid = row
            .get::<_, Option<SyncGuid>>("guid")?
            .unwrap_or_else(SyncGuid::random);
        let ext_id: String = row.get("ext_id")?;
        let raw_data: Option<String> = row.get("data")?;
        let (data, deleted) = if raw_data.is_some() {
            (raw_data, false)
        } else {
            (None, true)
        };
        Ok(OutgoingInfo {
            state: OutgoingStateHolder {
                ext_id: ext_id.clone(),
                change_counter: row.get("sync_change_counter")?,
            },
            payload: ServerPayload {
                ext_id: ext_id,
                guid,
                data,
                deleted,
                last_modified: ServerTimestamp(0),
            },
        })
    }
}

/// Gets into about what should be uploaded. Returns a vec of the payload which
// should be uploaded, plus the state for those items which should be held
// until the upload is complete, then passed back to record_uploaded.
pub fn get_outgoing<S: ?Sized + Interruptee>(
    conn: &Connection,
    _signal: &S,
) -> Result<Vec<OutgoingInfo>> {
    let sql = "SELECT l.ext_id, l.data, l.sync_change_counter, m.guid
               FROM moz_extension_data l
               LEFT JOIN moz_extension_data_mirror m ON m.ext_id = l.ext_id
               WHERE sync_change_counter > 0";
    let elts = conn
        .conn()
        .query_rows_and_then_named(sql, &[], OutgoingInfo::from_row)?;
    Ok(elts)
}

pub fn record_uploaded<S: ?Sized + Interruptee>(
    conn: &Connection,
    items: &[&OutgoingStateHolder],
    signal: &S,
) -> Result<()> {
    let cext = conn.conn();
    let tx = cext.unchecked_transaction()?;

    let sql = "
        UPDATE moz_extension_data SET
            sync_change_counter = (sync_change_counter - :old_counter),
            sync_status = :sync_status_normal
        WHERE ext_id = :ext_id;";
    for state in items.iter() {
        signal.err_if_interrupted()?;
        conn.execute_named(
            sql,
            rusqlite::named_params! {
                ":old_counter": state.change_counter,
                ":sync_status_normal": SyncStatus::Normal as u8,
                ":ext_id": state.ext_id,
            },
        )?;
    }

    tx.commit()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test::new_mem_db;
    use interrupt::NeverInterrupts;

    #[test]
    fn test_simple() -> Result<()> {
        let db = new_mem_db();
        let conn = db.writer.lock().unwrap();

        conn.execute_batch(
            r#"
            INSERT INTO moz_extension_data (ext_id, data, sync_status, sync_change_counter)
            VALUES
                ('ext_no_changes', '{"foo":"bar"}', 2, 0),
                ('ext_with_changes', '{"foo":"bar"}', 1, 1);

        "#,
        )?;

        let changes = get_outgoing(&conn, &NeverInterrupts)?;
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].state.ext_id, "ext_with_changes".to_string());

        record_uploaded(&conn, &[&changes[0].state], &NeverInterrupts)?;

        // let (counter, status): (i32, u8) = conn.query_row_and_then::<(i32, u8), _, _, _>(
        //     "SELECT sync_change_counter, sync_status FROM moz_extension_data WHERE ext_id = 'ext_with_changes'",
        //     NO_PARAMS,
        //     |row| Ok((row.get::<_, i32>(0)?, row.get::<_, u8>(1)?)))?;

        let counter: i32 = conn.conn().query_one(
            "SELECT sync_change_counter FROM moz_extension_data WHERE ext_id = 'ext_with_changes'",
        )?;
        assert_eq!(counter, 0);

        let status: u8 = conn.conn().query_one(
            "SELECT sync_status FROM moz_extension_data WHERE ext_id = 'ext_with_changes'",
        )?;
        assert_eq!(status, SyncStatus::Normal as u8);
        Ok(())
    }
}
