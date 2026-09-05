export const DOUBAO_SPEECH_PLAN_VERSION = "parenthetical-context-v1";

export interface DoubaoSpeechSegment {
  text: string;
  contextText?: string;
}

export interface DoubaoSpeechPlan {
  segments: DoubaoSpeechSegment[];
  extractedParentheticals: boolean;
  malformedParentheticals: boolean;
}

const PARENTHESIS_PAIRS: Record<string, string> = {
  "(": ")",
  "（": "）",
};

function combineContexts(values: Array<string | null | undefined>): string | undefined {
  const normalized = values
    .map((value) => value?.trim().replace(/[；;]+$/u, ""))
    .filter((value): value is string => Boolean(value));
  return normalized.length > 0 ? normalized.join("；") : undefined;
}

/**
 * Builds a provider-neutral speaking plan. Parenthetical content applies to the
 * next spoken segment; malformed parentheses preserve the full original text.
 */
export function buildDoubaoSpeechPlan(
  source: string,
  baseContextText?: string | null,
): DoubaoSpeechPlan {
  const originalText = source.trim();
  if (!originalText) {
    return {
      segments: [],
      extractedParentheticals: false,
      malformedParentheticals: false,
    };
  }

  const parts: Array<{ kind: "context" | "text"; value: string }> = [];
  let spoken = "";
  let index = 0;
  let extractedParentheticals = false;

  const flushSpoken = () => {
    const value = spoken.trim();
    spoken = "";
    if (value) parts.push({ kind: "text", value });
  };

  while (index < source.length) {
    const character = source[index];
    const closing = PARENTHESIS_PAIRS[character];
    if (!closing) {
      spoken += character;
      index += 1;
      continue;
    }

    const expectedClosings = [closing];
    let cursor = index + 1;
    while (cursor < source.length && expectedClosings.length > 0) {
      const current = source[cursor];
      const nestedClosing = PARENTHESIS_PAIRS[current];
      if (nestedClosing) {
        expectedClosings.push(nestedClosing);
      } else if (current === expectedClosings[expectedClosings.length - 1]) {
        expectedClosings.pop();
      }
      cursor += 1;
    }

    if (expectedClosings.length > 0) {
      return {
        segments: [
          {
            text: originalText,
            contextText: combineContexts([baseContextText]),
          },
        ],
        extractedParentheticals: false,
        malformedParentheticals: true,
      };
    }

    flushSpoken();
    extractedParentheticals = true;
    const context = source.slice(index + 1, cursor - 1).trim();
    if (context) parts.push({ kind: "context", value: context });
    index = cursor;
  }
  flushSpoken();

  const segments: DoubaoSpeechSegment[] = [];
  let pendingContexts: string[] = [];
  for (const part of parts) {
    if (part.kind === "context") {
      pendingContexts.push(part.value);
      continue;
    }
    segments.push({
      text: part.value,
      contextText: combineContexts([baseContextText, ...pendingContexts]),
    });
    pendingContexts = [];
  }

  return {
    segments,
    extractedParentheticals,
    malformedParentheticals: false,
  };
}

export function buildDoubaoSpeechPlanCachePrompt(basePrompt?: string): string {
  let payload: Record<string, unknown> = {};
  if (basePrompt?.trim()) {
    try {
      const parsed = JSON.parse(basePrompt) as unknown;
      if (parsed && typeof parsed === "object" && !Array.isArray(parsed)) {
        payload = parsed as Record<string, unknown>;
      }
    } catch {
      payload = { legacyPrompt: basePrompt };
    }
  }
  payload.speechPlanVersion = DOUBAO_SPEECH_PLAN_VERSION;
  return JSON.stringify(payload);
}
