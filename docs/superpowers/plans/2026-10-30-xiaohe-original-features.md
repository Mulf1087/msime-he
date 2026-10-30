# 小鹤音形功能接入计划

**Spec:** [设计说明](../specs/2026-10-30-xiaohe-original-features-design.md)

## 不变量

- 音形表项按 `entries(code,rank,text)` 的固定顺序显示；展示附加信息不参与排序和选词。
- 保留 `sm→什么`、`nirx→你`、四码唯一自动上屏、重码等待选择、分号次选、第五键顶屏和五项分页。
- 联网功能仅在原有或新增的显式开关打开时执行；不发送形码到只懂拼音的服务。
- 不将用户真实码表、偏好、凭据或输入历史提交到 Git。

## Task 1：形码提示与英文释义

**Files:** `crates/engine/src/session/{input,composition,mod}.rs`、`crates/engine/src/types.rs`、`crates/engine/src/session/tests.rs`、`crates/input-runtime/src/tests/`、`crates/host-api/src/tests.rs`。

1. 用合成音形码表及小鹤形码表写失败测试：普通候选显示小鹤形码；缺失表不影响输入；同一会话双拼辅助码配置不被改写；候选和自动上屏顺序保持。
2. 用现有翻译夹具写失败测试：音形候选在用户打开释义时进入查询；关闭时不请求；翻译回包只填译文。
3. 在音形会话读取独立的内置小鹤形码映射，用候选注释路径展示；保持 `helpcode_enabled` 的拼音筛选语义不变。
4. 打开音形的 `shows_glosses`，运行定向 Engine、Runtime、Host API 测试及 `cargo fmt`、`clippy`、macOS 原生测试。

## Task 2：音形专用显示设置

**Files:** `crates/client-core/src/preferences.rs`、`crates/host-api/src/lib.rs`、`packages/ui/src/settings/`、相应测试。

1. 新增独立的“显示小鹤形码”偏好，默认打开；设置页只在音形模式呈现，说明它不筛选或改变输入编码。
2. 偏好变化时仅更新注释；跨方案切换后，双拼仍使用自己选择的辅助码表。跑设置页类型、单测和原生切换验收。

## Task 3：云候选

**Files:** `crates/engine/src/session/{online,candidates,xiaohe}.rs`、`crates/host-api/src/ffi/candidates.rs`、`packages/ui/src/settings/`、相关测试。

1. 先定义音形在线查询文档和独立授权；自定义服务接收原样编码与模式，拼音服务仅接无歧义拼音转换。
2. 写旧代次回包、无结果、超时、四码自动提交、分号与本地固定首选回归，再实现只追加、不抢本地候选位的云行。
3. 用本地模拟 HTTP 服务验收网络协议，不拿公网服务是否可达判断正确性。

## Task 4：个人音形词条与云端同步

**Files:** `crates/engine/src/xiaohe/`、`crates/client-core/src/cloud/dictionary_sync/`、`crates/client-services/src/`、`crates/host-api/src/`、设置页与合成同步测试。

1. 单独建立音形用户覆盖层和有序冲突规则，不修改随包基线；本地增删、重装和导入回滚先通过测试。
2. 扩展 WebDAV 加密同步的数据类别，测试两设备并发、删除墓碑、断网重试和大规模基线不上传。
3. 检查账号词库服务的真实契约；若后端尚不支持音形类别，只保留 WebDAV 功能并在 UI 明示。

## Task 5：持续集成与交付

1. 对比上游当前 `develop` 的功能差异，逐批合并，避免直接覆盖音形分支；每批跑相关门禁。
2. 每个可验证切片独立提交；源码推送到 `Mulf1087/msime-he`。功能验收后重新构建 universal DMG，核对内置码表、新装与重装、签名及真实 macOS 输入行为。

## 当前执行点

公开基线已在 `main` 和 Release `v0.55.0-xiaohe-428597`。Task 1–2 的形码提示、释义和独立显示开关已经进入实现与验证；Task 3 的自定义服务云候选已经接通合成端到端测试。接下来完成全面门禁与 Task 4 的个人音形词条同步。Task 5 的安装包需在真实 macOS 输入行为验收后发布。
