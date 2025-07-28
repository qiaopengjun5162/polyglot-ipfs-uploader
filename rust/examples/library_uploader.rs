// examples/library_uploader.rs

// 从我们自己的库中导入共享的结构体和函数
use rust::{Attribute, NftMetadata, copy_directory};

use anyhow::{Result, anyhow};
use chrono::Utc;
use ipfs_api_backend_hyper::{IpfsApi, IpfsClient, TryFromUri};
use std::fs::{self, File};
use std::io::{Cursor, Write};
use std::path::{Path, PathBuf};

const USE_JSON_SUFFIX: bool = false;
const IPFS_API_URL: &str = "http://localhost:5001";

// --- 核心上传函数 ---

// 上传单个文件
async fn upload_file_to_ipfs(client: &IpfsClient, target_path: &Path) -> Result<String> {
    println!("\n--- 正在上传(库): {:?} ---", target_path);
    if !target_path.exists() {
        return Err(anyhow!("路径不存在: {:?}", target_path));
    }
    let data = fs::read(target_path)?;
    let cursor = Cursor::new(data);
    let res = client.add(cursor).await?;
    let cid = res.hash;
    println!("✅ 上传成功! CID: {}", cid);
    Ok(cid)
}

// 上传整个文件夹
async fn upload_directory_to_ipfs(client: &IpfsClient, dir_path: &Path) -> Result<String> {
    println!("\n--- 正在上传文件夹(库): {:?} ---", dir_path);
    // add_path 返回一个 Vec，最后一个元素是根目录的信息
    let responses = client.add_path(dir_path).await?;
    if let Some(root_res) = responses.last() {
        let cid = root_res.hash.clone();
        println!("✅ 文件夹上传成功! CID: {}", cid);
        Ok(cid)
    } else {
        Err(anyhow!("文件夹上传失败"))
    }
}

// 上传 JSON 数据
async fn upload_json_str_to_ipfs(client: &IpfsClient, data: &NftMetadata) -> Result<String> {
    let json_string = serde_json::to_string(data)?;
    let cursor = Cursor::new(json_string.into_bytes());
    let res = client.add(cursor).await?;
    let cid = res.hash;
    println!("\n✅ JSON 元数据上传成功! CID: {}", cid);
    Ok(cid)
}

// --- 工作流一：处理单个 NFT ---
async fn process_single_nft(client: &IpfsClient, image_path: &Path) -> Result<()> {
    println!("\n==============================================");
    println!("🚀 开始处理单个 NFT (官方库方式)...");
    println!("==============================================");

    let image_cid = upload_file_to_ipfs(client, image_path).await?;
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

    let metadata_cid = upload_json_str_to_ipfs(client, &metadata).await?;

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

// --- 工作流二：处理批量 NFT 集合 ---
async fn process_batch_collection(client: &IpfsClient, images_input_dir: &Path) -> Result<()> {
    println!("\n==============================================");
    println!("🚀 开始处理批量 NFT 集合 (官方库方式)...");
    println!("==============================================");
    let images_folder_cid = upload_directory_to_ipfs(client, images_input_dir).await?;
    let timestamp = Utc::now().format("%Y%m%d_%H%M%S").to_string();
    let collection_output_dir =
        PathBuf::from("output").join(format!("collection_lib_{}", timestamp));
    let images_output_dir = collection_output_dir.join("images");
    let metadata_output_dir = collection_output_dir.join("metadata");
    copy_directory(images_input_dir, &images_output_dir)?;
    println!("\n💾 所有图片已复制到: {:?}", images_output_dir);
    fs::create_dir_all(&metadata_output_dir)?;
    let mut image_files: Vec<PathBuf> = fs::read_dir(images_input_dir)?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.is_file())
        .collect();
    image_files.sort();
    for image_file in &image_files {
        let token_id_str = image_file
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| anyhow!("无效文件名"))?;
        let token_id: u64 = token_id_str.parse()?;
        let image_filename = image_file
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or_else(|| anyhow!("无效文件名"))?;
        let metadata = NftMetadata {
            name: format!("MetaCore #{}", token_id),
            description: "MetaCore 集合中的一个独特成员。".to_string(),
            image: format!("ipfs://{}/{}", images_folder_cid, image_filename),
            attributes: vec![Attribute {
                trait_type: "ID".to_string(),
                value: token_id.into(),
            }],
        };
        let file_name = if USE_JSON_SUFFIX {
            format!("{}.json", token_id_str)
        } else {
            token_id_str.to_string()
        };
        let mut file = File::create(metadata_output_dir.join(file_name))?;
        file.write_all(serde_json::to_string_pretty(&metadata)?.as_bytes())?;
    }
    println!(
        "✅ 成功生成 {} 个元数据文件到: {:?}",
        image_files.len(),
        metadata_output_dir
    );
    let metadata_folder_cid = upload_directory_to_ipfs(client, &metadata_output_dir).await?;
    println!("\n📄 元数据文件夹 CID 已获取: {}", metadata_folder_cid);
    println!("\n--- ✨ 批量流程完成 ✨ ---");
    println!(
        "下一步，您可以在合约中将 Base URI 设置为: ipfs://{}/",
        metadata_folder_cid
    );
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let client = IpfsClient::from_multiaddr_str(IPFS_API_URL)
        .map_err(|e| anyhow!("创建 IPFS 客户端失败: {}", e))?;

    if client.version().await.is_err() {
        eprintln!("❌ 连接 IPFS 节点失败。请确保 ipfs daemon 正在运行。");
        return Ok(());
    }
    println!("✅ 成功连接到 IPFS 节点");

    let single_image_path = PathBuf::from("../assets/image/IMG_20210626_180340.jpg");
    let batch_images_path = PathBuf::from("../assets/batch_images");
    fs::create_dir_all(&batch_images_path)?;

    // --- 在这里选择要运行的工作流 ---
    // 首先运行工作流一：处理单个 NFT
    process_single_nft(&client, &single_image_path).await?;
    // 然后运行工作流二：处理批量 NFT 集合
    process_batch_collection(&client, &batch_images_path).await?;

    Ok(())
}

