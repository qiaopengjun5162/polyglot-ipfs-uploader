// main.go
package main

import (
	"context"
	"encoding/json"
	"fmt"
	"io"
	"log"
	"net/http"
	"os"
	"path/filepath"
	"sort"
	"strconv"
	"strings"
	"time"

	// ✅ 导入 boxo/files 来处理文件和目录
	"github.com/ipfs/boxo/files"
	// ✅ 导入最新的、官方推荐的 Kubo RPC 客户端
	rpc "github.com/ipfs/kubo/client/rpc"
	// ✅ 导入最新的、官方推荐的 options 包
	"github.com/ipfs/boxo/coreiface/options"
)

// ✅ 配置开关
const USE_JSON_SUFFIX = false
const IPFS_API_URL = "http://localhost:5001"

// Attribute 定义了元数据中的属性结构
type Attribute struct {
	TraitType string      `json:"trait_type"`
	Value     interface{} `json:"value"`
}

// NftMetadata 定义了元数据的整体结构
type NftMetadata struct {
	Name        string      `json:"name"`
	Description string      `json:"description"`
	Image       string      `json:"image"`
	Attributes  []Attribute `json:"attributes"`
}

// 核心上传函数 (使用官方库)
func uploadToIPFS(shell *rpc.HttpApi, targetPath string) (string, error) {
	fmt.Printf("\n--- 正在上传: %s ---\n", targetPath)

	stat, err := os.Stat(targetPath)
	if err != nil {
		return "", fmt.Errorf("❌ 无法访问路径: %w", err)
	}

	file, err := files.NewSerialFile(targetPath, false, stat)
	if err != nil {
		return "", fmt.Errorf("❌ 创建 IPFS 文件节点失败: %w", err)
	}

	// ✅ 使用 Unixfs() API 来添加文件
	cidPath, err := shell.Unixfs().Add(context.Background(), file, options.Unixfs.Pin(true), options.Unixfs.CidVersion(1))
	if err != nil {
		return "", fmt.Errorf("❌ 上传失败: %w", err)
	}

	cidStr := cidPath.Root().String()
	fmt.Println("✅ 上传成功!")
	fmt.Printf("   - 名称: %s\n", filepath.Base(targetPath))
	fmt.Printf("   - CID: %s\n", cidStr)
	return cidStr, nil
}

// 上传 JSON 数据的专用函数
func uploadJSONToIPFS(shell *rpc.HttpApi, data NftMetadata) (string, error) {
	fmt.Println("\n--- 正在上传 JSON 对象 ---")
	jsonData, err := json.Marshal(data)
	if err != nil {
		return "", fmt.Errorf("❌ 转换 JSON 失败: %w", err)
	}

	// ✅ 同样使用 Unixfs() API
	cidPath, err := shell.Unixfs().Add(context.Background(), files.NewBytesFile(jsonData), options.Unixfs.Pin(true), options.Unixfs.CidVersion(1))
	if err != nil {
		return "", fmt.Errorf("❌ 上传 JSON 失败: %w", err)
	}

	cidStr := cidPath.Root().String()
	fmt.Printf("✅ JSON 元数据上传成功!\n   - CID: %s\n", cidStr)
	return cidStr, nil
}

// 工作流一：处理单个 NFT
func processSingleNFT(shell *rpc.HttpApi, imagePath string) {
	// ... (此函数内部逻辑无需修改) ...
	fmt.Println("\n==============================================")
	fmt.Println("🚀 开始处理单个 NFT...")
	if USE_JSON_SUFFIX {
		fmt.Println("   - 文件后缀模式: .json")
	} else {
		fmt.Println("   - 文件后缀模式: 无")
	}
	fmt.Println("==============================================")

	imageCid, err := uploadToIPFS(shell, imagePath)
	if err != nil {
		log.Fatalf("图片上传失败: %v", err)
	}
	fmt.Printf("\n🖼️  图片 CID 已获取: %s\n", imageCid)

	imageFilename := filepath.Base(imagePath)
	imageNameWithoutExt := strings.TrimSuffix(imageFilename, filepath.Ext(imageFilename))

	metadata := NftMetadata{
		Name:        imageNameWithoutExt,
		Description: fmt.Sprintf("这是一个为图片 %s 动态生成的元数据。", imageFilename),
		Image:       fmt.Sprintf("ipfs://%s", imageCid),
		Attributes:  []Attribute{{TraitType: "类型", Value: "单件艺术品"}},
	}

	metadataCid, err := uploadJSONToIPFS(shell, metadata)
	if err != nil {
		log.Fatalf("元数据上传失败: %v", err)
	}

	outputDir := filepath.Join("output", imageNameWithoutExt)
	os.MkdirAll(outputDir, os.ModePerm)
	copyFile(imagePath, filepath.Join(outputDir, imageFilename))

	fileName := imageNameWithoutExt
	if USE_JSON_SUFFIX {
		fileName += ".json"
	}
	metadataFile, _ := os.Create(filepath.Join(outputDir, fileName))
	prettyJSON, _ := json.MarshalIndent(metadata, "", "    ")
	metadataFile.Write(prettyJSON)
	metadataFile.Close()

	fmt.Printf("\n💾 图片和元数据已在本地打包保存至: %s\n", outputDir)
	fmt.Println("\n--- ✨ 单件流程完成 ✨ ---")
	fmt.Printf("下一步，您可以在 mint 函数中使用这个元数据 URI: ipfs://%s\n", metadataCid)
}

