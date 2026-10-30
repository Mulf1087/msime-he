//! 定制安装包的可选音形基线。只补缺表，不覆盖用户表；普通发布包没有清单，行为不变。
use std::io::Write;
use std::path::Path;

use msime_client_core::file_lock;
use msime_client_core::preferences::{
    ChineseScheme, InputScheme, PreferencesError, PreferencesStore,
};
use msime_engine::xiaohe::XiaoheTable;
use serde::Deserialize;
use sha2::{Digest, Sha256};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    format_version: u32,
    sha256: String,
    size: u64,
    entries: usize,
    default_scheme: String,
}

pub(crate) fn install(resources: &Path, state: &Path) -> Result<bool, Box<dyn std::error::Error>> {
    let Some(parent) = resources.parent() else {
        return Ok(false);
    };
    let seed = parent.join("xiaohe-yinxing");
    match std::fs::symlink_metadata(seed.join("manifest.json")) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error.into()),
        Ok(_) => {}
    }
    msime_path_trust::reject_symlinked_components(&seed)?;
    msime_path_trust::reject_symlinked_components(state)?;
    let user = state.join("user");
    msime_path_trust::reject_symlinked_components(&user)?;
    let target = user.join("xiaohe-yinxing.db");
    match std::fs::symlink_metadata(&target) {
        // 损坏或链接的已有表同样保留，不把它当作缺表覆盖，也不默认启用它。
        Ok(_) => return Ok(XiaoheTable::open(&target).is_ok()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    let manifest: Manifest = serde_json::from_slice(&crate::bounded_file::read(
        file_lock::open_private_file_in(seed.join("manifest.json"))?,
        16 * 1024,
    )?)?;
    if manifest.format_version != 1
        || manifest.default_scheme != "xiaohe_yinxing"
        || manifest.size == 0
        || manifest.size > 32 * 1024 * 1024
        || manifest.entries == 0
        || manifest.entries > 500_000
    {
        return Err("invalid bundled Xiaohe manifest".into());
    }
    let bytes = crate::bounded_file::read(
        file_lock::open_private_file_in(seed.join("xiaohe-yinxing.db"))?,
        32 * 1024 * 1024,
    )?;
    if bytes.len() as u64 != manifest.size || hex::encode(Sha256::digest(&bytes)) != manifest.sha256
    {
        return Err("bundled Xiaohe checksum mismatch".into());
    }
    std::fs::create_dir_all(&user)?;
    let directory = file_lock::open_private_directory(&user)?;
    let mut temporary = tempfile::NamedTempFile::new_in(&user)?;
    temporary.write_all(&bytes)?;
    temporary.as_file().sync_all()?;
    if XiaoheTable::open(temporary.path())?.count() != manifest.entries {
        return Err("bundled Xiaohe entry count mismatch".into());
    }
    // 两个进程首次启动同时准备时，晚到者不得替换先到者或用户导入的表。
    match temporary.persist_noclobber(&target) {
        Ok(_) => {
            directory.sync_all()?;
        }
        Err(error) if error.error.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(error) => return Err(error.error.into()),
    }
    Ok(XiaoheTable::open(&target).is_ok())
}

pub(crate) fn select_for_first_install(state: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let store = PreferencesStore::new(state);
    let Ok(snapshot) = store.load() else {
        // 旧版或损坏偏好交给既有恢复机制，不能用定制默认值改写。
        return Ok(());
    };
    if snapshot.revision != 0 {
        return Ok(());
    }
    let mut preferences = snapshot.preferences;
    preferences.scheme = InputScheme::XiaoheYinxing;
    preferences.last_chinese_scheme = Some(ChineseScheme::XiaoheYinxing);
    match store.save(0, preferences) {
        Ok(_) | Err(PreferencesError::Conflict) => Ok(()),
        Err(error) => Err(error.into()),
    }
}
