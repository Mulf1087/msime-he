use std::path::Path;

use rusqlite::{params, Connection};

use crate::{EngineError, Result};

/// 专用 UTF-8 文本：首行版本标识，后续为编码、原候选序号、文本三列。成功后原子替换。
pub fn import_tsv(user_root: &Path, text: &str) -> Result<usize> {
    let fail = || EngineError::failed("小鹤音形码表导入失败，请检查文件格式");
    if !user_root.is_absolute() || text.len() > 32 * 1024 * 1024 {
        return Err(fail());
    }
    msime_path_trust::reject_symlinked_components(user_root).map_err(|_| fail())?;
    let mut lines = text.trim_start_matches('\u{feff}').lines();
    if lines.next() != Some("# msime-xiaohe-yinxing-v1") {
        return Err(fail());
    }
    let destination = user_root.join("xiaohe-yinxing.db");
    let personal = if super::XiaoheTable::open(&destination).is_ok() {
        super::list_user_entries(user_root)?
    } else {
        Vec::new()
    };
    if let Ok(metadata) = std::fs::symlink_metadata(&destination) {
        if !metadata.is_file() {
            return Err(fail());
        }
    }
    let temporary = tempfile::NamedTempFile::new_in(user_root).map_err(|_| fail())?;
    let mut connection = Connection::open(temporary.path()).map_err(|_| fail())?;
    connection.execute_batch("PRAGMA user_version=1; CREATE TABLE entries(code TEXT NOT NULL,rank INTEGER NOT NULL,text TEXT NOT NULL,UNIQUE(code,text),UNIQUE(code,rank));CREATE INDEX entries_code_rank ON entries(code,rank)").map_err(|_| fail())?;
    let transaction = connection.transaction().map_err(|_| fail())?;
    {
        let mut insert = transaction
            .prepare("INSERT INTO entries VALUES(?1,?2,?3)")
            .map_err(|_| fail())?;
        for (index, line) in lines.enumerate() {
            if index >= 500_000 {
                return Err(fail());
            }
            let mut fields = line.split('\t');
            let code = fields.next().ok_or_else(fail)?;
            let rank: i64 = fields
                .next()
                .ok_or_else(fail)?
                .parse()
                .map_err(|_| fail())?;
            let word = fields.next().ok_or_else(fail)?;
            if fields.next().is_some() {
                return Err(fail());
            }
            insert
                .execute(params![code, rank, word])
                .map_err(|_| fail())?;
        }
    }
    transaction.commit().map_err(|_| fail())?;
    super::user::write_users(&connection, &personal)?;
    drop(connection);
    let checked = super::XiaoheTable::open(temporary.path())?;
    let count = checked.count();
    drop(checked);
    temporary.as_file().sync_all().map_err(|_| fail())?;
    temporary.persist(&destination).map_err(|_| fail())?;
    Ok(count)
}
