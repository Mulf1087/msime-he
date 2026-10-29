# 小鹤音形实施计划

> **For agentic workers:** 使用 superpowers:executing-plans，由当前会话执行任务；不自动派发实现代理。

**Goal:** 在本机水杉 macOS 增加可切换、按现用鼠须管静态码表及打字规则运行的小鹤音形模式。

**Architecture:** 新增独立 Engine 方案与只读本地码表；运行时接收引擎的提交事实，设置业务放共享 Rust，macOS 与共享 UI 只适配入口。既有 ABI 序号和默认值不变。

**Tech Stack:** Rust、SQLite、React/TypeScript、Objective-C++、macOS InputMethodKit、本机 librime 参考会话。

**Spec:** ../specs/2026-10-29-xiaohe-yinxing-design.md

## Global Constraints

- 仅本机 macOS 首版；不公开私人码表或自动发布。
- `SchemeType::XiaoheYinxing = 10`；既有 ABI 序号不变。
- 独立文件 `user_data/xiaohe-yinxing.db`，SQLite `user_version=1`。
- 输入规则以复制环境中的实际 Rime 会话为准；不得仅凭正则猜提交时机。

## Review Focus

- 重码与空码四码的提交时机、第五键是否保留。
- 分号既为次选键又为快符引导的上下文冲突。
- 导入失败及官方词库更新时保留用户音形表。
- 组字中切换方案与英文模式时不丢字。
- 私人码表与源码、测试、日志的隔离。

### Task 1: 参考会话与本地码表

**Files:** 仓库外交付目录中的迁移工具、数据库和汇总报告；源码内只记录数据格式文档。

**Interfaces:** 产出 `entries(code TEXT, rank INTEGER, text TEXT)` 及非私人参考按键夹具；主键 `(code,text)`、索引 `(code,rank)`。

- [x] 用本机 librime 的准确 ABI 在复制数据目录建立会话；真实 Rime 数据只读。
- [x] 记录单候选、重码、空码、分号与第五键参考结果；导出静态候选最终次序。
- [x] 与文本码表核对，报告来源、哈希、计数和差异数量。

### Task 2: Engine 方案

**Files:** 新增 `crates/engine/src/xiaohe/{mod,scheme,table}.rs`；修改 `types.rs`、`ime/{scheme,registry,mod}.rs`、`session/{composition,input,candidates,learning}.rs` 及相应测试。

**Interfaces:** `XiaoheScheme` 处理原始编码；`XiaoheTable::open(&Path)` 校验只读表；`query(&str) -> Result<Vec<WordItem>>` 按 rank 返回精确编码；自动提交判定由 Engine 提供给快照。

- [x] 先写合成码表的失败测试，覆盖顺序、z、全编码消费、缺表和损坏表。
- [x] 接入 ABI 10 及方案 traits 的显式选择，隔离拼音覆盖与学习。
- [x] 依参考夹具实现自动提交、空码、分号及取消；相关单测和 golden 回归通过。

### Task 3: 偏好、导入与 macOS 入口

**Files:** `crates/client-core/src/preferences.rs`、`crates/client-services/src/dictionary/`、`crates/host-api/src/`、`packages/ui/src/core/`、`packages/ui/src/settings/`、`platforms/macos/src/input/` 与工具栏标题路径。

**Interfaces:** 偏好值 `xiaohe_yinxing`；本地码表导入与状态服务；macOS 能力报告与新方案切换。

- [x] 测试偏好持久化、中文方案回切、缺表阻止启用、导入失败保留旧表。
- [x] 实现校验与原子导入、设置 UI、原生宿主标识和每页五项。
- [ ] 在实际安装中验证中英文与方案切换、热重载、其他方案的默认行为。

### Task 4: 完整应用验收与安装

**Files:** macOS 既有打包产物目录；仓库外验收报告和恢复说明。

- [x] 相关 Rust 测试、fmt、clippy、UI 类型检查与构建、macOS 原生测试、quick 门禁。
- [x] 独立原生宿主运行时比对所有导出编码与所有分页，零差异。
- [ ] 真正前台编辑器输入 `sm`、`b`、`bc` 和四码连续词（当前登录会话入口缓存待重新登录）。
- [x] 运行完整打包流程，核对设置应用、输入法和 CLI 同源及版本；本地安装构建号 428594。
- [x] 备份现有应用与数据，安装并核对真实路径、版本、签名及实际导入条数。
- [ ] 重新登录后添加小鹤专属菜单项并完成真实编辑器验收。
