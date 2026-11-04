# C06 实施 Prompt

这是顺序第 3 份。它是保持开放，不是新的实现，也不是软件验证。

任务：`10-15-gpui-optimize-maintenance`。分支：`codex/gpui-desktop-rewrite`。
C06-AC1 到 C06-AC3 已勾选。C06-AC4 未勾选。不要归档。

## 本份要做

确认 Optimize 行仍然把主机 defrag、DISM 和时钟同步显示为未运行。
guidance 和 observe 没有调度按钮，也不能被记成成功。
如果窗口把这些行画成成功，只修这处显示，让它回到未运行。不要新增执行器。

## 本份明确不做

- 不要对这台机器运行 defrag、dism `/Online`、`/RestoreHealth` 或 `w32tm /resync`。
- 没有可丢弃卷或可丢弃映像时，C06-AC4 保持 NOT_RUN。这不是 PASS。
- 可丢弃目标上的实测如果以后做，只能放进最后的软件验证 Prompt，不能放在本份。
- 不要勾选 C06-AC4，不要把 C12 的主机行改成成功，不要归档。

本 Prompt 不授权提交、推送、归档、安装或主机维修。
