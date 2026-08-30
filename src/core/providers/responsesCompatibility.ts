import type { ReasoningEffort } from "../models/reasoning";

export const RESPONSES_DIALECTS = ["openai", "volcengine-ark"] as const;

export type ResponsesDialect = (typeof RESPONSES_DIALECTS)[number];

export const OPENAI_REASONING_EFFORTS = ["low", "medium", "high"] as const satisfies readonly ReasoningEffort[];
export const ARK_REASONING_EFFORTS = [
  "low",
  "medium",
  "high",
  "xhigh",
  "max",
] as const satisfies readonly ReasoningEffort[];

export function readResponsesDialect(
  providerConfig: Record<string, unknown> | undefined,
): ResponsesDialect {
  if (providerConfig?.responsesDialect === "volcengine-ark") return "volcengine-ark";
  if (providerConfig?.responsesDialect === "openai") return "openai";

  // Compatibility for providers created while Ark Web Search was the only Ark marker.
  const webSearch = providerConfig?.webSearch;
  if (
    typeof webSearch === "object" &&
    webSearch !== null &&
    !Array.isArray(webSearch) &&
    (webSearch as Record<string, unknown>).mode === "volcengine-ark"
  ) {
    return "volcengine-ark";
  }
  return "openai";
}

export function writeResponsesDialect(
  providerConfig: Record<string, unknown> | undefined,
  responsesDialect: ResponsesDialect,
): Record<string, unknown> {
  return {
    ...providerConfig,
    responsesDialect,
  };
}

export function reasoningEffortsForDialect(
  dialect: ResponsesDialect,
): readonly ReasoningEffort[] {
  return dialect === "volcengine-ark" ? ARK_REASONING_EFFORTS : OPENAI_REASONING_EFFORTS;
}

