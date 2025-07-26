# main.py

import json
import subprocess
import shlex
import shutil
from pathlib import Path
from datetime import datetime
from typing import Any

# ✅ 配置开关
# 设置为 True  -> 生成 1.json, 2.json... (用于需要后缀的合约)
# 设置为 False -> 生成 1, 2... (用于标准 ERC721A 合约)
USE_JSON_SUFFIX = False

################################################################
# 核心上传函数 (使用 subprocess)
################################################################


def upload_to_ipfs(target_path: Path) -> str | None:
    """
    上传一个文件或整个文件夹到 IPFS CLI。
    """
    if not target_path.exists():
        print(f"❌ 路径不存在: {target_path}")
        return None

    safe_path = shlex.quote(str(target_path))
    command = f"ipfs add -r -Q --cid-version 1 {safe_path}"

    try:
        print(f"\n--- 正在执行上传命令: {command} ---")
        result = subprocess.run(
            command, shell=True, check=True, capture_output=True, text=True
        )
        cid = result.stdout.strip()
        print("✅ 上传成功!")
        print(f"   - 名称: {target_path.name}")
        print(f"   - CID: {cid}")
        return cid
    except subprocess.CalledProcessError as e:
        print(f"❌ 上传失败 (命令返回非零退出码): {e}")
        print(f"   - 错误信息: {e.stderr.strip()}")
        return None
    except Exception as e:
        print(f"❌ 执行上传命令时发生未知错误: {e}")
        return None


def upload_json_str_to_ipfs(
    data: dict[str, Any],
) -> str | None:
    """
    将一个 Python 字典 (JSON 对象) 作为字符串直接上传到 IPFS。
    """
    try:
        print("\n--- 正在上传 JSON 对象 ---")
        json_string = json.dumps(data)
        command = "ipfs add -Q --cid-version 1"
        result = subprocess.run(
            command,
            shell=True,
            check=True,
            input=json_string,
            capture_output=True,
            text=True,
        )
        cid = result.stdout.strip()
        print("✅ JSON 元数据上传成功!")
        print(f"   - CID: {cid}")
        return cid
    except subprocess.CalledProcessError as e:
        print(f"❌ 上传 JSON 失败: {e}")
        print(f"   - 错误信息: {e.stderr.strip()}")
        return None
    except Exception as e:
        print(f"❌ 执行 JSON 上传时发生未知错误: {e}")
        return None


################################################################
# 工作流一：处理单个 NFT
################################################################


def process_single_nft(image_path: Path):
    """
    处理单个 NFT 的流程，并在本地创建一个包含图片和元数据的独立文件夹。
    """
    print("\n==============================================")
    print("🚀 开始处理单个 NFT...")
    print(f"   - 文件后缀模式: {'.json' if USE_JSON_SUFFIX else '无'}")
    print("==============================================")

    image_cid = upload_to_ipfs(image_path)
    if not image_cid:
        return

    print(f"\n🖼️  图片 CID 已获取: {image_cid}")

    metadata = {
        "name": image_path.stem,
        "description": f"这是一个为图片 {image_path.name} 动态生成的元数据。",
        "image": f"ipfs://{image_cid}",
        "attributes": [{"trait_type": "类型", "value": "单件艺术品"}],
    }

    metadata_cid = upload_json_str_to_ipfs(metadata)
    if not metadata_cid:
        return

    # ✅ 创建独立的输出文件夹，并将图片和 JSON 都保存在里面
    output_dir = Path(__file__).parent / "output" / image_path.stem
    output_dir.mkdir(parents=True, exist_ok=True)

    # 复制图片
    _ = shutil.copy(image_path, output_dir / image_path.name)

    # 根据配置开关决定文件名
    file_name = f"{image_path.stem}.json" if USE_JSON_SUFFIX else image_path.stem
    output_file_path = output_dir / file_name
    with open(output_file_path, "w", encoding="utf-8") as f:
        json.dump(metadata, f, indent=4, ensure_ascii=False)

    print(f"\n💾 图片和元数据已在本地打包保存至: {output_dir}")
    print("\n--- ✨ 单件流程完成 ✨ ---")
    print(f"下一步，您可以在 mint 函数中使用这个元数据 URI: ipfs://{metadata_cid}")


################################################################
# 工作流二：处理批量 NFT 集合
################################################################


