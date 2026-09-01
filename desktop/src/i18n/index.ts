import { invoke } from "@tauri-apps/api/core";
import englishSource from "../../../resources/i18n/en.json";
import chineseSource from "../../../resources/i18n/zh-CN.json";

export type PresentationLanguageTag = "en" | "zh-CN";
export type MessageKey = keyof typeof englishSource.messages;
export type TruncationContract = "never" | "user_data_visual_only";

export const SHELL_V1_KEYS = [
  "shell.v1.help",
  "shell.v1.more",
  "shell.v1.persistence.saving",
  "shell.v1.persistence.unavailable",
  "shell.v1.route.dismiss",
  "shell.v1.route.error",
  "shell.v1.settings.action",
  "shell.v1.settings.cancel",
  "shell.v1.settings.instruction",
  "shell.v1.settings.option.en",
  "shell.v1.settings.option.zh_cn",
  "shell.v1.settings.save",
  "shell.v1.settings.saving",
  "shell.v1.settings.title",
  "shell.v1.store.loading",
  "shell.v1.store.unavailable.detail",
  "shell.v1.store.unavailable.recovery",
  "shell.v1.store.unavailable.title",
  "shell.v1.supporting",
  "shell.v1.supporting.history",
  "shell.v1.supporting.protection",
  "shell.v1.supporting.rules",
  "shell.v1.workbench",
] as const satisfies readonly MessageKey[];

interface Message {
  forms: Readonly<Record<string, string>>;
  placeholders: readonly string[];
  count: "none" | "cardinal";
  accelerator: string | null;
  group: string | null;
  truncation: TruncationContract;
}

interface Catalogue {
  locale: PresentationLanguageTag;
  messages: Readonly<Record<MessageKey, Message>>;
}

export interface PresentationSettings {
  language: PresentationLanguageTag | null;
}

export interface PresentationSettingsBridge {
  load(): Promise<PresentationSettings>;
  save(language: PresentationLanguageTag | null): Promise<PresentationSettings>;
}

const SIMPLIFIED_CHINESE_LOCALES = new Set([
  "zh-cn", "zh-sg", "zh-hans", "zh-hans-cn", "zh-hans-sg",
]);

function parseCatalogue(source: unknown, expectedLocale: PresentationLanguageTag): Catalogue {
  if (!source || typeof source !== "object") throw new Error("catalogue must be an object");
  const candidate = source as Record<string, unknown>;
  if (Object.keys(candidate).sort().join(",") !== "locale,messages") {
    throw new Error("catalogue must contain only locale and messages");
  }
  if (candidate.locale !== expectedLocale || !candidate.messages || typeof candidate.messages !== "object") {
    throw new Error(`catalogue must declare locale ${expectedLocale}`);
  }
  const messages = candidate.messages as Record<string, unknown>;
  const usedAccelerators = new Set<string>();
  for (const [key, raw] of Object.entries(messages)) {
    if (!raw || typeof raw !== "object") throw new Error(`message ${key} must be an object`);
    const message = raw as Partial<Message> & Record<string, unknown>;
    if (Object.keys(message).sort().join(",") !== "accelerator,count,forms,group,placeholders,truncation") {
      throw new Error(`message ${key} must contain the exact metadata fields`);
    }
    if (!message.forms || typeof message.forms !== "object" || typeof message.forms.other !== "string") {
      throw new Error(`message ${key} must contain an other form`);
    }
    if (!Array.isArray(message.placeholders) || !["none", "cardinal"].includes(message.count ?? "")) {
      throw new Error(`message ${key} has invalid count metadata`);
    }
    if (!message.placeholders.every((placeholder) => typeof placeholder === "string")
      || new Set(message.placeholders).size !== message.placeholders.length) {
      throw new Error(`message ${key} has invalid placeholder metadata`);
    }
    const expectedForms = message.count === "none"
      ? ["other"]
      : expectedLocale === "en" ? ["one", "other"] : ["other"];
    if (Object.keys(message.forms).sort().join(",") !== [...expectedForms].sort().join(",")) {
      throw new Error(`message ${key} plural forms do not match locale/count metadata`);
    }
    for (const form of Object.values(message.forms)) {
      if (typeof form !== "string") throw new Error(`message ${key} form must be text`);
      const actualPlaceholders = [...form.matchAll(/\{([A-Za-z0-9_]+)\}/g)].map((match) => match[1]);
      if ([...new Set(actualPlaceholders)].sort().join(",") !== [...message.placeholders].sort().join(",")) {
        throw new Error(`message ${key} form placeholder signature differs from metadata`);
      }
    }
    if (message.accelerator !== null && (typeof message.accelerator !== "string" || !/^[A-Za-z]$/.test(message.accelerator))) {
      throw new Error(`message ${key} has invalid accelerator metadata`);
    }
    if (message.group !== null && typeof message.group !== "string") {
      throw new Error(`message ${key} has invalid accelerator group metadata`);
    }
    if (message.accelerator !== null && message.group === null) {
      throw new Error(`message ${key} accelerator requires a visible-scope group`);
    }
    if (message.accelerator && message.group) {
      const identity = `${message.group}:${message.accelerator.toLowerCase()}`;
      if (usedAccelerators.has(identity)) throw new Error(`message ${key} reuses accelerator ${identity}`);
      usedAccelerators.add(identity);
    }
    if (!["never", "user_data_visual_only"].includes(message.truncation ?? "")) {
      throw new Error(`message ${key} has invalid truncation metadata`);
    }
  }
  return candidate as unknown as Catalogue;
}

