//! 小鹤音形独立码表；候选顺序来自导入的参考方案，不参与拼音调频。

mod import;
mod table;
mod user;
pub use import::import_tsv;
pub(crate) mod scheme;
pub use table::{XiaoheEntry, XiaoheTable};
pub use user::{edit_user_entry, list_user_entries, valid_user_entry};

/// 一至四位小写字母，或分号引导的一位快符编码。
pub fn valid_code(code: &str) -> bool {
    (1..=4).contains(&code.len()) && code.bytes().all(|c| c.is_ascii_lowercase())
        || code == ";"
        || (code.len() == 2 && code.starts_with(';') && code.as_bytes()[1].is_ascii_lowercase())
}
