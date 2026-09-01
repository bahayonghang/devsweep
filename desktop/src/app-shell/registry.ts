import { createElement, type ReactNode } from "react";
import type { DesktopBridge } from "../api/bridge";
import type { PresentationLanguageTag } from "../i18n";
import { AnalyzePage } from "../modes/analyze";
import { CleanWorkbench } from "../modes/clean";
import { OptimizeWorkbench } from "../modes/optimize";
import { SoftwareWorkbench } from "../modes/software";
import { StatusWorkbench } from "../modes/status";
import { OperationCoordinator } from "../state/operation-coordinator";
import { HistoryPage, ProtectionPage, RulesPage } from "../support";
import {
  MODE_IDS,
  SUPPORTING_DESTINATION_IDS,
  type ModeId,
  type ModeRegistration,
  type SupportingDestinationId,
  type SupportingDestinationRegistration,
} from "./AppShell";

export interface ShellRegistrationInput {
  readonly bridge: DesktopBridge;
  readonly coordinator: OperationCoordinator;
  readonly locale: PresentationLanguageTag;
}

const MODE_ADAPTERS: Record<ModeId, (input: ShellRegistrationInput) => ReactNode> = {
  clean: (input) => createElement(CleanWorkbench, {
    bridge: input.bridge,
    coordinator: input.coordinator,
    locale: input.locale,
  }),
  software: (input) => createElement(SoftwareWorkbench, {
    bridge: input.bridge,
    coordinator: input.coordinator,
    locale: input.locale,
  }),
  optimize: (input) => createElement(OptimizeWorkbench, {
    bridge: input.bridge,
    coordinator: input.coordinator,
    locale: input.locale,
  }),
  analyze: (input) => createElement(AnalyzePage, {
    bridge: input.bridge,
    coordinator: input.coordinator,
    locale: input.locale,
  }),
  status: (input) => createElement(StatusWorkbench, {
    bridge: input.bridge,
    coordinator: input.coordinator,
    locale: input.locale,
  }),
};

const SUPPORT_ADAPTERS: Record<
  SupportingDestinationId,
  (input: ShellRegistrationInput) => ReactNode
> = {
  protection: (input) => createElement(ProtectionPage, {
    bridge: input.bridge,
    locale: input.locale,
  }),
  rules: (input) => createElement(RulesPage, {
    bridge: input.bridge,
    locale: input.locale,
  }),
  history: (input) => createElement(HistoryPage, {
    bridge: input.bridge,
    locale: input.locale,
  }),
};

export function shippedModeRegistrations(
  input: ShellRegistrationInput,
): readonly ModeRegistration[] {
  return MODE_IDS.map((id) => ({
    id,
    render: () => MODE_ADAPTERS[id](input),
  }));
}

export function shippedSupportingRegistrations(
  input: ShellRegistrationInput,
): readonly SupportingDestinationRegistration[] {
  return SUPPORTING_DESTINATION_IDS.map((id) => ({
    id,
    render: () => SUPPORT_ADAPTERS[id](input),
  }));
}
