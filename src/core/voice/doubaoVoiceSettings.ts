import type { CharacterVoiceConfig } from "../storage/schemas";

export const DEFAULT_DOUBAO_VOICE_SETTINGS = {
  pitch: 0,
  speechRate: 0,
  loudnessRate: 0,
  speechExpressionEnabled: true,
};

export type DoubaoVoiceSettings = typeof DEFAULT_DOUBAO_VOICE_SETTINGS;

export interface DoubaoVoicePromptOptions {
  contextText?: string | null;
  expressiveClone?: boolean;
}

export function clampDoubaoVoiceSetting(
  key: "pitch" | "speechRate" | "loudnessRate",
  value: number,
): number {
  const min = key === "pitch" ? -12 : -50;
  const max = key === "pitch" ? 12 : 100;
  return Math.trunc(Math.max(min, Math.min(max, Number.isFinite(value) ? value : 0)));
}

export function normalizeDoubaoVoiceSettings(
  settings: CharacterVoiceConfig["doubaoVoiceSettings"] | null | undefined,
): DoubaoVoiceSettings {
  return {
    pitch: clampDoubaoVoiceSetting("pitch", settings?.pitch ?? 0),
    speechRate: clampDoubaoVoiceSetting("speechRate", settings?.speechRate ?? 0),
    loudnessRate: clampDoubaoVoiceSetting("loudnessRate", settings?.loudnessRate ?? 0),
    speechExpressionEnabled: settings?.speechExpressionEnabled ?? true,
  };
}

export function buildDoubaoVoicePrompt(
  settings: CharacterVoiceConfig["doubaoVoiceSettings"] | null | undefined,
  sampleRate?: number,
  options?: DoubaoVoicePromptOptions,
): string | undefined {
  const normalized = normalizeDoubaoVoiceSettings(settings);
  const payload: Record<string, unknown> = {
    pitch: normalized.pitch,
    speechRate: normalized.speechRate,
    loudnessRate: normalized.loudnessRate,
  };
  if (Number.isFinite(sampleRate) && (sampleRate ?? 0) > 0) {
    payload.sampleRate = Math.round(sampleRate as number);
  }
  const contextText = options?.contextText?.trim();
  if (normalized.speechExpressionEnabled && contextText) {
    payload.contextTexts = [contextText];
  }
  if (normalized.speechExpressionEnabled && options?.expressiveClone) {
    payload.model = "seed-tts-2.0-expressive";
  }
  if (
    payload.pitch === 0 &&
    payload.speechRate === 0 &&
    payload.loudnessRate === 0 &&
    payload.sampleRate === undefined &&
    payload.contextTexts === undefined &&
    payload.model === undefined
  ) {
    return undefined;
  }
  return JSON.stringify(payload);
}
