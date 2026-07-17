# Cullify

Cullify 是一款面向摄影师的本地离线照片选片桌面应用。当前实现聚焦技术方案中的快速模式 MVP：使用 Tauri 2、Svelte 5 和 Rust，在本机完成照片扫描、缩略图生成、EXIF 读取、质量评分、连拍分组、人工决策和导出。

## 当前可交付能力

- 本地项目导入：选择照片文件夹，递归扫描 JPG、PNG、WebP、TIFF。
- 快速质量分析：Rust 原生模糊检测、曝光检测、质量分和自动淘汰建议。
- 照片元数据：读取尺寸、文件大小、拍摄时间、相机、镜头、焦距、光圈、快门、ISO。
- 缩略图缓存：生成 800px 长边 PNG 缩略图，网格使用缩略图，灯箱优先使用原图。
- 连拍分组：基于 pHash 相似度自动归入连拍组。
- 挑选工作台：筛选、排序、键盘 K/X/S、撤销、当前筛选批量全要/全不要/清空。
- 全屏灯箱：F 打开/关闭，Esc 关闭，左右键切换，灯箱内可继续决策。
- 项目管理：最近项目列表、重命名、删除项目记录与缩略图缓存。
- 设置持久化：Provider、模型、阈值、快捷键等配置保存为本地 TOML。
- 导出：用户选择导出目录，写出 JSON、CSV、ZIP；ZIP 内按 keep/cull/pending 分类复制原图。

## 当前边界

快速模式是当前生产可用链路。专家模式和竞技场模式按产品原型保留入口状态，但后端会拒绝创建非 `quick` 项目，避免展示未接入的 VLM 或 Tournament 能力。后续接入 Provider、模型任务队列和 A/B 竞技场状态机后再开放。

RAW 解码、VLM 深度分析、模型下载/校验/回滚、AI PDF 报告和自动更新仍属于后续版本范围。

## 技术栈

- Desktop: Tauri 2
- Frontend: SvelteKit + Svelte 5 + TypeScript
- Backend: Rust 2024
- Storage: SQLite via `rusqlite` bundled
- Image pipeline: `image`, `kamadak-exif`, custom pHash, ZIP export

## 数据位置

应用数据目录由 Tauri 的 `app.path().app_data_dir()` 解析，并落在 `com.cullify.desktop` 对应的应用数据目录下：

- SQLite: `com.cullify.desktop/cullify.db`
- 配置: `com.cullify.desktop/config.toml`
- 缩略图缓存: `com.cullify.desktop/thumbnails/<project_id>/`
- 默认导出根目录: `com.cullify.desktop/exports/`

正常 UI 导出会让用户选择目标文件夹；默认导出根目录用于后端兜底和测试。

## 开发命令

```bash
pnpm install
pnpm check
pnpm build
pnpm tauri dev
```

Rust 验证：

```bash
cargo fmt --manifest-path src-tauri/Cargo.toml
cargo test --package cullify --manifest-path src-tauri/Cargo.toml
```

桌面构建验证：

```bash
pnpm tauri build --debug --no-bundle --ci
```

## 验收建议

1. 启动桌面应用并新建快速模式项目。
2. 导入一个包含 JPG/PNG/WebP/TIFF 的照片文件夹。
3. 在挑选页验证网格、筛选、排序、K/X/S、Undo、批量标记和 F 灯箱。
4. 重命名项目，再删除测试项目，确认原始照片不会被删除。
5. 导出到用户指定文件夹，确认 JSON、CSV、ZIP 均存在，ZIP 内照片按 keep/cull/pending 分类。