const CATALOGUES: Readonly<Record<PresentationLanguageTag, Catalogue>> = {
  en: parseCatalogue(englishSource, "en"),
  "zh-CN": parseCatalogue(chineseSource, "zh-CN"),
};

function validateParity() {
  const englishKeys = Object.keys(CATALOGUES.en.messages).sort();
  const chineseKeys = Object.keys(CATALOGUES["zh-CN"].messages).sort();
  if (JSON.stringify(englishKeys) !== JSON.stringify(chineseKeys)) throw new Error("catalogue keys differ");
  for (const key of englishKeys as MessageKey[]) {
    const english = CATALOGUES.en.messages[key];
    const chinese = CATALOGUES["zh-CN"].messages[key];
    if (JSON.stringify([...english.placeholders].sort()) !== JSON.stringify([...chinese.placeholders].sort())
      || english.count !== chinese.count
      || english.group !== chinese.group
      || english.truncation !== chinese.truncation) {
      throw new Error(`catalogue metadata differs for ${key}`);
    }
  }
}
validateParity();

function decodeSettings(value: unknown): PresentationSettings {
  if (!value || typeof value !== "object" || Array.isArray(value)) throw new Error("settings response must be an object");
  const record = value as Record<string, unknown>;
  if (Object.keys(record).sort().join(",") !== "language") throw new Error("settings response has an unknown field");
  if (record.language !== null && record.language !== "en" && record.language !== "zh-CN") {
    throw new Error("settings response has an unknown language tag");
  }
  return { language: record.language } as PresentationSettings;
}

export const tauriPresentationSettingsBridge: PresentationSettingsBridge = {
  async load() { return decodeSettings(await invoke("presentation_settings_get")); },
  async save(language) { return decodeSettings(await invoke("presentation_settings_set", { language })); },
};

export function resolvePresentationLanguage(
  explicit: PresentationLanguageTag | null,
  userLocales: readonly string[],
): PresentationLanguageTag {
  if (explicit) return explicit;
  for (const locale of userLocales) {
    const normalized = locale.replaceAll("_", "-").toLowerCase();
    if (SIMPLIFIED_CHINESE_LOCALES.has(normalized)) return "zh-CN";
    if (normalized === "en" || normalized.startsWith("en-")) return "en";
  }
  return "en";
}

export function message(
  locale: PresentationLanguageTag,
  key: MessageKey,
  values: Readonly<Record<string, string>> = {},
  count?: number,
): string {
  const entry = CATALOGUES[locale].messages[key];
  const form = locale === "en" && count === 1 && "one" in entry.forms ? "one" : "other";
  const supplied = Object.keys(values).sort();
  const expected = [...entry.placeholders].sort();
  if (JSON.stringify(supplied) !== JSON.stringify(expected)) throw new Error(`interpolation signature differs for ${key}`);
  return entry.forms[form].replaceAll(/\{([A-Za-z0-9_]+)\}/g, (_, name: string) => values[name]);
}

export function metadata(locale: PresentationLanguageTag, key: MessageKey) {
  const entry = CATALOGUES[locale].messages[key];
  return { accelerator: entry.accelerator?.toLowerCase() ?? null, group: entry.group, truncation: entry.truncation } as const;
}

export function uniqueAccelerators(locale: PresentationLanguageTag, keys: readonly MessageKey[]): ReadonlyMap<MessageKey, string> {
  const candidates = keys.map((key) => [key, metadata(locale, key)] as const);
  const counts = new Map<string, number>();
  for (const [, item] of candidates) {
    if (item.accelerator && item.group) {
      const identity = `${item.group}:${item.accelerator}`;
      counts.set(identity, (counts.get(identity) ?? 0) + 1);
    }
  }
  return new Map(candidates.flatMap(([key, item]) => {
    if (!item.accelerator || !item.group || counts.get(`${item.group}:${item.accelerator}`) !== 1) return [];
    return [[key, item.accelerator] as const];
  }));
}

export function formatBinaryBytes(bytes: number): string {
  if (!Number.isSafeInteger(bytes) || bytes < 0) throw new Error("bytes must be a non-negative safe integer");
  const units = ["B", "KiB", "MiB", "GiB", "TiB"] as const;
  if (bytes < 1024) return `${bytes} B`;
  let divisor = 1024;
  let unit = 1;
  while (unit < units.length - 1 && bytes >= divisor * 1024) { divisor *= 1024; unit += 1; }
  return `${(Math.round(bytes * 10 / divisor) / 10).toFixed(1)} ${units[unit]}`;
}
