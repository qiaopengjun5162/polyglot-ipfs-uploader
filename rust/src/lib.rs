use std::{fs, path::Path};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

// ✅ 定义元数据结构体
#[derive(Serialize, Deserialize, Debug)]
pub struct Attribute {
    pub trait_type: String,
    pub value: serde_json::Value,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct NftMetadata {
    pub name: String,
    pub description: String,
    pub image: String,
    pub attributes: Vec<Attribute>,
}

// ✅ 共享的辅助函数
pub fn copy_directory(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;
    for entry in WalkDir::new(src) {
        let entry = entry?;
        let path = entry.path();
        let relative_path = path.strip_prefix(src)?;
        let dest_path = dst.join(relative_path);
        if path.is_dir() {
            fs::create_dir_all(&dest_path)?;
        } else {
            fs::copy(path, &dest_path)?;
        }
    }
    Ok(())
}
