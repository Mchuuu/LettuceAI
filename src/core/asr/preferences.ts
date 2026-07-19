import type { AsrEngine, AsrInstalledModel } from "./api";

const ACTIVE_MODEL_STORAGE_KEY = "lettuce.asr.activeModel.v1";
const INPUT_BEHAVIOR_STORAGE_KEY = "lettuce.asr.inputBehavior.v1";

export const ASR_ACTIVE_MODEL_CHANGED_EVENT = "asr:active-model-changed";
export const ASR_INPUT_BEHAVIOR_CHANGED_EVENT = "asr:input-behavior-changed";

export type AsrInputBehavior = "dictation" | "holdToSend";

interface StoredAsrModelSelection {
  engine: AsrEngine;
  id: string;
}

function readStoredSelection(): StoredAsrModelSelection | null {
  try {
    const raw = window.localStorage.getItem(ACTIVE_MODEL_STORAGE_KEY);
    if (!raw) return null;
    const parsed = JSON.parse(raw) as Partial<StoredAsrModelSelection>;
    if (
      (parsed.engine !== "whisper" &&
        parsed.engine !== "senseVoice" &&
        parsed.engine !== "zipformerCtc") ||
      typeof parsed.id !== "string" ||
      !parsed.id
    ) {
      return null;
    }
    return { engine: parsed.engine, id: parsed.id };
  } catch {
    return null;
  }
}

export function resolveActiveAsrModel(models: AsrInstalledModel[]): AsrInstalledModel | null {
  const stored = readStoredSelection();
  if (stored) {
    const selected = models.find(
      (model) => model.engine === stored.engine && model.id === stored.id,
    );
    if (selected) return selected;
  }
  return models[0] ?? null;
}

export function setActiveAsrModel(model: AsrInstalledModel): void {
  const selection: StoredAsrModelSelection = {
    engine: model.engine,
    id: model.id,
  };
  try {
    window.localStorage.setItem(ACTIVE_MODEL_STORAGE_KEY, JSON.stringify(selection));
  } catch {
    return;
  }
  window.dispatchEvent(new CustomEvent(ASR_ACTIVE_MODEL_CHANGED_EVENT, { detail: selection }));
}

export function getAsrInputBehavior(): AsrInputBehavior {
  try {
    return window.localStorage.getItem(INPUT_BEHAVIOR_STORAGE_KEY) === "holdToSend"
      ? "holdToSend"
      : "dictation";
  } catch {
    return "dictation";
  }
}

export function setAsrInputBehavior(behavior: AsrInputBehavior): void {
  try {
    window.localStorage.setItem(INPUT_BEHAVIOR_STORAGE_KEY, behavior);
  } catch {
    return;
  }
  window.dispatchEvent(
    new CustomEvent(ASR_INPUT_BEHAVIOR_CHANGED_EVENT, { detail: behavior }),
  );
}
