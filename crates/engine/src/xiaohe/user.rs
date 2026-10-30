use std::path::Path;

use rusqlite::{params, Connection, OpenFlags};

use crate::{EngineError, Result};

const MAX_USER_ROWS: i64 = 20_000;
const USER_TABLE: &str = "CREATE TABLE IF NOT EXISTS user_entries(code TEXT NOT NULL,text TEXT NOT NULL,PRIMARY KEY(code,text))";

fn fail() -> EngineError {
    EngineError::failed("小鹤音形个人词条不可用")
}

fn connection(user_root: &Path, write: bool) -> Result<Connection> {
    if !user_root.is_absolute() {
        return Err(fail());
    }
    msime_path_trust::reject_symlinked_components(user_root).map_err(|_| fail())?;
    let path = user_root.join("xiaohe-yinxing.db");
    if !std::fs::symlink_metadata(&path)
        .map_err(|_| fail())?
        .is_file()
    {
        return Err(fail());
    }
    let flags = if write {
        OpenFlags::SQLITE_OPEN_READ_WRITE
    } else {
        OpenFlags::SQLITE_OPEN_READ_ONLY
    } | OpenFlags::SQLITE_OPEN_NOFOLLOW
        | OpenFlags::SQLITE_OPEN_NO_MUTEX;
    let path = if write {
        crate::paths::sqlite_path_no_follow(&path)
    } else {
        crate::paths::sqlite_read_only_path_no_follow(&path)
    }
    .map_err(|_| fail())?;
    let db = Connection::open_with_flags(&path, flags).map_err(|_| fail())?;
    db.execute_batch("PRAGMA trusted_schema=OFF")
        .map_err(|_| fail())?;
    Ok(db)
}

pub fn list_user_entries(user_root: &Path) -> Result<Vec<(String, String)>> {
    let db = connection(user_root, false)?;
    let exists: bool = db
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='user_entries')",
            [],
            |row| row.get(0),
        )
        .map_err(|_| fail())?;
    if !exists {
        return Ok(Vec::new());
    }
    let mut statement = db
        .prepare("SELECT code,text FROM user_entries ORDER BY code,text LIMIT 20001")
        .map_err(|_| fail())?;
    let rows = statement
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|_| fail())?;
    let entries = rows
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|_| fail())?;
    if entries.len() as i64 > MAX_USER_ROWS
        || entries
            .iter()
            .any(|(code, word)| !valid_user_entry(code, word))
    {
        return Err(fail());
    }
    Ok(entries)
}

pub fn valid_user_entry(code: &str, word: &str) -> bool {
    super::valid_code(code)
        && !word.is_empty()
        && word.encode_utf16().count() <= 199
        && !word.chars().any(char::is_control)
}

/// Caller serializes edits with dictionary maintenance; a repeated add or delete is idempotent.
pub fn edit_user_entry(user_root: &Path, code: &str, word: &str, add: bool) -> Result<()> {
    if !valid_user_entry(code, word) {
        return Err(fail());
    }
    let mut db = connection(user_root, true)?;
    db.execute_batch(USER_TABLE).map_err(|_| fail())?;
    let transaction = db.transaction().map_err(|_| fail())?;
    if add {
        let count: i64 = transaction
            .query_row("SELECT COUNT(*) FROM user_entries", [], |row| row.get(0))
            .map_err(|_| fail())?;
        if count >= MAX_USER_ROWS {
            let exists: bool = transaction
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM user_entries WHERE code=?1 AND text=?2)",
                    params![code, word],
                    |row| row.get(0),
                )
                .map_err(|_| fail())?;
            if !exists {
                return Err(fail());
            }
        }
        transaction
            .execute(
                "INSERT OR IGNORE INTO user_entries(code,text) VALUES(?1,?2)",
                params![code, word],
            )
            .map_err(|_| fail())?;
    } else {
        transaction
            .execute(
                "DELETE FROM user_entries WHERE code=?1 AND text=?2",
                params![code, word],
            )
            .map_err(|_| fail())?;
    }
    transaction.commit().map_err(|_| fail())
}

pub(super) fn write_users(db: &Connection, entries: &[(String, String)]) -> Result<()> {
    db.execute_batch(USER_TABLE).map_err(|_| fail())?;
    let mut insert = db
        .prepare("INSERT INTO user_entries(code,text) VALUES(?1,?2)")
        .map_err(|_| fail())?;
    for (code, word) in entries {
        insert.execute(params![code, word]).map_err(|_| fail())?;
    }
    Ok(())
}
