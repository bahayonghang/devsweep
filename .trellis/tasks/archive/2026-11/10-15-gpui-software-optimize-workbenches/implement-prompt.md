# 软件验证 Prompt

这是顺序第 5 份，也是最后一份。前面的 C11、C13、C06、C14 Prompt 都做完或确认无需改代码之后，才打开本份。

任务目录是 C12，但本份承担还没做的原生验证。Software 工作台验证放在最后一节。
分支：`codex/gpui-desktop-rewrite`。产品提交是 `c7846e4` 或它之后的提交。
`6E827733`、`5BFE59AF`、`23F7D614` 全部作废，不能复用。

父顺序：`../10-15-gpui-desktop-rewrite/next-session-prompt.md`。

## 本份先做的验证

先构建当前 HEAD 的 release，再测量。不要用旧哈希。

```text
cargo build --locked --release --target-dir dist/goal-gpui-release
```

在 `desktop-gpui` 下执行，链接加 `RUSTFLAGS=-C link-arg=/Brepro`。
记录退出码、字节数和 SHA-256。rustc 1.99.0 不是 MSRV 1.88.0 通过。

然后只在这个新哈希上补前面故意留下的原生缺口。失败就记失败，不要放宽：

1. C11：16.7 ms PageDown、100 次取消、FocusPattern。样本不够就是未满足，不是 PASS。
   不要分叉 AccessKit。
2. C13：20 次创建、最小化、恢复、`WM_CLOSE`；大快照如果能隔离就测，不能就 NOT_RUN。
   OS 崩溃释放做不到就保持 NOT_RUN。显示的 PID 不能用来结束进程。
3. C06-AC4：只有存在可丢弃卷或可丢弃映像时才测 defrag、dism、`w32tm /resync`。
   这台机器的主机卷和系统映像不是可丢弃目标。没有可丢弃目标就保持 NOT_RUN。
   不要运行 dism `/Online` 或 `/RestoreHealth`。

## 最后一节：Software 工作台

上面三步记录完之后，才验证 Software。不要提前打开这一节。

- 只用隔离 hive 和缺失的更新提供器。没有隔离执行器时拒绝卸载，不调用真实卸载。
- 不改真实启动项，不读主机卸载清单来凑数。残留项保持不可用。
- 复核确认、过期摘要拒绝、`probe_failed`、权限拒绝、受保护项和再次确认。
  再次确认只能改变隔离 hive。
- Optimize 的主机行必须仍是未运行。guidance 和 observe 不可调度。
  不要把 C06-AC4 的 NOT_RUN 写成成功。
- 英文和 zh-CN 都要有当前哈希的 UIA 或键盘记录。旧截图不能顶替。
- C12 现有勾选只覆盖 `6E827733`。新哈希未复核前，不要归档，也不要宣称 C12 已验收。

## 结束后

停止。不要开始 C14 的打包、签名、安装或 Tauri 退役。
不要编辑 `desktop/`。本 Prompt 不授权提交、推送或归档。
把新哈希、每条仍开放的验收项，以及 Software 节是最后执行的，写进回复。
提交、构建成功或窗口能打开，都不等于重写完成。