/*
polyglot-ipfs-uploader/rust on  main [!?] is 📦 0.1.0 via 🦀 1.88.0 on 🐳 v28.2.2 (orbstack)
➜ cargo run --example library_uploader
   Compiling rust v0.1.0 (/Users/qiaopengjun/Code/Solidity/YuanqiGenesis/polyglot-ipfs-uploader/rust)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.95s
     Running `target/debug/examples/library_uploader`
✅ 成功连接到 IPFS 节点

==============================================
🚀 开始处理单个 NFT (官方库方式)...
==============================================

--- 正在上传(库): "../assets/image/IMG_20210626_180340.jpg" ---
✅ 上传成功! CID: QmXgwL18mcPFTJvbLmGXet4rpGwU9oNH9bDRGYuV1vNtQs

🖼️  图片 CID 已获取: QmXgwL18mcPFTJvbLmGXet4rpGwU9oNH9bDRGYuV1vNtQs

✅ JSON 元数据上传成功! CID: QmZj3odMubignuppJo93wBiNVDv1U1HbYRCmQcQQ8VS4rd

💾 图片和元数据已在本地打包保存至: "output/IMG_20210626_180340"

--- ✨ 单件流程完成 ✨ ---
下一步，您可以在 mint 函数中使用这个元数据 URI: ipfs://QmZj3odMubignuppJo93wBiNVDv1U1HbYRCmQcQQ8VS4rd

==============================================
🚀 开始处理批量 NFT 集合 (官方库方式)...
==============================================

--- 正在上传文件夹(库): "../assets/batch_images" ---
✅ 文件夹上传成功! CID: QmVKhPv53d3WKZi5if4Tm4sZnYEL9t2n7kD4v7ENMqx8WP

💾 所有图片已复制到: "output/collection_lib_20250728_114023/images"
✅ 成功生成 3 个元数据文件到: "output/collection_lib_20250728_114023/metadata"

--- 正在上传文件夹(库): "output/collection_lib_20250728_114023/metadata" ---
✅ 文件夹上传成功! CID: QmcZtafg6yiSzNaNsjyyh2ttHYbekbBVGubJnANPhHXwQM

📄 元数据文件夹 CID 已获取: QmcZtafg6yiSzNaNsjyyh2ttHYbekbBVGubJnANPhHXwQM

--- ✨ 批量流程完成 ✨ ---
下一步，您可以在合约中将 Base URI 设置为: ipfs://QmcZtafg6yiSzNaNsjyyh2ttHYbekbBVGubJnANPhHXwQM/

polyglot-ipfs-uploader/rust on  main [!?] is 📦 0.1.0 via 🦀 1.88.0 on 🐳 v28.2.2 (orbstack) took 4.1s
➜ cargo run --example library_uploader
   Compiling rust v0.1.0 (/Users/qiaopengjun/Code/Solidity/YuanqiGenesis/polyglot-ipfs-uploader/rust)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.25s
     Running `target/debug/examples/library_uploader`
✅ 成功连接到 IPFS 节点

==============================================
🚀 开始处理单个 NFT (官方库方式)...
==============================================

--- 正在上传(库): "../assets/image/IMG_20210626_180340.jpg" ---
✅ 上传成功! CID: QmXgwL18mcPFTJvbLmGXet4rpGwU9oNH9bDRGYuV1vNtQs

🖼️  图片 CID 已获取: QmXgwL18mcPFTJvbLmGXet4rpGwU9oNH9bDRGYuV1vNtQs

✅ JSON 元数据上传成功! CID: QmZj3odMubignuppJo93wBiNVDv1U1HbYRCmQcQQ8VS4rd

💾 图片和元数据已在本地打包保存至: "output/IMG_20210626_180340"

--- ✨ 单件流程完成 ✨ ---
下一步，您可以在 mint 函数中使用这个元数据 URI: ipfs://QmZj3odMubignuppJo93wBiNVDv1U1HbYRCmQcQQ8VS4rd

==============================================
🚀 开始处理批量 NFT 集合 (官方库方式)...
==============================================

--- 正在上传文件夹(库): "../assets/batch_images" ---
✅ 文件夹上传成功! CID: QmVKhPv53d3WKZi5if4Tm4sZnYEL9t2n7kD4v7ENMqx8WP

💾 所有图片已复制到: "output/collection_lib_20250728_114130/images"
✅ 成功生成 3 个元数据文件到: "output/collection_lib_20250728_114130/metadata"

--- 正在上传文件夹(库): "output/collection_lib_20250728_114130/metadata" ---
✅ 文件夹上传成功! CID: QmYcgHTuFBkwv3HRyxmjFmUBPZSwtt3LzFV4kZuCxX5Ti5

📄 元数据文件夹 CID 已获取: QmYcgHTuFBkwv3HRyxmjFmUBPZSwtt3LzFV4kZuCxX5Ti5

--- ✨ 批量流程完成 ✨ ---
下一步，您可以在合约中将 Base URI 设置为: ipfs://QmYcgHTuFBkwv3HRyxmjFmUBPZSwtt3LzFV4kZuCxX5Ti5/

*/
