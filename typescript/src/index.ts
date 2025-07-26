import { create } from "kubo-rpc-client";
import * as fs from "fs";
import { Buffer } from "buffer";
import * as path from "path";
import { fileURLToPath } from "url"; // ✅ 导入 url 模块的辅助函数

// ✅ 定义更详细的元数据接口，以匹配您的示例
interface Attribute {
  trait_type: string;
  value: string | number;
  display_type?: "number";
}

interface NftMetadata {
  name: string;
  description: string;
  image: string; // 将会是 ipfs://<Image_CID>
  external_url?: string;
  attributes: Attribute[];
}

// --- 配置 ---
const ipfs = create({ url: "http://localhost:5001/api/v0" });

// ✅ 新增：在 ESM 模块中获取当前目录路径的正确方法
const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

/**
 * 上传一个本地文件到 IPFS。
 * @param filePath 文件的本地路径
 * @returns 上传结果或 undefined
 */
export async function uploadFileToIPFS(filePath: string) {
  try {
    console.log(`\n--- 正在上传文件: ${filePath} ---`);
    const file: Buffer = fs.readFileSync(filePath);

    const result = await ipfs.add({
      path: path.basename(filePath), // 只使用文件名
      content: file,
    });

    console.log("✅ 文件上传成功!");
    console.log(`   - 文件名: ${result.path}`);
    console.log(`   - CID: ${result.cid.toString()}`);
    console.log(`   - 大小: ${result.size} 字节`);
    return result;
  } catch (err) {
    console.error("❌ 上传文件失败:", err);
  }
}

/**
 * 将一个 JSON 对象上传到 IPFS。
 * @param json 要上传的 JSON 对象
 * @returns 上传结果或 undefined
 */
export async function uploadJSONToIPFS(json: NftMetadata) {
  try {
    console.log("\n--- 正在上传 JSON 对象 ---");
    const result = await ipfs.add(JSON.stringify(json));

    console.log("✅ JSON 元数据上传成功!");
    console.log(`   - CID: ${result.cid.toString()}`);
    console.log(`   - 大小: ${result.size} 字节`);
    return result;
  } catch (err) {
    console.error("❌ 上传 JSON 失败:", err);
  }
}

// 主执行函数
async function main() {
  try {
    // 检查 IPFS 节点连接
    const version = await ipfs.version();
    console.log(`✅ 成功连接到 IPFS 节点 (版本: ${version.version})`);

    // --- 步骤 1: 上传图片文件 ---
    // ✅ 修复：使用新的 __dirname 变量来构建正确的路径
    const imagePath = path.join(
      __dirname,
      "..",
      "..",
      "assets",
      "image",
      "IMG_20210626_180340.jpg"
    );

    if (!fs.existsSync(imagePath)) {
      console.error(`❌ 图片文件未找到: ${imagePath}`);
      return;
    }

    const imageUploadResult = await uploadFileToIPFS(imagePath);
    if (!imageUploadResult) {
      console.error("图片上传失败，脚本终止。");
      return;
    }
    const imageCid = imageUploadResult.cid.toString();
    console.log(`\n🖼️ 图片 CID 已获取: ${imageCid}`);

    // --- 步骤 2: 构建并上传元数据 JSON ---
    console.log("\n--- 正在构建元数据 JSON ---");

    // ✅ 使用获取到的图片 CID 构建元数据
    // 注意：在链上元数据中，标准做法是使用 "ipfs://" 协议前缀
    const metadata: NftMetadata = {
      name: "MyERC721Token",
      description: "这是一个使用 TypeScript 脚本动态生成的元数据。",
      image: `ipfs://${imageCid}`, // <-- 关键步骤！
      external_url: "https://testnets.opensea.io/zh-CN/account/collected",
      attributes: [
        { trait_type: "Background", value: "Blue" },
        { trait_type: "Eyes", value: "Green" },
        { trait_type: "Mouth", value: "Smile" },
        { trait_type: "Clothing", value: "T-shirt" },
        { trait_type: "Accessories", value: "Hat" },
        { display_type: "number", trait_type: "Generation", value: 1 },
      ],
    };

    const metadataUploadResult = await uploadJSONToIPFS(metadata);
    if (!metadataUploadResult) {
      console.error("元数据上传失败，脚本终止。");
      return;
    }
    const metadataCid = metadataUploadResult.cid.toString();
    console.log(`\n📄 元数据 CID 已获取: ${metadataCid}`);

    console.log("\n--- ✨ 流程完成 ✨ ---");
    console.log(
      `下一步，您可以在 mint 函数中使用这个元数据 URI: ipfs://${metadataCid}`
    );
  } catch (error) {
    console.error(`\n❌ 脚本执行过程中发生错误:`, error);
    console.error("\n--- 故障排查 ---");
    console.error("1. 请确保你的 IPFS 节点正在运行 (命令: ipfs daemon)。");
    console.error("2. 检查文件路径是否正确，以及脚本是否有读取权限。");
  }
}

main();

/*
YuanqiGenesis/polyglot-ipfs-uploader/typescript is 📦 1.0.0 via ⬢ v23.11.0 via 🍞 v1.2.17 on 🐳 v28.2.2 (orbstack) 
➜ bun start
$ ts-node src/index.ts
(node:52662) ExperimentalWarning: Type Stripping is an experimental feature and might change at any time
(Use `node --trace-warnings ...` to show where the warning was created)
✅ 成功连接到 IPFS 节点 (版本: 0.36.0)

--- 正在上传文件: /Users/qiaopengjun/Code/Solidity/YuanqiGenesis/polyglot-ipfs-uploader/assets/image/IMG_20210626_180340.jpg ---
✅ 文件上传成功!
   - 文件名: IMG_20210626_180340.jpg
   - CID: QmXgwL18mcPFTJvbLmGXet4rpGwU9oNH9bDRGYuV1vNtQs
   - 大小: 4051551 字节

🖼️ 图片 CID 已获取: QmXgwL18mcPFTJvbLmGXet4rpGwU9oNH9bDRGYuV1vNtQs

--- 正在构建元数据 JSON ---

--- 正在上传 JSON 对象 ---
✅ JSON 元数据上传成功!
   - CID: QmVHfPYQVnoaUa1Z6MuEy4rmUfWadR6yAkU44fe18ztrL1
   - 大小: 532 字节

📄 元数据 CID 已获取: QmVHfPYQVnoaUa1Z6MuEy4rmUfWadR6yAkU44fe18ztrL1

--- ✨ 流程完成 ✨ ---
下一步，您可以在 mint 函数中使用这个元数据 URI: ipfs://QmVHfPYQVnoaUa1Z6MuEy4rmUfWadR6yAkU44fe18ztrL1

Mint ERC721 Token
https://hoodi.etherscan.io/tx/0x51424695af291d3a3b7fc54b1a5b1308ea39a94f564e16de3bafe3bff565423a
https://hoodi.etherscan.io/tx/0x0b0771edee0cc9f702433ef8f6b044d6aa6740ad91961cf745ccf7b235426e3c
*/