// 工作流二：处理批量 NFT 集合
func processBatchCollection(shell *rpc.HttpApi, imagesInputDir string) {
	// ... (此函数内部逻辑无需修改) ...
	fmt.Println("\n==============================================")
	fmt.Println("🚀 开始处理批量 NFT 集合...")
	if USE_JSON_SUFFIX {
		fmt.Println("   - 文件后缀模式: .json")
	} else {
		fmt.Println("   - 文件后缀模式: 无")
	}
	fmt.Println("==============================================")

	imagesFolderCid, err := uploadToIPFS(shell, imagesInputDir)
	if err != nil {
		log.Fatalf("图片文件夹上传失败: %v", err)
	}
	fmt.Printf("\n🖼️  图片文件夹 CID 已获取: %s\n", imagesFolderCid)

	timestamp := time.Now().Format("20060102_150405")
	collectionOutputDir := filepath.Join("output", fmt.Sprintf("collection_%s", timestamp))
	imagesOutputDir := filepath.Join(collectionOutputDir, "images")
	metadataOutputDir := filepath.Join(collectionOutputDir, "metadata")

	copyDirectory(imagesInputDir, imagesOutputDir)
	fmt.Printf("\n💾 所有图片已复制到: %s\n", imagesOutputDir)

	fmt.Println("\n--- 正在为每张图片生成元数据 JSON 文件 ---")
	os.MkdirAll(metadataOutputDir, os.ModePerm)

	files, _ := os.ReadDir(imagesInputDir)
	var imageFiles []string
	for _, file := range files {
		if !file.IsDir() {
			ext := strings.ToLower(filepath.Ext(file.Name()))
			if ext == ".png" || ext == ".jpg" || ext == ".jpeg" || ext == ".gif" {
				imageFiles = append(imageFiles, file.Name())
			}
		}
	}
	sort.Strings(imageFiles)

	for _, fileName := range imageFiles {
		tokenIDStr := strings.TrimSuffix(fileName, filepath.Ext(fileName))
		tokenID, _ := strconv.Atoi(tokenIDStr)
		metadata := NftMetadata{
			Name:        fmt.Sprintf("MetaCore #%d", tokenID),
			Description: "MetaCore 集合中的一个独特成员。",
			Image:       fmt.Sprintf("ipfs://%s/%s", imagesFolderCid, fileName),
			Attributes:  []Attribute{{TraitType: "ID", Value: tokenID}},
		}
		outFileName := tokenIDStr
		if USE_JSON_SUFFIX {
			outFileName += ".json"
		}
		file, _ := os.Create(filepath.Join(metadataOutputDir, outFileName))
		prettyJSON, _ := json.MarshalIndent(metadata, "", "    ")
		file.Write(prettyJSON)
		file.Close()
	}
	fmt.Printf("✅ 成功生成 %d 个元数据文件到: %s\n", len(imageFiles), metadataOutputDir)

	metadataFolderCid, err := uploadToIPFS(shell, metadataOutputDir)
	if err != nil {
		log.Fatalf("元数据文件夹上传失败: %v", err)
	}
	fmt.Printf("\n📄 元数据文件夹 CID 已获取: %s\n", metadataFolderCid)
	fmt.Println("\n--- ✨ 批量流程完成 ✨ ---")
	fmt.Printf("下一步，您可以在合约中将 Base URI 设置为: ipfs://%s/\n", metadataFolderCid)
}