def process_batch_collection(images_input_dir: Path):
    """
    处理整个 NFT 集合的流程，并在本地创建一个包含 images 和 metadata 两个子文件夹的集合文件夹。
    """
    print("\n==============================================")
    print("🚀 开始处理批量 NFT 集合...")
    print(f"   - 文件后缀模式: {'.json' if USE_JSON_SUFFIX else '无'}")
    print("==============================================")

    images_folder_cid = upload_to_ipfs(images_input_dir)
    if not images_folder_cid:
        return

    print(f"\n🖼️  图片文件夹 CID 已获取: {images_folder_cid}")

    # ✅ 为本次批量处理创建一个带时间戳的唯一父文件夹
    timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
    collection_output_dir = Path(__file__).parent / "output" / f"collection_{timestamp}"
    images_output_dir = collection_output_dir / "images"
    metadata_output_dir = collection_output_dir / "metadata"

    # 复制整个图片文件夹到输出目录
    _ = shutil.copytree(images_input_dir, images_output_dir)
    print(f"\n💾 所有图片已复制到: {images_output_dir}")

    print("\n--- 正在为每张图片生成元数据 JSON 文件 ---")
    metadata_output_dir.mkdir(parents=True, exist_ok=True)

    # 清理旧文件，防止混淆
    for old_file in metadata_output_dir.glob("*"):
        if old_file.is_file():
            old_file.unlink()

    image_files = sorted(
        [
            f
            for f in images_input_dir.iterdir()
            if f.is_file()
            and f.name.lower().endswith((".png", ".jpg", ".jpeg", ".gif"))
        ]
    )

    for image_file in image_files:
        token_id = image_file.stem
        metadata = {
            "name": f"MetaCore #{token_id}",
            "description": "MetaCore 集合中的一个独特成员。",
            "image": f"ipfs://{images_folder_cid}/{image_file.name}",
            "attributes": [{"trait_type": "ID", "value": int(token_id)}],
        }
        # ✅ 根据配置开关决定文件名
        file_name = f"{token_id}.json" if USE_JSON_SUFFIX else token_id
        with open(metadata_output_dir / file_name, "w", encoding="utf-8") as f:
            json.dump(metadata, f, indent=4, ensure_ascii=False)

    print(f"✅ 成功生成 {len(image_files)} 个元数据文件到: {metadata_output_dir}")

    metadata_folder_cid = upload_to_ipfs(metadata_output_dir)
    if not metadata_folder_cid:
        return

    print(f"\n📄 元数据文件夹 CID 已获取: {metadata_folder_cid}")
    print("\n--- ✨ 批量流程完成 ✨ ---")
    print(f"下一步，您可以在合约中将 Base URI 设置为: ipfs://{metadata_folder_cid}/")


################################################################
# 主程序入口
################################################################

if __name__ == "__main__":
    current_dir = Path(__file__).parent
    single_image_path = (
        current_dir.parent / "assets" / "image" / "IMG_20210626_180340.jpg"
    )
    batch_images_path = current_dir.parent / "assets" / "batch_images"
    batch_images_path.mkdir(parents=True, exist_ok=True)

    try:
        _ = subprocess.run("ipfs id", shell=True, check=True, capture_output=True)
        print("✅ 成功连接到 IPFS 节点")
    except subprocess.CalledProcessError:
        print("❌ 连接 IPFS 节点失败。")
        print("请确保你的 IPFS 节点正在运行 (命令: ipfs daemon)。")
        exit()

    # --- 在这里选择要运行的工作流 ---

    # 运行工作流一：处理单个 NFT
    # process_single_nft(single_image_path)

    # 运行工作流二：处理批量 NFT 集合
    process_batch_collection(batch_images_path)

    # ✅ 生产环境最终发布流程说明
    print("\n======================================================================")
    print("✅ 本地准备工作已完成！")
    print("下一步是发布到专业的 Pinning 服务 (如 Pinata):")
    print("1. 登录 Pinata。")
    print("2. 上传您本地 `python/output/collection_[时间戳]/images` 文件夹。")
    print("3. 上传您本地 `python/output/collection_[时间戳]/metadata` 文件夹。")
    print("4. ⚠️  使用 Pinata 返回的【metadata】文件夹的 CID 来设置您合约的 Base URI。")
    print("======================================================================")


