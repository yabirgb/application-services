-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at http://mozilla.org/MPL/2.0/.

-- This is a very simple schema.
CREATE TABLE IF NOT EXISTS moz_extension_data (
    ext_id TEXT PRIMARY KEY,
    -- The JSON payload. NULL means a tombstone (but see the CHECK below)
    data TEXT,

    -- Same "sync status" strategy used by places components.
    sync_status INTEGER NOT NULL DEFAULT 1, -- 1 == SyncStatus::New
    sync_change_counter INTEGER NOT NULL DEFAULT 1,

    -- We only keep tombstones for items with a sync_status of Normal (and only
    -- then until we upload the tombstone to the server. Enforce that.
    CHECK(sync_status = 2 OR data IS NOT NULL)
) WITHOUT ROWID;

/* index on sync_status? */

CREATE TABLE IF NOT EXISTS moz_extension_data_mirror (
    guid TEXT PRIMARY KEY,
    /* The extension_id is explicitly not the GUID used on the server.
       We may end up making this a regular foreign-key relationship back to
       moz_extension_data, although maybe not - the ext_id may not exist in
       moz_extension_data at the time we populate this table.
       We can iterate here as we site up sync support.
    */
    ext_id TEXT NOT NULL UNIQUE,

    /* Timestamp as recorded by the server */
    server_modified INTEGER NOT NULL,
    /* The JSON payload. We *do* allow NULL here - it means "deleted" */
    data TEXT
) WITHOUT ROWID;

-- This table holds key-value metadata - primarily for sync.
CREATE TABLE IF NOT EXISTS moz_meta (
    key TEXT PRIMARY KEY,
    value NOT NULL
) WITHOUT ROWID;
