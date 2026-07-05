# Local-First Privacy, Data Ownership, Backup, and Export

## Local-first contract

- The application must function without a user account or network connection.
- The SQLite database is the source of truth.
- The user owns all entered, derived, and exported data.
- No data leaves the device without an explicit user action.
- No telemetry is required for core use.

## Storage

The implementation must identify the local database location clearly and provide an in-app “open data location” action where platform policy permits.

Data should be stored in a durable application-data directory, not inside the install directory.

## Backup

v1 requires:

- manual backup creation;
- configurable backup destination;
- backup includes database and any required metadata/assets;
- backup manifest with creation time, app version, schema version, and checksum;
- restore workflow with confirmation and a safety copy of current data;
- clear reporting of backup success/failure.

Automatic scheduled backup is strongly recommended once the base workflow is stable.

## Export

v1 requires export of:

- human-readable CSV for main domain tables;
- analysis-friendly export format, preferably Parquet and/or SQLite copy;
- assessment raw trials;
- protocol definitions;
- analysis result metadata;
- data dictionary or export manifest.

Exports must preserve identifiers, timestamps, timezone context, source/provenance, validity state, and protocol versions.

## Import

No general import is required in v1. The architecture may reserve an import boundary for later wearable or CSV sources, but manual data must remain distinguishable from imported data.

## Corrections and deletion

- Users can edit or delete their own records.
- Edited records should preserve `updated_at`.
- Where reasonable, corrections should be auditable without blocking user control.
- Deleting a source record must not silently leave a misleading derived result. Derived views should recompute or disclose staleness.
- A user must be able to delete local data permanently.

## Privacy-sensitive UI

- Five Precepts reflection notes are private and must not appear in generic system notifications.
- Lock-screen notification text should be configurable or minimal.
- No externally visible notification should reveal sensitive notes or assessment results by default.