"""
polyglot-ipfs-uploader/python on  main on 🐳 v28.2.2 (orbstack) via uv 3.13.5
➜ python main.py
✅ 成功连接到 IPFS 节点

==============================================
🚀 开始处理批量 NFT 集合...
   - 文件后缀模式: .json
==============================================

--- 正在执行上传命令: ipfs add -r -Q --cid-version 1 /Users/qiaopengjun/Code/Solidity/YuanqiGenesis/polyglot-ipfs-uploader/assets/batch_images ---
✅ 上传成功!
   - 名称: batch_images
   - CID: bafybeia22ed2lhakgwu76ojojhuavlxkccpclciy6hgqsmn6o7ur7cw44e

🖼️  图片文件夹 CID 已获取: bafybeia22ed2lhakgwu76ojojhuavlxkccpclciy6hgqsmn6o7ur7cw44e

💾 所有图片已复制到: /Users/qiaopengjun/Code/Solidity/YuanqiGenesis/polyglot-ipfs-uploader/python/output/collection_20250726_152634/images

--- 正在为每张图片生成元数据 JSON 文件 ---
✅ 成功生成 3 个元数据文件到: /Users/qiaopengjun/Code/Solidity/YuanqiGenesis/polyglot-ipfs-uploader/python/output/collection_20250726_152634/metadata

--- 正在执行上传命令: ipfs add -r -Q --cid-version 1 /Users/qiaopengjun/Code/Solidity/YuanqiGenesis/polyglot-ipfs-uploader/python/output/collection_20250726_152634/metadata ---
✅ 上传成功!
   - 名称: metadata
   - CID: bafybeiczqa75ljidb7esu464fj6a64nfujxcd2mum73t5yaw2llkrzb4zy

📄 元数据文件夹 CID 已获取: bafybeiczqa75ljidb7esu464fj6a64nfujxcd2mum73t5yaw2llkrzb4zy

--- ✨ 批量流程完成 ✨ ---
下一步，您可以在合约中将 Base URI 设置为: ipfs://bafybeiczqa75ljidb7esu464fj6a64nfujxcd2mum73t5yaw2llkrzb4zy/

======================================================================
✅ 本地准备工作已完成！
下一步是发布到专业的 Pinning 服务 (如 Pinata):
1. 登录 Pinata。
2. 上传您本地 `python/output/collection_[时间戳]/images` 文件夹。
3. 上传您本地 `python/output/collection_[时间戳]/metadata` 文件夹。
4. ⚠️  使用 Pinata 返回的【metadata】文件夹的 CID 来设置您合约的 Base URI。
======================================================================



polyglot-ipfs-uploader/python on  main [✘!?] on 🐳 v28.2.2 (orbstack) via uv 3.13.5
➜ python main.py
✅ 成功连接到 IPFS 节点

==============================================
🚀 开始处理单个 NFT...
   - 文件后缀模式: .json
==============================================

--- 正在执行上传命令: ipfs add -r -Q --cid-version 1 /Users/qiaopengjun/Code/Solidity/YuanqiGenesis/polyglot-ipfs-uploader/assets/image/IMG_20210626_180340.jpg ---
✅ 上传成功!
   - 名称: IMG_20210626_180340.jpg
   - CID: bafybeifwvvo7qacd5ksephyxbqkqjih2dmm2ffgqa6u732b2evw5iijppi

🖼️  图片 CID 已获取: bafybeifwvvo7qacd5ksephyxbqkqjih2dmm2ffgqa6u732b2evw5iijppi

--- 正在上传 JSON 对象 ---
✅ JSON 元数据上传成功!
   - CID: bafkreigvggefv56bv6nmmqekyh2hc4iybn5lblinqimajatrvoxbbqcy2e

💾 图片和元数据已在本地打包保存至: /Users/qiaopengjun/Code/Solidity/YuanqiGenesis/polyglot-ipfs-uploader/python/output/IMG_20210626_180340

--- ✨ 单件流程完成 ✨ ---
下一步，您可以在 mint 函数中使用这个元数据 URI: ipfs://bafkreigvggefv56bv6nmmqekyh2hc4iybn5lblinqimajatrvoxbbqcy2e

======================================================================
✅ 本地准备工作已完成！
下一步是发布到专业的 Pinning 服务 (如 Pinata):
1. 登录 Pinata。
2. 上传您本地 `python/output/collection_[时间戳]/images` 文件夹。
3. 上传您本地 `python/output/collection_[时间戳]/metadata` 文件夹。
4. ⚠️  使用 Pinata 返回的【metadata】文件夹的 CID 来设置您合约的 Base URI。
======================================================================

polyglot-ipfs-uploader/python on  main [✘!?] on 🐳 v28.2.2 (orbstack) via uv 3.13.5
➜ python main.py
✅ 成功连接到 IPFS 节点

==============================================
🚀 开始处理单个 NFT...
   - 文件后缀模式: 无
==============================================

--- 正在执行上传命令: ipfs add -r -Q --cid-version 1 /Users/qiaopengjun/Code/Solidity/YuanqiGenesis/polyglot-ipfs-uploader/assets/image/IMG_20210626_180340.jpg ---
✅ 上传成功!
   - 名称: IMG_20210626_180340.jpg
   - CID: bafybeifwvvo7qacd5ksephyxbqkqjih2dmm2ffgqa6u732b2evw5iijppi

🖼️  图片 CID 已获取: bafybeifwvvo7qacd5ksephyxbqkqjih2dmm2ffgqa6u732b2evw5iijppi

--- 正在上传 JSON 对象 ---
✅ JSON 元数据上传成功!
   - CID: bafkreigvggefv56bv6nmmqekyh2hc4iybn5lblinqimajatrvoxbbqcy2e

💾 图片和元数据已在本地打包保存至: /Users/qiaopengjun/Code/Solidity/YuanqiGenesis/polyglot-ipfs-uploader/python/output/IMG_20210626_180340

--- ✨ 单件流程完成 ✨ ---
下一步，您可以在 mint 函数中使用这个元数据 URI: ipfs://bafkreigvggefv56bv6nmmqekyh2hc4iybn5lblinqimajatrvoxbbqcy2e

======================================================================
✅ 本地准备工作已完成！
下一步是发布到专业的 Pinning 服务 (如 Pinata):
1. 登录 Pinata。
2. 上传您本地 `python/output/collection_[时间戳]/images` 文件夹。
3. 上传您本地 `python/output/collection_[时间戳]/metadata` 文件夹。
4. ⚠️  使用 Pinata 返回的【metadata】文件夹的 CID 来设置您合约的 Base URI。
======================================================================


polyglot-ipfs-uploader/python on  main [✘!?] on 🐳 v28.2.2 (orbstack) via uv 3.13.5
➜ python main.py
✅ 成功连接到 IPFS 节点

==============================================
🚀 开始处理批量 NFT 集合...
   - 文件后缀模式: 无
==============================================

--- 正在执行上传命令: ipfs add -r -Q --cid-version 1 /Users/qiaopengjun/Code/Solidity/YuanqiGenesis/polyglot-ipfs-uploader/assets/batch_images ---
✅ 上传成功!
   - 名称: batch_images
   - CID: bafybeia22ed2lhakgwu76ojojhuavlxkccpclciy6hgqsmn6o7ur7cw44e

🖼️  图片文件夹 CID 已获取: bafybeia22ed2lhakgwu76ojojhuavlxkccpclciy6hgqsmn6o7ur7cw44e

💾 所有图片已复制到: /Users/qiaopengjun/Code/Solidity/YuanqiGenesis/polyglot-ipfs-uploader/python/output/collection_20250726_154609/images

--- 正在为每张图片生成元数据 JSON 文件 ---
✅ 成功生成 3 个元数据文件到: /Users/qiaopengjun/Code/Solidity/YuanqiGenesis/polyglot-ipfs-uploader/python/output/collection_20250726_154609/metadata

--- 正在执行上传命令: ipfs add -r -Q --cid-version 1 /Users/qiaopengjun/Code/Solidity/YuanqiGenesis/polyglot-ipfs-uploader/python/output/collection_20250726_154609/metadata ---
✅ 上传成功!
   - 名称: metadata
   - CID: bafybeidcdd6osm2gvnxt3vlp434kmfq673fbkv4xtrrkqkpbkqe6iakvdm

📄 元数据文件夹 CID 已获取: bafybeidcdd6osm2gvnxt3vlp434kmfq673fbkv4xtrrkqkpbkqe6iakvdm

--- ✨ 批量流程完成 ✨ ---
下一步，您可以在合约中将 Base URI 设置为: ipfs://bafybeidcdd6osm2gvnxt3vlp434kmfq673fbkv4xtrrkqkpbkqe6iakvdm/

======================================================================
✅ 本地准备工作已完成！
下一步是发布到专业的 Pinning 服务 (如 Pinata):
1. 登录 Pinata。
2. 上传您本地 `python/output/collection_[时间戳]/images` 文件夹。
3. 上传您本地 `python/output/collection_[时间戳]/metadata` 文件夹。
4. ⚠️  使用 Pinata 返回的【metadata】文件夹的 CID 来设置您合约的 Base URI。
======================================================================
"""
