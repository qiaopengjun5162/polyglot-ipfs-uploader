use anyhow::Result;
use ipfs_api_backend_hyper::{IpfsApi, IpfsClient};
use serde_json::json;
use std::fs::File;
use std::io::Read;

const IPFS_API_URL: &str = "http://localhost:5001";

async fn upload_file_to_ipfs(client: &IpfsClient, file_path: &str) -> Result<()> {
    println!("\n--- 正在上传文件: {} ---", file_path);
    let mut file = File::open(file_path)?;
    let mut data = Vec::new();
    file.read_to_end(&mut data)?;
    match client.add(data.as_slice()).await {
        Ok(res) => {
            println!("✅ 文件上传成功!");
            println!("   - CID: {}", res.hash);
        }
        Err(e) => eprintln!("❌ 上传文件失败: {}", e),
    }
    Ok(())
}

async fn upload_json_to_ipfs(client: &IpfsClient, data: serde_json::Value) -> Result<()> {
    println!("\n--- 正在上传 JSON 对象 ---");
    let json_string = data.to_string();
    match client.add(json_string.as_bytes()).await {
        Ok(res) => {
            println!("✅ JSON 上传成功!");
            println!("   - CID: {}", res.hash);
        }
        Err(e) => eprintln!("❌ 上传 JSON 失败: {}", e),
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let client = IpfsClient::from_multiaddr_str(IPFS_API_URL).unwrap();
    if client.version().await.is_err() {
        eprintln!("❌ 连接 IPFS 节点失败。请确保 Kubo 节点正在运行。");
        return Ok(());
    }
    println!("✅ 成功连接到 IPFS 节点");

    let dummy_file_path = "temp_upload_file_rust.txt";
    std::fs::write(dummy_file_path, "你好，IPFS！这是来自 Rust 的问候。")?;
    upload_file_to_ipfs(&client, dummy_file_path).await?;
    std::fs::remove_file(dummy_file_path)?;

    let my_nft_metadata = json!({
        "name": "我的第一个Rust NFT",
        "description": "这是一个使用 Rust 上传的元数据。",
        "attributes": [{"trait_type": "语言", "value": "Rust"}]
    });
    upload_json_to_ipfs(&client, my_nft_metadata).await?;

    Ok(())
}
