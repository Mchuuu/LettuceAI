export const CUSTOM_OPENAI_CHAT_PROVIDER_ID = "custom";
export const CUSTOM_OPENAI_RESPONSES_PROVIDER_ID = "custom-openai-responses";
export const CUSTOM_ANTHROPIC_PROVIDER_ID = "custom-anthropic";

export type CustomProviderId =
  | typeof CUSTOM_OPENAI_CHAT_PROVIDER_ID
  | typeof CUSTOM_OPENAI_RESPONSES_PROVIDER_ID
  | typeof CUSTOM_ANTHROPIC_PROVIDER_ID;
export type CustomAuthMode = "bearer" | "header" | "query" | "none";

const CUSTOM_PROVIDER_IDS = new Set<string>([
  CUSTOM_OPENAI_CHAT_PROVIDER_ID,
  CUSTOM_OPENAI_RESPONSES_PROVIDER_ID,
  CUSTOM_ANTHROPIC_PROVIDER_ID,
]);

export function isCustomProviderId(
  providerId: string | null | undefined,
): providerId is CustomProviderId {
  return !!providerId && CUSTOM_PROVIDER_IDS.has(providerId);
}

export function isCustomOpenAIProviderId(providerId: string | null | undefined): boolean {
  return (
    providerId === CUSTOM_OPENAI_CHAT_PROVIDER_ID ||
    providerId === CUSTOM_OPENAI_RESPONSES_PROVIDER_ID
  );
}

export function supportsCustomRoleMapping(providerId: string | null | undefined): boolean {
  return (
    providerId === CUSTOM_OPENAI_CHAT_PROVIDER_ID || providerId === CUSTOM_ANTHROPIC_PROVIDER_ID
  );
}

export function defaultCustomEndpoint(providerId: string | null | undefined): string {
  switch (providerId) {
    case CUSTOM_OPENAI_RESPONSES_PROVIDER_ID:
      return "/v1/responses";
    case CUSTOM_ANTHROPIC_PROVIDER_ID:
      return "/v1/messages";
    default:
      return "/v1/chat/completions";
  }
}

export function defaultCustomAuthMode(
  providerId: string | null | undefined,
): CustomAuthMode {
  return providerId === CUSTOM_OPENAI_RESPONSES_PROVIDER_ID ? "bearer" : "header";
}

export function createDefaultCustomProviderConfig(
  providerId: string | null | undefined,
): Record<string, unknown> | undefined {
  if (!isCustomProviderId(providerId)) return undefined;

  const common = {
    chatEndpoint: defaultCustomEndpoint(providerId),
    modelsEndpoint: "",
    fetchModelsEnabled: false,
    modelsListPath: "data",
    modelsIdPath: "id",
    modelsDisplayNamePath: "name",
    modelsDescriptionPath: "description",
    modelsContextLengthPath: "",
    authMode: defaultCustomAuthMode(providerId),
    authHeaderName: "x-api-key",
    authQueryParamName: "api_key",
    supportsStream: true,
  };

  if (providerId === CUSTOM_OPENAI_RESPONSES_PROVIDER_ID) {
    return {
      ...common,
      toolChoiceMode: "auto",
    };
  }

  return {
    ...common,
    systemRole: "system",
    userRole: "user",
    assistantRole: "assistant",
    mergeSameRoleMessages: true,
    ...(providerId === CUSTOM_OPENAI_CHAT_PROVIDER_ID ? { toolChoiceMode: "auto" } : {}),
  };
}