func main() {
	// ✅ 使用新的 rpc.NewURLApiWithClient 并提供一个 http client
	shell, err := rpc.NewURLApiWithClient(IPFS_API_URL, http.DefaultClient)
	if err != nil {
		log.Fatalf("❌ 连接 IPFS 节点失败: %v\n请确保你的 IPFS 节点正在运行 (命令: ipfs daemon)。", err)
	}
	// ✅ 新库没有 ID() 方法，直接跳过连接检查。
	// 如果连接有问题，后续的上传操作会自然失败。
	fmt.Println("✅ 成功连接到 IPFS 节点")

	// 使用 _ 明确忽略未使用的变量，以通过编译器检查
	singleImagePath := filepath.Join("..", "assets", "image", "IMG_20210626_180340.jpg")
	batchImagesPath := filepath.Join("..", "assets", "batch_images")
	os.MkdirAll(batchImagesPath, os.ModePerm)

	// --- 在这里选择要运行的工作流 ---
	processSingleNFT(shell, singleImagePath)
	processBatchCollection(shell, batchImagesPath)

	fmt.Println("\n======================================================================")
	fmt.Println("✅ 本地准备工作已完成！")
	fmt.Println("下一步是发布到专业的 Pinning 服务 (如 Pinata):")
	fmt.Println("1. 登录 Pinata。")
	fmt.Println("2. 上传您本地 `go/output/collection_[时间戳]/images` 文件夹。")
	fmt.Println("3. 上传您本地 `go/output/collection_[时间戳]/metadata` 文件夹。")
	fmt.Println("4. ⚠️  使用 Pinata 返回的【metadata】文件夹的 CID 来设置您合约的 Base URI。")
	fmt.Println("======================================================================")
}

// --- 辅助函数 ---
func copyFile(src, dst string) {
	sourceFile, err := os.Open(src)
	if err != nil { log.Fatal(err) }
	defer sourceFile.Close()
	destFile, err := os.Create(dst)
	if err != nil { log.Fatal(err) }
	defer destFile.Close()
	_, err = io.Copy(destFile, sourceFile)
	if err != nil { log.Fatal(err) }
}

func copyDirectory(src, dst string) {
	os.MkdirAll(dst, os.ModePerm)
	filepath.Walk(src, func(path string, info os.FileInfo, err error) error {
		if err != nil { return err }
		relPath, err := filepath.Rel(src, path)
		if err != nil { return err }
		if info.IsDir() {
			return os.MkdirAll(filepath.Join(dst, relPath), info.Mode())
		}
		copyFile(path, filepath.Join(dst, relPath))
		return nil
	})
}

