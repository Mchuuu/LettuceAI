export const PROVIDER_WEB_SEARCH_MODES = ["none", "volcengine-ark"] as const;

export type ProviderWebSearchMode = (typeof PROVIDER_WEB_SEARCH_MODES)[number];

export interface ProviderWebSearchConfig {
  mode: ProviderWebSearchMode;
  maxKeyword: number;
  limit: number;
  maxToolCalls: number;
}

const DEFAULT_CONFIG: ProviderWebSearchConfig = {
  mode: "none",
  maxKeyword: 2,
  limit: 10,
  maxToolCalls: 3,
};

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function boundedInteger(value: unknown, fallback: number, min: number, max: number): number {
  return typeof value === "number" && Number.isFinite(value)
    ? Math.min(max, Math.max(min, Math.trunc(value)))
    : fallback;
}

export function readProviderWebSearchConfig(
  providerConfig: Record<string, unknown> | undefined,
): ProviderWebSearchConfig {
  const raw = providerConfig?.webSearch;
  if (!isRecord(raw)) return { ...DEFAULT_CONFIG };

  return {
    mode: raw.mode === "volcengine-ark" ? "volcengine-ark" : "none",
    maxKeyword: boundedInteger(raw.maxKeyword, DEFAULT_CONFIG.maxKeyword, 1, 50),
    limit: boundedInteger(raw.limit, DEFAULT_CONFIG.limit, 1, 50),
    maxToolCalls: boundedInteger(raw.maxToolCalls, DEFAULT_CONFIG.maxToolCalls, 1, 10),
  };
}

export function writeProviderWebSearchConfig(
  providerConfig: Record<string, unknown> | undefined,
  webSearch: ProviderWebSearchConfig,
): Record<string, unknown> {
  const normalized = readProviderWebSearchConfig({ webSearch });
  return {
    ...providerConfig,
    webSearch: normalized,
  };
}
