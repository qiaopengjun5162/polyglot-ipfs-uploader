package main

import (
	"bytes"
	"encoding/json"
	"fmt"
	"os"
	httpapi "github.com/ipfs/go-ipfs-http-client"
	"github.com/ipfs/interface-go-ipfs-core/options"
)

const ipfsApiUrl = "http://localhost:5001"

func uploadFileToIPFS(shell *httpapi.HttpApi, filePath string) {
	fmt.Printf("\n--- 正在上传文件: %s ---\n", filePath)
	file, err := os.Open(filePath)
	if err != nil {
		fmt.Fprintf(os.Stderr, "❌ 打开文件失败: %v\n", err)
		return
	}
	defer file.Close()
	cid, err := shell.Add(file, options.Add.Pin(true))
	if err != nil {
		fmt.Fprintf(os.Stderr, "❌ 上传文件到 IPFS 失败: %v\n", err)
		return
	}
	fmt.Println("✅ 文件上传成功!")
	fmt.Printf("   - CID: %s\n", cid.String())
}

func uploadJSONToIPFS(shell *httpapi.HttpApi, data map[string]interface{}) {
	fmt.Println("\n--- 正在上传 JSON 对象 ---")
	jsonData, err := json.Marshal(data)
	if err != nil {
		fmt.Fprintf(os.Stderr, "❌ 转换 JSON 失败: %v\n", err)
		return
	}
	cid, err := shell.Add(bytes.NewReader(jsonData), options.Add.Pin(true))
	if err != nil {
		fmt.Fprintf(os.Stderr, "❌ 上传 JSON 到 IPFS 失败: %v\n", err)
		return
	}
	fmt.Println("✅ JSON 上传成功!")
	fmt.Printf("   - CID: %s\n", cid.String())
}

func main() {
	shell, err := httpapi.NewURLApi(ipfsApiUrl)
	if err != nil {
		fmt.Fprintf(os.Stderr, "❌ 连接 IPFS 节点失败: %v\n", err)
		os.Exit(1)
	}
	fmt.Println("✅ 成功连接到 IPFS 节点")

	dummyFilePath := "temp_upload_file_go.txt"
	os.WriteFile(dummyFilePath, []byte("你好，IPFS！这是来自 Go 的问候。"), 0644)
	uploadFileToIPFS(shell, dummyFilePath)
	os.Remove(dummyFilePath)

	myNftMetadata := map[string]interface{}{
		"name":        "我的第一个Go NFT",
		"description": "这是一个使用 Go 上传的元数据。",
		"attributes": []map[string]string{
			{"trait_type": "语言", "value": "Go"},
		},
	}
	uploadJSONToIPFS(shell, myNftMetadata)
}
