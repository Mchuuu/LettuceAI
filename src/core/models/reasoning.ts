export const REASONING_MODES = ["provider-default", "auto", "enabled", "disabled"] as const;

export type ReasoningMode = (typeof REASONING_MODES)[number];

export const REASONING_EFFORTS = ["low", "medium", "high", "xhigh", "max"] as const;

export type ReasoningEffort = (typeof REASONING_EFFORTS)[number];

interface ReasoningSettingsLike {
  reasoningMode?: ReasoningMode | null;
  reasoningEnabled?: boolean | null;
}

export function readExplicitReasoningMode(
  settings: ReasoningSettingsLike | null | undefined,
): ReasoningMode | null {
  const mode = settings?.reasoningMode;
  if (mode && REASONING_MODES.includes(mode)) return mode;
  if (settings?.reasoningEnabled === true) return "enabled";
  if (settings?.reasoningEnabled === false) return "disabled";
  return null;
}

export function readModelReasoningMode(
  settings: ReasoningSettingsLike | null | undefined,
): ReasoningMode {
  return readExplicitReasoningMode(settings) ?? "provider-default";
}

export function reasoningModePatch(mode: ReasoningMode | null): ReasoningSettingsLike {
  return {
    reasoningMode: mode,
    reasoningEnabled:
      mode === "enabled" || mode === "auto" ? true : mode === "disabled" ? false : null,
  };
}

