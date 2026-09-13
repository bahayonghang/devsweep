import { describe, expect, it } from "vitest";
import englishSource from "../../../resources/i18n/en.json";
import chineseSource from "../../../resources/i18n/zh-CN.json";
import {
  SHELL_V1_KEYS,
  formatBinaryBytes,
  message,
  metadata,
  resolvePresentationLanguage,
  uniqueAccelerators,
} from ".";

describe("desktop canonical catalogue adapter", () => {
  it("renders canonical mode labels and locale-specific metadata", () => {
    expect(message("en", "command.clean")).toBe("Clean");
    expect(message("zh-CN", "command.clean")).toBe("清理");
    expect(metadata("en", "command.clean")).toEqual({ accelerator: "c", group: "root_modes", truncation: "never" });
    expect(metadata("zh-CN", "command.clean")).toEqual({ accelerator: null, group: "root_modes", truncation: "never" });

    const rootModes = [
      ["command.analyze", "a", "f"],
      ["command.clean", "c", null],
      ["command.history", "h", "l"],
      ["command.optimize", "p", "y"],
      ["command.software", "s", "r"],
      ["command.status", "t", "z"],
    ] as const;
    for (const [key, english, chinese] of rootModes) {
      expect(metadata("en", key).accelerator).toBe(english);
      expect(metadata("zh-CN", key).accelerator).toBe(chinese);
      expect(metadata("en", key).group).toBe("root_modes");
      expect(metadata("zh-CN", key).group).toBe("root_modes");
    }
  });

  it("maps only confirmed Simplified-Chinese locale families", () => {
    for (const locale of ["zh-CN", "zh_SG", "zh-Hans", "zh-Hans-CN", "zh-Hans-SG"]) {
      expect(resolvePresentationLanguage(null, [locale])).toBe("zh-CN");
    }
    for (const locale of ["zh-TW", "zh-HK", "zh-MO", "zh-Hant", "fr-FR"]) {
      expect(resolvePresentationLanguage(null, [locale])).toBe("en");
    }
    expect(resolvePresentationLanguage("en", ["zh-CN"])).toBe("en");
  });

  it("uses binary units at exact boundaries", () => {
    expect([0, 1023, 1024, 1536, 1048576].map(formatBinaryBytes)).toEqual([
      "0 B", "1023 B", "1.0 KiB", "1.5 KiB", "1.0 MiB",
    ]);
  });

  it("exposes only collision-free accelerators in a visible scope", () => {
    expect([...uniqueAccelerators("en", ["command.clean", "command.status"])]).toEqual([
      ["command.clean", "c"], ["command.status", "t"],
    ]);
    expect([...uniqueAccelerators("zh-CN", ["command.clean", "command.status"])]).toEqual([
      ["command.status", "z"],
    ]);
    expect([...uniqueAccelerators("en", ["command.clean", "command.clean"])]).toEqual([]);
    expect(metadata("en", "field.path").truncation).toBe("user_data_visual_only");
  });

  it("keeps the canonical shell V1 namespace exact, closed, and locale-parallel", () => {
    const expected = {
      en: [
        "Help",
        "More",
        "Saving language preference…",
        "Language preference was not changed. Close Settings, check the presentation settings file, and try again.",
        "Dismiss",
        "Could not change destination. The current page remains active.",
        "Modes",
        "Language",
        "Cancel",
        "Choose a language, then press Enter to save.",
        "English",
        "Simplified Chinese",
        "Save",
        "Saving",
        "Language settings",
        "Loading presentation settings…",
        "DevSweep could not safely read the presentation settings file. Existing bytes were preserved.",
        "Close DevSweep, check the file, and try again.",
        "Presentation settings unavailable",
        "Map disk use. This mode is read-only.",
        "Review every Cleanup Target before anything moves.",
        "Inspect past cleanup records.",
        "Review catalogued actions before any run.",
        "Inspect protection policy without changing cleanup authority.",
        "Inspect the rule catalogue.",
        "Choose English or Simplified Chinese for this window.",
        "Review current-user software inventory before uninstall.",
        "Host facts from collectors. No score.",
        "Supporting destinations",
        "History",
        "Protection",
        "Rules",
        "Cleanup plan workbench",
      ],
      "zh-CN": [
        "帮助",
        "更多",
        "正在保存语言偏好…",
        "语言偏好未更改。请关闭设置、检查显示设置文件后重试。",
        "关闭",
        "无法切换目标页面；当前页面保持不变。",
        "模式",
        "语言",
        "取消",
        "选择语言，然后按 Enter 保存。",
        "英语",
        "简体中文",
        "保存",
        "正在保存",
        "语言设置",
        "正在加载显示设置…",
        "DevSweep 无法安全读取显示设置文件；现有字节已保留。",
        "请关闭 DevSweep、检查该文件后重试。",
        "显示设置不可用",
        "映射磁盘占用。此模式只读。",
        "先审查每个清理目标，再移动任何内容。",
        "查看既往清理记录。",
        "审查目录中的操作后再运行。",
        "查看保护策略，不改变清理权限。",
        "查看规则目录。",
        "为本窗口选择英语或简体中文。",
        "审查当前用户软件清单后再卸载。",
        "来自采集器的主机事实。无评分。",
        "支持目的地",
        "历史",
        "保护",
        "规则",
        "清理计划工作区",
      ],
    } as const;
    for (const [locale, source] of [["en", englishSource], ["zh-CN", chineseSource]] as const) {
      const shellKeys = Object.keys(source.messages).filter((key) => key.startsWith("shell.v1.")).sort();
      expect(shellKeys).toEqual(SHELL_V1_KEYS);
      SHELL_V1_KEYS.forEach((key, index) => {
        const entry = source.messages[key];
        expect(Object.keys(entry).sort()).toEqual([
          "accelerator", "count", "forms", "group", "placeholders", "truncation",
        ]);
        expect(entry).toEqual({
          forms: { other: expected[locale][index] },
          placeholders: [],
          count: "none",
          accelerator: null,
          group: null,
          truncation: "never",
        });
        expect(message(locale, key)).toBe(expected[locale][index]);
        expect(metadata(locale, key)).toEqual({ accelerator: null, group: null, truncation: "never" });
      });
    }
    expect(Object.keys(englishSource.messages).sort()).toEqual(Object.keys(chineseSource.messages).sort());
  });
});
