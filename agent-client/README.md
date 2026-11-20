# agent-client（暂定名）

架阁的通用 Agent 客户端。**不是**云办公，是「基础 Agent + 只算一次的上下文」。

- GUI：Slint 声明式（纯 Rust 渲染，无 WebView）
- 内核：`crates/agent-core`，状态机 + 前缀指纹，不碰窗口、不碰网络
- 与主线（`E:\AI\ques-src-20261119` 的 `feat/agent-client` 分支）共用身份、上下文拼装、直通协议；
  唯一分叉：Agent loop 跑在本机进程，文件只是普通附件

## 跑

```
cargo run -p agent-app        # 出窗口
```

## 目录

```
crates/agent-core   状态机 Step / 前缀 prefix_hash / 工具 trait
crates/agent-app    Slint 窗口 + 二进制 jiage.exe
```
