import type {
  AudioProvider,
  UserVoice,
} from "../storage/audioProviders";
import type { CharacterVoiceConfig } from "../storage/schemas";

interface ResolvedCharacterVoiceTargetBase {
  provider: AudioProvider;
  providerId: string;
  modelId?: string;
  voiceId: string;
}

export interface ResolvedUserVoiceTarget extends ResolvedCharacterVoiceTargetBase {
  source: "user";
  userVoice: UserVoice;
}

export interface ResolvedProviderVoiceTarget extends ResolvedCharacterVoiceTargetBase {
  source: "provider";
}

export type ResolvedCharacterVoiceTarget =
  | ResolvedUserVoiceTarget
  | ResolvedProviderVoiceTarget;

export function resolveCharacterVoiceTarget(
  config: CharacterVoiceConfig | null | undefined,
  providers: readonly AudioProvider[],
  userVoices: readonly UserVoice[],
): ResolvedCharacterVoiceTarget | null {
  if (!config) return null;

  if (config.source === "user") {
    const userVoice = userVoices.find((voice) => voice.id === config.userVoiceId);
    if (!userVoice?.voiceId) return null;
    const provider = providers.find((item) => item.id === userVoice.providerId);
    if (!provider) return null;
    return {
      source: "user",
      userVoice,
      provider,
      providerId: userVoice.providerId,
      modelId: userVoice.modelId || undefined,
      voiceId: userVoice.voiceId,
    };
  }

  if (!config.providerId || !config.voiceId) return null;
  const provider = providers.find((item) => item.id === config.providerId);
  if (!provider) return null;
  return {
    source: "provider",
    provider,
    providerId: config.providerId,
    modelId: config.modelId,
    voiceId: config.voiceId,
  };
}
