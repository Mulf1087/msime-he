use std::path::Path;

use rusqlite::{Connection, OpenFlags};

use crate::{EngineError, Result};

const INVALID: &str = "小鹤音形码表不可用，请重新导入";
const MAX_BYTES: u64 = 32 * 1024 * 1024;
const MAX_ROWS: usize = 500_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XiaoheEntry {
    pub code: String,
    pub rank: i64,
    pub word: String,
}

pub struct XiaoheTable {
    connection: Connection,
    count: usize,
}

impl XiaoheTable {
    /// 只读打开并完整验证一次，不创建文件，也不迁移用户导入的数据库。
    pub fn open(path: &Path) -> Result<Self> {
        let fail = || EngineError::failed(INVALID);
        let metadata = std::fs::symlink_metadata(path).map_err(|_| fail())?;
        if !metadata.is_file() || metadata.len() > MAX_BYTES {
            return Err(fail());
        }
        let path = crate::paths::sqlite_read_only_path_no_follow(path).map_err(|_| fail())?;
        let connection = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_ONLY
                | OpenFlags::SQLITE_OPEN_NO_MUTEX
                | OpenFlags::SQLITE_OPEN_NOFOLLOW,
        )
        .map_err(|_| fail())?;
        connection
            .execute_batch("PRAGMA trusted_schema=OFF")
            .map_err(|_| fail())?;
        let version: i64 = connection
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .map_err(|_| fail())?;
        if version != 1 {
            return Err(fail());
        }
        let mut statement = connection
            .prepare("SELECT code,rank,text FROM entries ORDER BY code,rank")
            .map_err(|_| fail())?;
        let mut rows = statement.query([]).map_err(|_| fail())?;
        let mut count = 0;
        let mut previous: Option<(String, i64)> = None;
        while let Some(row) = rows.next().map_err(|_| fail())? {
            let code: String = row.get(0).map_err(|_| fail())?;
            let rank: i64 = row.get(1).map_err(|_| fail())?;
            let word: String = row.get(2).map_err(|_| fail())?;
            count += 1;
            if count > MAX_ROWS
                || !super::valid_code(&code)
                || rank < 1
                || word.is_empty()
                || word.encode_utf16().count() > 199
                || word.chars().any(char::is_control)
                || previous
                    .as_ref()
                    .is_some_and(|(p, r)| p == &code && *r == rank)
            {
                return Err(fail());
            }
            previous = Some((code, rank));
        }
        drop(rows);
        drop(statement);
        if count == 0 {
            return Err(fail());
        }
        Ok(Self { connection, count })
    }

    pub fn count(&self) -> usize {
        self.count
    }

    /// 只认完整编码；缺少短码时不把长码候选当成短码候选。
    pub fn query(&self, code: &str) -> Result<Vec<XiaoheEntry>> {
        if !super::valid_code(code) {
            return Ok(Vec::new());
        }
        let fail = || EngineError::failed(INVALID);
        let mut statement = self
            .connection
            .prepare_cached("SELECT code,rank,text FROM entries WHERE code=?1 ORDER BY rank")
            .map_err(|_| fail())?;
        let rows = statement
            .query_map([code], |row| {
                Ok(XiaoheEntry {
                    code: row.get(0)?,
                    rank: row.get(1)?,
                    word: row.get(2)?,
                })
            })
            .map_err(|_| fail())?;
        let mut entries = rows
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|_| fail())?;
        // Older imported databases have no personal table. A newly created table becomes visible
        // to an already open read-only connection on its next query.
        if let Ok(mut users) = self
            .connection
            .prepare_cached("SELECT text FROM user_entries WHERE code=?1 ORDER BY text")
        {
            if let Ok(rows) = users.query_map([code], |row| row.get::<_, String>(0)) {
                let mut rank = entries.iter().map(|entry| entry.rank).max().unwrap_or(0);
                for word in rows.flatten() {
                    if !super::valid_user_entry(code, &word)
                        || entries.iter().any(|entry| entry.word == word)
                    {
                        continue;
                    }
                    rank += 1;
                    entries.push(XiaoheEntry {
                        code: code.to_owned(),
                        rank,
                        word,
                    });
                }
            }
        }
        Ok(entries)
    }
}
