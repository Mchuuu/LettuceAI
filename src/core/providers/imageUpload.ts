import type { Model, ProviderCredential, Settings } from "../storage/schemas";

export const VOLCENGINE_ARK_FILES_ENDPOINT =
  "https://ark.cn-beijing.volces.com/api/v3/files";

export type ProviderImageUploadMode = "base64" | "volcengine-ark-files";
export type ProviderImageUploadApiKeySource = "provider" | "custom";

export interface ProviderImageUploadConfig {
  mode: ProviderImageUploadMode;
  endpoint: string;
  apiKeySource: ProviderImageUploadApiKeySource;
  apiKey?: string;
}

const DEFAULT_CONFIG: ProviderImageUploadConfig = {
  mode: "base64",
  endpoint: VOLCENGINE_ARK_FILES_ENDPOINT,
  apiKeySource: "provider",
};

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

export function readProviderImageUploadConfig(
  providerConfig: Record<string, unknown> | undefined,
): ProviderImageUploadConfig {
  const raw = providerConfig?.imageUpload;
  if (!isRecord(raw)) return { ...DEFAULT_CONFIG };

  return {
    mode: raw.mode === "volcengine-ark-files" ? "volcengine-ark-files" : "base64",
    endpoint:
      typeof raw.endpoint === "string" && raw.endpoint.trim()
        ? raw.endpoint
        : VOLCENGINE_ARK_FILES_ENDPOINT,
    apiKeySource: raw.apiKeySource === "custom" ? "custom" : "provider",
    apiKey: typeof raw.apiKey === "string" ? raw.apiKey : undefined,
  };
}

export function writeProviderImageUploadConfig(
  providerConfig: Record<string, unknown> | undefined,
  imageUpload: ProviderImageUploadConfig,
): Record<string, unknown> {
  return {
    ...providerConfig,
    imageUpload: {
      mode: imageUpload.mode,
      endpoint: imageUpload.endpoint.trim() || VOLCENGINE_ARK_FILES_ENDPOINT,
      apiKeySource: imageUpload.apiKeySource,
      ...(imageUpload.apiKey?.trim() ? { apiKey: imageUpload.apiKey } : {}),
    },
  };
}

export function resolveModelProviderCredential(
  settings: Settings,
  model: Model,
): ProviderCredential | null {
  if (model.providerCredentialId) {
    const explicit = settings.providerCredentials.find(
      (credential) =>
        credential.id === model.providerCredentialId &&
        credential.providerId === model.providerId,
    );
    if (explicit) return explicit;
  }

  const candidates = settings.providerCredentials.filter(
    (credential) => credential.providerId === model.providerId,
  );
  if (candidates.length === 0) return null;

  const defaultCredential = candidates.find(
    (credential) => credential.id === settings.defaultProviderCredentialId,
  );
  if (defaultCredential) return defaultCredential;
  if (candidates.length === 1) return candidates[0];

  if (model.providerLabel.trim()) {
    const labelMatch = candidates.find(
      (credential) => credential.label === model.providerLabel,
    );
    if (labelMatch) return labelMatch;
  }

  return (
    candidates.find((credential) => credential.defaultModel === model.name) ?? null
  );
}