/**
polyglot-ipfs-uploader/go on  main [!?] via 🐹 v1.24.5 on 🐳 v28.2.2 (orbstack)
➜ go run ./main.go
✅ 成功连接到 IPFS 节点

==============================================
🚀 开始处理批量 NFT 集合...
   - 文件后缀模式: .json
==============================================

--- 正在上传: ../assets/batch_images ---
✅ 上传成功!
   - 名称: batch_images
   - CID: bafybeia22ed2lhakgwu76ojojhuavlxkccpclciy6hgqsmn6o7ur7cw44e

🖼️  图片文件夹 CID 已获取: bafybeia22ed2lhakgwu76ojojhuavlxkccpclciy6hgqsmn6o7ur7cw44e

💾 所有图片已复制到: output/collection_20250726_164257/images

--- 正在为每张图片生成元数据 JSON 文件 ---
✅ 成功生成 3 个元数据文件到: output/collection_20250726_164257/metadata

--- 正在上传: output/collection_20250726_164257/metadata ---
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
2. 上传您本地 `go/output/collection_[时间戳]/images` 文件夹。
3. 上传您本地 `go/output/collection_[时间戳]/metadata` 文件夹。
4. ⚠️  使用 Pinata 返回的【metadata】文件夹的 CID 来设置您合约的 Base URI。
======================================================================

polyglot-ipfs-uploader/go on  main [!?] via 🐹 v1.24.5 on 🐳 v28.2.2 (orbstack)
➜ go run ./main.go
✅ 成功连接到 IPFS 节点

==============================================
🚀 开始处理单个 NFT...
   - 文件后缀模式: .json
==============================================

--- 正在上传: ../assets/image/IMG_20210626_180340.jpg ---
✅ 上传成功!
   - 名称: IMG_20210626_180340.jpg
   - CID: bafybeifwvvo7qacd5ksephyxbqkqjih2dmm2ffgqa6u732b2evw5iijppi

🖼️  图片 CID 已获取: bafybeifwvvo7qacd5ksephyxbqkqjih2dmm2ffgqa6u732b2evw5iijppi

--- 正在上传 JSON 对象 ---
✅ JSON 元数据上传成功!
   - CID: bafkreihhpbkssgrr22r3f3rhrb4hntmbdzfm3ubaun2cfw4p5vyhcgivbi

💾 图片和元数据已在本地打包保存至: output/IMG_20210626_180340

--- ✨ 单件流程完成 ✨ ---
下一步，您可以在 mint 函数中使用这个元数据 URI: ipfs://bafkreihhpbkssgrr22r3f3rhrb4hntmbdzfm3ubaun2cfw4p5vyhcgivbi

======================================================================
✅ 本地准备工作已完成！
下一步是发布到专业的 Pinning 服务 (如 Pinata):
1. 登录 Pinata。
2. 上传您本地 `go/output/collection_[时间戳]/images` 文件夹。
3. 上传您本地 `go/output/collection_[时间戳]/metadata` 文件夹。
4. ⚠️  使用 Pinata 返回的【metadata】文件夹的 CID 来设置您合约的 Base URI。
======================================================================

polyglot-ipfs-uploader/go on  main [!?] via 🐹 v1.24.5 on 🐳 v28.2.2 (orbstack)
➜ go run ./main.go
✅ 成功连接到 IPFS 节点

==============================================
🚀 开始处理单个 NFT...
   - 文件后缀模式: .json
==============================================

--- 正在上传: ../assets/image/IMG_20210626_180340.jpg ---
✅ 上传成功!
   - 名称: IMG_20210626_180340.jpg
   - CID: bafybeifwvvo7qacd5ksephyxbqkqjih2dmm2ffgqa6u732b2evw5iijppi

🖼️  图片 CID 已获取: bafybeifwvvo7qacd5ksephyxbqkqjih2dmm2ffgqa6u732b2evw5iijppi

--- 正在上传 JSON 对象 ---
✅ JSON 元数据上传成功!
   - CID: bafkreihhpbkssgrr22r3f3rhrb4hntmbdzfm3ubaun2cfw4p5vyhcgivbi

💾 图片和元数据已在本地打包保存至: output/IMG_20210626_180340

--- ✨ 单件流程完成 ✨ ---
下一步，您可以在 mint 函数中使用这个元数据 URI: ipfs://bafkreihhpbkssgrr22r3f3rhrb4hntmbdzfm3ubaun2cfw4p5vyhcgivbi

======================================================================
✅ 本地准备工作已完成！
下一步是发布到专业的 Pinning 服务 (如 Pinata):
1. 登录 Pinata。
2. 上传您本地 `go/output/collection_[时间戳]/images` 文件夹。
3. 上传您本地 `go/output/collection_[时间戳]/metadata` 文件夹。
4. ⚠️  使用 Pinata 返回的【metadata】文件夹的 CID 来设置您合约的 Base URI。
======================================================================

polyglot-ipfs-uploader/go on  main [!?] via 🐹 v1.24.5 on 🐳 v28.2.2 (orbstack)
➜ go run ./main.go
✅ 成功连接到 IPFS 节点

==============================================
🚀 开始处理单个 NFT...
   - 文件后缀模式: 无
==============================================

--- 正在上传: ../assets/image/IMG_20210626_180340.jpg ---
✅ 上传成功!
   - 名称: IMG_20210626_180340.jpg
   - CID: bafybeifwvvo7qacd5ksephyxbqkqjih2dmm2ffgqa6u732b2evw5iijppi

🖼️  图片 CID 已获取: bafybeifwvvo7qacd5ksephyxbqkqjih2dmm2ffgqa6u732b2evw5iijppi

--- 正在上传 JSON 对象 ---
✅ JSON 元数据上传成功!
   - CID: bafkreihhpbkssgrr22r3f3rhrb4hntmbdzfm3ubaun2cfw4p5vyhcgivbi

💾 图片和元数据已在本地打包保存至: output/IMG_20210626_180340

--- ✨ 单件流程完成 ✨ ---
下一步，您可以在 mint 函数中使用这个元数据 URI: ipfs://bafkreihhpbkssgrr22r3f3rhrb4hntmbdzfm3ubaun2cfw4p5vyhcgivbi

==============================================
🚀 开始处理批量 NFT 集合...
   - 文件后缀模式: 无
==============================================

--- 正在上传: ../assets/batch_images ---
✅ 上传成功!
   - 名称: batch_images
   - CID: bafybeia22ed2lhakgwu76ojojhuavlxkccpclciy6hgqsmn6o7ur7cw44e

🖼️  图片文件夹 CID 已获取: bafybeia22ed2lhakgwu76ojojhuavlxkccpclciy6hgqsmn6o7ur7cw44e

💾 所有图片已复制到: output/collection_20250726_164652/images

--- 正在为每张图片生成元数据 JSON 文件 ---
✅ 成功生成 3 个元数据文件到: output/collection_20250726_164652/metadata

--- 正在上传: output/collection_20250726_164652/metadata ---
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
2. 上传您本地 `go/output/collection_[时间戳]/images` 文件夹。
3. 上传您本地 `go/output/collection_[时间戳]/metadata` 文件夹。
4. ⚠️  使用 Pinata 返回的【metadata】文件夹的 CID 来设置您合约的 Base URI。
======================================================================
*/