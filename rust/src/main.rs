use anyhow::{Result, anyhow};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use walkdir::WalkDir;

// ✅ 配置开关
const USE_JSON_SUFFIX: bool = false;

// ✅ 定义元数据结构体
#[derive(Serialize, Deserialize)]
struct Attribute {
    trait_type: String,
    value: serde_json::Value,
}

#[derive(Serialize, Deserialize)]
struct NftMetadata {
    name: String,
    description: String,
    image: String,
    attributes: Vec<Attribute>,
}

// 核心上传函数 (使用 std::process::Command)
fn upload_to_ipfs(target_path: &Path) -> Result<String> {
    if !target_path.exists() {
        return Err(anyhow!("❌ 路径不存在: {:?}", target_path));
    }

    let path_str = target_path
        .to_str()
        .ok_or_else(|| anyhow!("无效的文件路径"))?;
    println!(
        "\n--- 正在执行上传命令: ipfs add -r -Q --cid-version 1 {} ---",
        path_str
    );

    let output = Command::new("ipfs")
        .arg("add")
        .arg("-r") // 递归上传
        .arg("-Q") // 只输出根 CID
        .arg("--cid-version")
        .arg("1")
        .arg(path_str)
        .output()?;

    if !output.status.success() {
        return Err(anyhow!(
            "❌ 上传失败: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let cid = String::from_utf8(output.stdout)?.trim().to_string();
    println!("✅ 上传成功!");
    println!(
        "   - 名称: {}",
        target_path.file_name().unwrap().to_str().unwrap()
    );
    println!("   - CID: {}", cid);
    Ok(cid)
}

// 上传 JSON 数据的专用函数
fn upload_json_str_to_ipfs(data: &NftMetadata) -> Result<String> {
    println!("\n--- 正在上传 JSON 对象 ---");
    let json_string = serde_json::to_string(data)?;

    let mut child = Command::new("ipfs")
        .arg("add")
        .arg("-Q")
        .arg("--cid-version")
        .arg("1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    // 将 JSON 字符串写入子进程的标准输入
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(json_string.as_bytes())?;
    }

    let output = child.wait_with_output()?;
    if !output.status.success() {
        return Err(anyhow!(
            "❌ 上传 JSON 失败: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let cid = String::from_utf8(output.stdout)?.trim().to_string();
    println!("✅ JSON 元数据上传成功!\n   - CID: {}", cid);
    Ok(cid)
}

// 工作流一：处理单个 NFT
fn process_single_nft(image_path: &Path) -> Result<()> {
    println!("\n==============================================");
    println!("🚀 开始处理单个 NFT...");
    println!(
        "   - 文件后缀模式: {}",
        if USE_JSON_SUFFIX { ".json" } else { "无" }
    );
    println!("==============================================");

    let image_cid = upload_to_ipfs(image_path)?;
    println!("\n🖼️  图片 CID 已获取: {}", image_cid);

    let image_filename = image_path
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| anyhow!("无效的图片文件名"))?;
    let image_name_without_ext = image_path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| anyhow!("无效的图片文件名"))?;

    let metadata = NftMetadata {
        name: image_name_without_ext.to_string(),
        description: format!("这是一个为图片 {} 动态生成的元数据。", image_filename),
        image: format!("ipfs://{}", image_cid),
        attributes: vec![Attribute {
            trait_type: "类型".to_string(),
            value: serde_json::Value::String("单件艺术品".to_string()),
        }],
    };

    let metadata_cid = upload_json_str_to_ipfs(&metadata)?;

    let output_dir = PathBuf::from("output").join(image_name_without_ext);
    fs::create_dir_all(&output_dir)?;
    fs::copy(image_path, output_dir.join(image_filename))?;

    let file_name = if USE_JSON_SUFFIX {
        format!("{}.json", image_name_without_ext)
    } else {
        image_name_without_ext.to_string()
    };
    let mut metadata_file = File::create(output_dir.join(file_name))?;
    let pretty_json = serde_json::to_string_pretty(&metadata)?;
    metadata_file.write_all(pretty_json.as_bytes())?;

    println!("\n💾 图片和元数据已在本地打包保存至: {:?}", output_dir);
    println!("\n--- ✨ 单件流程完成 ✨ ---");
    println!(
        "下一步，您可以在 mint 函数中使用这个元数据 URI: ipfs://{}",
        metadata_cid
    );
    Ok(())
}

// 工作流二：处理批量 NFT 集合
fn process_batch_collection(images_input_dir: &Path) -> Result<()> {
    println!("\n==============================================");
    println!("🚀 开始处理批量 NFT 集合...");
    println!(
        "   - 文件后缀模式: {}",
        if USE_JSON_SUFFIX { ".json" } else { "无" }
    );
    println!("==============================================");

    let images_folder_cid = upload_to_ipfs(images_input_dir)?;
    println!("\n🖼️  图片文件夹 CID 已获取: {}", images_folder_cid);

    let timestamp = Utc::now().format("%Y%m%d_%H%M%S").to_string();
    let collection_output_dir = PathBuf::from("output").join(format!("collection_{}", timestamp));
    let images_output_dir = collection_output_dir.join("images");
    let metadata_output_dir = collection_output_dir.join("metadata");

    copy_directory(images_input_dir, &images_output_dir)?;
    println!("\n💾 所有图片已复制到: {:?}", images_output_dir);

    println!("\n--- 正在为每张图片生成元数据 JSON 文件 ---");
    fs::create_dir_all(&metadata_output_dir)?;

    let mut image_files: Vec<PathBuf> = fs::read_dir(images_input_dir)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .collect();
    image_files.sort();

    for image_file in &image_files {
        let token_id_str = image_file
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| anyhow!("无效的文件名"))?;
        let token_id: u64 = token_id_str.parse()?;
        let image_filename = image_file
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or_else(|| anyhow!("无效的文件名"))?;

        let metadata = NftMetadata {
            name: format!("MetaCore #{}", token_id),
            description: "MetaCore 集合中的一个独特成员。".to_string(),
            image: format!("ipfs://{}/{}", images_folder_cid, image_filename),
            attributes: vec![Attribute {
                trait_type: "ID".to_string(),
                value: serde_json::Value::Number(token_id.into()),
            }],
        };
        let file_name = if USE_JSON_SUFFIX {
            format!("{}.json", token_id_str)
        } else {
            token_id_str.to_string()
        };
        let mut file = File::create(metadata_output_dir.join(file_name))?;
        let pretty_json = serde_json::to_string_pretty(&metadata)?;
        file.write_all(pretty_json.as_bytes())?;
    }
    println!(
        "✅ 成功生成 {} 个元数据文件到: {:?}",
        image_files.len(),
        metadata_output_dir
    );

    let metadata_folder_cid = upload_to_ipfs(&metadata_output_dir)?;
    println!("\n📄 元数据文件夹 CID 已获取: {}", metadata_folder_cid);
    println!("\n--- ✨ 批量流程完成 ✨ ---");
    println!(
        "下一步，您可以在合约中将 Base URI 设置为: ipfs://{}/",
        metadata_folder_cid
    );
    Ok(())
}

fn main() -> Result<()> {
    // 前置检查
    let status = Command::new("ipfs").arg("id").output()?.status;
    if !status.success() {
        eprintln!("❌ 连接 IPFS 节点失败。");
        eprintln!("请确保你的 IPFS 节点正在运行 (命令: ipfs daemon)。");
        return Err(anyhow!("IPFS daemon not running"));
    }
    println!("✅ 成功连接到 IPFS 节点");

    let single_image_path = PathBuf::from("../assets/image/IMG_20210626_180340.jpg");
    let batch_images_path = PathBuf::from("../assets/batch_images");
    fs::create_dir_all(&batch_images_path)?;

    // --- 在这里选择要运行的工作流 ---
    process_single_nft(&single_image_path)?;
    process_batch_collection(&batch_images_path)?;

    println!("\n======================================================================");
    println!("✅ 本地准备工作已完成！");
    println!("下一步是发布到专业的 Pinning 服务 (如 Pinata):");
    println!("1. 登录 Pinata。");
    println!("2. 上传您本地 `rust/output/collection_[时间戳]/images` 文件夹。");
    println!("3. 上传您本地 `rust/output/collection_[时间戳]/metadata` 文件夹。");
    println!("4. ⚠️  使用 Pinata 返回的【metadata】文件夹的 CID 来设置您合约的 Base URI。");
    println!("======================================================================");

    Ok(())
}

// --- 辅助函数 ---
fn copy_directory(src: &Path, dst: &Path) -> io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in WalkDir::new(src) {
        let entry = entry?;
        let path = entry.path();
        let relative_path = path.strip_prefix(src).unwrap();
        let dest_path = dst.join(relative_path);
        if path.is_dir() {
            fs::create_dir_all(&dest_path)?;
        } else {
            fs::copy(path, &dest_path)?;
        }
    }
    Ok(())
}

/*
polyglot-ipfs-uploader/rust on  main [!] is 📦 0.1.0 via 🦀 1.88.0 on 🐳 v28.2.2 (orbstack)
➜ cargo build
warning: function `upload_json_str_to_ipfs` is never used
  --> src/main.rs:69:4
   |
69 | fn upload_json_str_to_ipfs(data: &NftMetadata) -> Result<String> {
   |    ^^^^^^^^^^^^^^^^^^^^^^^
   |
   = note: `#[warn(dead_code)]` on by default

warning: function `process_single_nft` is never used
   --> src/main.rs:102:4
    |
102 | fn process_single_nft(image_path: &Path) -> Result<()> {
    |    ^^^^^^^^^^^^^^^^^^

warning: `rust` (bin "rust") generated 2 warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.14s

polyglot-ipfs-uploader/rust on  main [!] is 📦 0.1.0 via 🦀 1.88.0 on 🐳 v28.2.2 (orbstack)
➜ cargo run
warning: function `upload_json_str_to_ipfs` is never used
  --> src/main.rs:69:4
   |
69 | fn upload_json_str_to_ipfs(data: &NftMetadata) -> Result<String> {
   |    ^^^^^^^^^^^^^^^^^^^^^^^
   |
   = note: `#[warn(dead_code)]` on by default

warning: function `process_single_nft` is never used
   --> src/main.rs:102:4
    |
102 | fn process_single_nft(image_path: &Path) -> Result<()> {
    |    ^^^^^^^^^^^^^^^^^^

warning: `rust` (bin "rust") generated 2 warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.19s
     Running `target/debug/rust`
✅ 成功连接到 IPFS 节点

==============================================
🚀 开始处理批量 NFT 集合...
   - 文件后缀模式: .json
==============================================

--- 正在执行上传命令: ipfs add -r -Q --cid-version 1 ../assets/batch_images ---
✅ 上传成功!
   - 名称: batch_images
   - CID: bafybeia22ed2lhakgwu76ojojhuavlxkccpclciy6hgqsmn6o7ur7cw44e

🖼️  图片文件夹 CID 已获取: bafybeia22ed2lhakgwu76ojojhuavlxkccpclciy6hgqsmn6o7ur7cw44e

💾 所有图片已复制到: "output/collection_20250728_092506/images"

--- 正在为每张图片生成元数据 JSON 文件 ---
✅ 成功生成 3 个元数据文件到: "output/collection_20250728_092506/metadata"

--- 正在执行上传命令: ipfs add -r -Q --cid-version 1 output/collection_20250728_092506/metadata ---
✅ 上传成功!
   - 名称: metadata
   - CID: bafybeiguvcmspmkhyheyh5c7wmixuiiysjpcrw4hjvvydmfhqmwsopvjk4

📄 元数据文件夹 CID 已获取: bafybeiguvcmspmkhyheyh5c7wmixuiiysjpcrw4hjvvydmfhqmwsopvjk4

--- ✨ 批量流程完成 ✨ ---
下一步，您可以在合约中将 Base URI 设置为: ipfs://bafybeiguvcmspmkhyheyh5c7wmixuiiysjpcrw4hjvvydmfhqmwsopvjk4/

======================================================================
✅ 本地准备工作已完成！
下一步是发布到专业的 Pinning 服务 (如 Pinata):
1. 登录 Pinata。
2. 上传您本地 `rust/output/collection_[时间戳]/images` 文件夹。
3. 上传您本地 `rust/output/collection_[时间戳]/metadata` 文件夹。
4. ⚠️  使用 Pinata 返回的【metadata】文件夹的 CID 来设置您合约的 Base URI。
======================================================================

polyglot-ipfs-uploader/rust on  main [!?] is 📦 0.1.0 via 🦀 1.88.0 on 🐳 v28.2.2 (orbstack)
➜ cargo run
   Compiling rust v0.1.0 (/Users/qiaopengjun/Code/Solidity/YuanqiGenesis/polyglot-ipfs-uploader/rust)
warning: function `process_batch_collection` is never used
   --> src/main.rs:158:4
    |
158 | fn process_batch_collection(images_input_dir: &Path) -> Result<()> {
    |    ^^^^^^^^^^^^^^^^^^^^^^^^
    |
    = note: `#[warn(dead_code)]` on by default

warning: function `copy_directory` is never used
   --> src/main.rs:264:4
    |
264 | fn copy_directory(src: &Path, dst: &Path) -> io::Result<()> {
    |    ^^^^^^^^^^^^^^

warning: `rust` (bin "rust") generated 2 warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.04s
     Running `target/debug/rust`
✅ 成功连接到 IPFS 节点

==============================================
🚀 开始处理单个 NFT...
   - 文件后缀模式: .json
==============================================

--- 正在执行上传命令: ipfs add -r -Q --cid-version 1 ../assets/image/IMG_20210626_180340.jpg ---
✅ 上传成功!
   - 名称: IMG_20210626_180340.jpg
   - CID: bafybeifwvvo7qacd5ksephyxbqkqjih2dmm2ffgqa6u732b2evw5iijppi

🖼️  图片 CID 已获取: bafybeifwvvo7qacd5ksephyxbqkqjih2dmm2ffgqa6u732b2evw5iijppi

--- 正在上传 JSON 对象 ---
✅ JSON 元数据上传成功!
   - CID: bafkreihhpbkssgrr22r3f3rhrb4hntmbdzfm3ubaun2cfw4p5vyhcgivbi

💾 图片和元数据已在本地打包保存至: "output/IMG_20210626_180340"

--- ✨ 单件流程完成 ✨ ---
下一步，您可以在 mint 函数中使用这个元数据 URI: ipfs://bafkreihhpbkssgrr22r3f3rhrb4hntmbdzfm3ubaun2cfw4p5vyhcgivbi

======================================================================
✅ 本地准备工作已完成！
下一步是发布到专业的 Pinning 服务 (如 Pinata):
1. 登录 Pinata。
2. 上传您本地 `rust/output/collection_[时间戳]/images` 文件夹。
3. 上传您本地 `rust/output/collection_[时间戳]/metadata` 文件夹。
4. ⚠️  使用 Pinata 返回的【metadata】文件夹的 CID 来设置您合约的 Base URI。
======================================================================

polyglot-ipfs-uploader/rust on  main [!?] is 📦 0.1.0 via 🦀 1.88.0 on 🐳 v28.2.2 (orbstack)
➜ cargo run
   Compiling rust v0.1.0 (/Users/qiaopengjun/Code/Solidity/YuanqiGenesis/polyglot-ipfs-uploader/rust)
warning: function `process_batch_collection` is never used
   --> src/main.rs:158:4
    |
158 | fn process_batch_collection(images_input_dir: &Path) -> Result<()> {
    |    ^^^^^^^^^^^^^^^^^^^^^^^^
    |
    = note: `#[warn(dead_code)]` on by default

warning: function `copy_directory` is never used
   --> src/main.rs:264:4
    |
264 | fn copy_directory(src: &Path, dst: &Path) -> io::Result<()> {
    |    ^^^^^^^^^^^^^^

warning: `rust` (bin "rust") generated 2 warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.57s
     Running `target/debug/rust`
✅ 成功连接到 IPFS 节点

==============================================
🚀 开始处理单个 NFT...
   - 文件后缀模式: 无
==============================================

--- 正在执行上传命令: ipfs add -r -Q --cid-version 1 ../assets/image/IMG_20210626_180340.jpg ---
✅ 上传成功!
   - 名称: IMG_20210626_180340.jpg
   - CID: bafybeifwvvo7qacd5ksephyxbqkqjih2dmm2ffgqa6u732b2evw5iijppi

🖼️  图片 CID 已获取: bafybeifwvvo7qacd5ksephyxbqkqjih2dmm2ffgqa6u732b2evw5iijppi

--- 正在上传 JSON 对象 ---
✅ JSON 元数据上传成功!
   - CID: bafkreihhpbkssgrr22r3f3rhrb4hntmbdzfm3ubaun2cfw4p5vyhcgivbi

💾 图片和元数据已在本地打包保存至: "output/IMG_20210626_180340"

--- ✨ 单件流程完成 ✨ ---
下一步，您可以在 mint 函数中使用这个元数据 URI: ipfs://bafkreihhpbkssgrr22r3f3rhrb4hntmbdzfm3ubaun2cfw4p5vyhcgivbi

======================================================================
✅ 本地准备工作已完成！
下一步是发布到专业的 Pinning 服务 (如 Pinata):
1. 登录 Pinata。
2. 上传您本地 `rust/output/collection_[时间戳]/images` 文件夹。
3. 上传您本地 `rust/output/collection_[时间戳]/metadata` 文件夹。
4. ⚠️  使用 Pinata 返回的【metadata】文件夹的 CID 来设置您合约的 Base URI。
======================================================================

polyglot-ipfs-uploader/rust on  main [!?] is 📦 0.1.0 via 🦀 1.88.0 on 🐳 v28.2.2 (orbstack)
➜ cargo run
   Compiling rust v0.1.0 (/Users/qiaopengjun/Code/Solidity/YuanqiGenesis/polyglot-ipfs-uploader/rust)
warning: function `upload_json_str_to_ipfs` is never used
  --> src/main.rs:69:4
   |
69 | fn upload_json_str_to_ipfs(data: &NftMetadata) -> Result<String> {
   |    ^^^^^^^^^^^^^^^^^^^^^^^
   |
   = note: `#[warn(dead_code)]` on by default

warning: function `process_single_nft` is never used
   --> src/main.rs:102:4
    |
102 | fn process_single_nft(image_path: &Path) -> Result<()> {
    |    ^^^^^^^^^^^^^^^^^^

warning: `rust` (bin "rust") generated 2 warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.67s
     Running `target/debug/rust`
✅ 成功连接到 IPFS 节点

==============================================
🚀 开始处理批量 NFT 集合...
   - 文件后缀模式: 无
==============================================

--- 正在执行上传命令: ipfs add -r -Q --cid-version 1 ../assets/batch_images ---
✅ 上传成功!
   - 名称: batch_images
   - CID: bafybeia22ed2lhakgwu76ojojhuavlxkccpclciy6hgqsmn6o7ur7cw44e

🖼️  图片文件夹 CID 已获取: bafybeia22ed2lhakgwu76ojojhuavlxkccpclciy6hgqsmn6o7ur7cw44e

💾 所有图片已复制到: "output/collection_20250728_092723/images"

--- 正在为每张图片生成元数据 JSON 文件 ---
✅ 成功生成 3 个元数据文件到: "output/collection_20250728_092723/metadata"

--- 正在执行上传命令: ipfs add -r -Q --cid-version 1 output/collection_20250728_092723/metadata ---
✅ 上传成功!
   - 名称: metadata
   - CID: bafybeihnyl6zp4q4xusvpt77nzl7ljg3ec6xhbgaflzrn6bzrpo7nivgzq

📄 元数据文件夹 CID 已获取: bafybeihnyl6zp4q4xusvpt77nzl7ljg3ec6xhbgaflzrn6bzrpo7nivgzq

--- ✨ 批量流程完成 ✨ ---
下一步，您可以在合约中将 Base URI 设置为: ipfs://bafybeihnyl6zp4q4xusvpt77nzl7ljg3ec6xhbgaflzrn6bzrpo7nivgzq/

======================================================================
✅ 本地准备工作已完成！
下一步是发布到专业的 Pinning 服务 (如 Pinata):
1. 登录 Pinata。
2. 上传您本地 `rust/output/collection_[时间戳]/images` 文件夹。
3. 上传您本地 `rust/output/collection_[时间戳]/metadata` 文件夹。
4. ⚠️  使用 Pinata 返回的【metadata】文件夹的 CID 来设置您合约的 Base URI。
======================================================================
*/
