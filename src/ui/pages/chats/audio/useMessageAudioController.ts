import { useCallback, useEffect, useRef, useState } from "react";

import { useI18n } from "../../../../core/i18n/context";
import {
  abortAudioPreview,
  listAudioModels,
  listAudioProviders,
  listUserVoices,
  resolveUserVoicePrompt,
  type AudioModel,
  type AudioProvider,
  type AudioProviderType,
  type TtsCacheContext,
  type TtsCacheConversationKind,
  type TtsPreviewResponse,
  type UserVoice,
} from "../../../../core/storage/audioProviders";
import type { Character } from "../../../../core/storage/schemas";
import { resolveCharacterVoiceTarget } from "../../../../core/voice/characterVoiceTarget";
import {
  buildDoubaoSpeechPlan,
  buildDoubaoSpeechPlanCachePrompt,
} from "../../../../core/voice/doubaoSpeechPlan";
import {
  buildDoubaoVoicePrompt,
  normalizeDoubaoVoiceSettings,
} from "../../../../core/voice/doubaoVoiceSettings";
import { getCachedDoubaoVoicePreviewMetadata } from "../../../../core/voice/doubaoVoicePreview";
import { startMessageAudioPlayback, type MessageAudioPlayback } from "./messageAudioPlayer";

const MAX_AUDIO_CACHE_ENTRIES = 50;

export type MessageAudioStatus = "loading" | "playing";

interface PlayableMessageVariant {
  id: string;
  ttsContextText?: string | null;
}

export interface PlayableAudioMessage {
  id: string;
  role: "system" | "user" | "assistant" | "scene";
  variants?: readonly PlayableMessageVariant[];
  selectedVariantId?: string | null;
  ttsContextText?: string | null;
}

export interface MessageAudioScope {
  conversationKind: TtsCacheConversationKind;
  conversationId: string;
  onTtsUsage?: (update: MessageTtsUsageUpdate) => void;
}

export interface MessageTtsUsageUpdate {
  messageId: string;
  variantId?: string;
  ttsCharacters: number;
}

interface AudioResourceCache {
  providers: AudioProvider[] | null;
  userVoices: UserVoice[] | null;
  modelsByProviderType: Map<AudioProviderType, AudioModel[]>;
}

function isAbortError(error: unknown): boolean {
  const message = error instanceof Error ? error.message : String(error);
  const normalized = message.toLowerCase();
  return normalized.includes("aborted") || normalized.includes("cancel");
}

function resolveTtsContextText(message: PlayableAudioMessage): string | null | undefined {
  const selectedVariant = message.selectedVariantId
    ? message.variants?.find((variant) => variant.id === message.selectedVariantId)
    : message.variants?.[message.variants.length - 1];
  return selectedVariant?.ttsContextText ?? message.ttsContextText;
}

function buildAudioCacheKey(params: {
  providerId: string;
  modelId: string;
  voiceId: string;
  text: string;
  prompt?: string | null;
}): string {
  return [
    params.providerId,
    params.modelId,
    params.voiceId,
    params.prompt?.trim() ?? "",
    params.text,
  ].join("::");
}

export function useMessageAudioController(scope?: MessageAudioScope | null) {
  const { t } = useI18n();
  const conversationKind = scope?.conversationKind;
  const conversationId = scope?.conversationId;
  const onTtsUsage = scope?.onTtsUsage;
  const scopeKey =
    conversationKind && conversationId ? `${conversationKind}:${conversationId}` : null;
  const resourceCacheRef = useRef<AudioResourceCache>({
    providers: null,
    userVoices: null,
    modelsByProviderType: new Map(),
  });
  const previewCacheRef = useRef<Map<string, TtsPreviewResponse>>(new Map());
  const [audioStatusByMessage, setAudioStatusByMessage] = useState<
    Record<string, MessageAudioStatus>
  >({});
  const playbackRef = useRef<MessageAudioPlayback | null>(null);
  const playingMessageIdRef = useRef<string | null>(null);
  const requestRef = useRef<{ requestId: string; messageId: string } | null>(null);
  const cancelledRequestIdsRef = useRef<Set<string>>(new Set());

  const ensureAudioProviders = useCallback(async () => {
    if (resourceCacheRef.current.providers) return resourceCacheRef.current.providers;
    const providers = await listAudioProviders();
    resourceCacheRef.current.providers = providers;
    return providers;
  }, []);

  const ensureUserVoices = useCallback(async () => {
    if (resourceCacheRef.current.userVoices) return resourceCacheRef.current.userVoices;
    const voices = await listUserVoices();
    resourceCacheRef.current.userVoices = voices;
    return voices;
  }, []);

  const ensureAudioModels = useCallback(async (providerType: AudioProviderType) => {
    const cached = resourceCacheRef.current.modelsByProviderType.get(providerType);
    if (cached) return cached;
    const models = await listAudioModels(providerType);
    resourceCacheRef.current.modelsByProviderType.set(providerType, models);
    return models;
  }, []);

  const setAudioStatus = useCallback((messageId: string, status: MessageAudioStatus | null) => {
    setAudioStatusByMessage((previous) => {
      if (status === null) {
        if (!(messageId in previous)) return previous;
        const next = { ...previous };
        delete next[messageId];
        return next;
      }
      if (previous[messageId] === status) return previous;
      return { ...previous, [messageId]: status };
    });
  }, []);

  const cacheAudioPreview = useCallback((key: string, response: TtsPreviewResponse) => {
    const cache = previewCacheRef.current;
    cache.set(key, response);
    if (cache.size <= MAX_AUDIO_CACHE_ENTRIES) return;
    const oldestKey = cache.keys().next().value;
    if (oldestKey) cache.delete(oldestKey);
  }, []);

  const stopAudioPlayback = useCallback(() => {
    playbackRef.current?.stop();
    playbackRef.current = null;
    const messageId = playingMessageIdRef.current;
    if (messageId) {
      playingMessageIdRef.current = null;
      setAudioStatus(messageId, null);
    }
  }, [setAudioStatus]);

  const cancelAudioGeneration = useCallback(async () => {
    const pending = requestRef.current;
    if (!pending) return;
    requestRef.current = null;
    cancelledRequestIdsRef.current.add(pending.requestId);
    playbackRef.current?.stop();
    playbackRef.current = null;
    if (playingMessageIdRef.current === pending.messageId) {
      playingMessageIdRef.current = null;
    }
    setAudioStatus(pending.messageId, null);
    try {
      await abortAudioPreview(pending.requestId);
    } catch (error) {
      console.warn("Failed to cancel audio preview:", error);
    }
  }, [setAudioStatus]);

  const stopMessageAudio = useCallback(
    (message: Pick<PlayableAudioMessage, "id">) => {
      if (playingMessageIdRef.current && playingMessageIdRef.current !== message.id) return;
      stopAudioPlayback();
    },
    [stopAudioPlayback],
  );

  const cancelMessageAudio = useCallback(
    (message: Pick<PlayableAudioMessage, "id">) => {
      if (requestRef.current && requestRef.current.messageId !== message.id) return;
      void cancelAudioGeneration();
    },
    [cancelAudioGeneration],
  );

  useEffect(() => {
    return () => {
      stopAudioPlayback();
      void cancelAudioGeneration();
    };
  }, [cancelAudioGeneration, scopeKey, stopAudioPlayback]);

  const playMessageAudio = useCallback(
    async (message: PlayableAudioMessage, text: string, character?: Character | null) => {
      if (message.id.startsWith("placeholder") || message.id.startsWith("temp-")) return;
      if (message.role !== "assistant" && message.role !== "scene") return;
      if (!character?.voiceConfig) return;

      const trimmedText = text.trim();
      if (!trimmedText) return;

      if (requestRef.current?.messageId === message.id) {
        await cancelAudioGeneration();
        return;
      }
      if (playingMessageIdRef.current === message.id) {
        stopAudioPlayback();
        return;
      }
      if (requestRef.current) await cancelAudioGeneration();
      if (playbackRef.current) stopAudioPlayback();

      const requestId = globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random()}`;
      requestRef.current = { requestId, messageId: message.id };
      setAudioStatus(message.id, "loading");

      try {
        const providers = await ensureAudioProviders();
        const voiceConfig = character.voiceConfig;
        let voiceTarget;

        if (voiceConfig.source === "user") {
          let voices = await ensureUserVoices();
          voiceTarget = resolveCharacterVoiceTarget(voiceConfig, providers, voices);
          if (!voiceTarget) {
            resourceCacheRef.current.userVoices = null;
            voices = await ensureUserVoices();
            voiceTarget = resolveCharacterVoiceTarget(voiceConfig, providers, voices);
          }
          if (!voiceTarget || voiceTarget.source !== "user") {
            const voiceExists = voices.some((voice) => voice.id === voiceConfig.userVoiceId);
            throw new Error(
              t(
                voiceExists
                  ? "chats.errors.assignedProviderNotFound"
                  : "chats.errors.assignedVoiceNotFound",
              ),
            );
          }
        } else {
          if (!voiceConfig.providerId || !voiceConfig.voiceId) {
            throw new Error(t("chats.errors.voiceMissingProviderDetails"));
          }
          voiceTarget = resolveCharacterVoiceTarget(voiceConfig, providers, []);
          if (!voiceTarget || voiceTarget.source !== "provider") {
            throw new Error(t("chats.errors.assignedProviderNotFound"));
          }
        }

        const { provider, providerId, voiceId } = voiceTarget;
        let modelId = voiceTarget.modelId;
        if (!modelId && voiceTarget.source === "provider") {
          if (provider.providerType === "kokoro" && provider.kokoroVariant) {
            modelId = provider.kokoroVariant;
          } else {
            const models = await ensureAudioModels(provider.providerType as AudioProviderType);
            modelId = models[0]?.id;
          }
        }
        if (!modelId) throw new Error(t("chats.errors.noAudioModelsForProvider"));

        const cloneSampleRate =
          provider.resourceId === "seed-icl-2.0"
            ? getCachedDoubaoVoicePreviewMetadata(providerId, voiceId)?.sampleRate
            : undefined;
        const ttsContextText = resolveTtsContextText(message);
        const normalizedDoubaoSettings =
          provider.providerType === "doubao_tts"
            ? normalizeDoubaoVoiceSettings(voiceConfig.doubaoVoiceSettings)
            : null;
        const speechPlan = normalizedDoubaoSettings?.speechExpressionEnabled
          ? buildDoubaoSpeechPlan(trimmedText, ttsContextText)
          : null;
        const useSpeechPlan = Boolean(
          speechPlan?.extractedParentheticals &&
          !speechPlan.malformedParentheticals &&
          speechPlan.segments.length > 0,
        );
        if (
          speechPlan?.extractedParentheticals &&
          !speechPlan.malformedParentheticals &&
          speechPlan.segments.length === 0
        ) {
          console.debug("[Doubao TTS] skipped parenthetical-only message", {
            messageId: message.id,
          });
          if (requestRef.current?.requestId === requestId) requestRef.current = null;
          setAudioStatus(message.id, null);
          return;
        }
        if (speechPlan?.malformedParentheticals) {
          console.warn("[Doubao TTS] malformed parenthetical text; using the original text", {
            messageId: message.id,
          });
        }
        const prompt =
          provider.providerType === "doubao_tts"
            ? buildDoubaoVoicePrompt(voiceConfig.doubaoVoiceSettings, cloneSampleRate, {
                contextText: useSpeechPlan ? undefined : ttsContextText,
                expressiveClone:
                  provider.resourceId === "seed-icl-2.0" || modelId === "seed-icl-2.0",
              })
            : voiceTarget.source === "user"
              ? resolveUserVoicePrompt(provider.providerType, voiceTarget.userVoice.prompt)
              : undefined;
        const cachePrompt = useSpeechPlan ? buildDoubaoSpeechPlanCachePrompt(prompt) : prompt;
        const cacheKey = buildAudioCacheKey({
          providerId,
          modelId,
          voiceId,
          text: trimmedText,
          prompt: cachePrompt,
        });
        const cached = previewCacheRef.current.get(cacheKey);
        const fallbackVariant = message.variants?.[message.variants.length - 1];
        const playbackVariantId = message.selectedVariantId ?? fallbackVariant?.id ?? undefined;
        const cacheContext: TtsCacheContext | undefined =
          conversationKind && conversationId
            ? {
                providerId,
                modelId,
                voiceId,
                reference: {
                  conversationKind,
                  conversationId,
                  messageId: message.id,
                  variantId: playbackVariantId,
                  characterId: character.id,
                },
              }
            : undefined;

        const playback = await startMessageAudioPlayback({
          providerId,
          providerType: provider.providerType as AudioProviderType,
          modelId,
          voiceId,
          text: trimmedText,
          prompt,
          cachePrompt,
          doubaoSegments: useSpeechPlan ? speechPlan?.segments : undefined,
          requestId,
          cacheContext,
          sampleRate: cloneSampleRate,
          streamDoubao: true,
          cached,
          onCache: (response) => cacheAudioPreview(cacheKey, response),
          onPlaybackStart: () => {
            if (requestRef.current?.requestId === requestId) requestRef.current = null;
            setAudioStatus(message.id, "playing");
          },
          onTtsUsage: (ttsCharacters) => {
            onTtsUsage?.({
              messageId: message.id,
              variantId: playbackVariantId,
              ttsCharacters,
            });
          },
        });

        if (requestRef.current && requestRef.current.requestId !== requestId) {
          playback.stop();
          cancelledRequestIdsRef.current.delete(requestId);
          return;
        }
        if (cancelledRequestIdsRef.current.has(requestId)) {
          playback.stop();
          cancelledRequestIdsRef.current.delete(requestId);
          setAudioStatus(message.id, null);
          return;
        }

        playbackRef.current = playback;
        playingMessageIdRef.current = message.id;
        void playback.done
          .catch((error) => {
            console.warn("Message audio playback ended with an error:", error);
          })
          .finally(() => {
            if (playbackRef.current === playback) {
              playbackRef.current = null;
              playingMessageIdRef.current = null;
              setAudioStatus(message.id, null);
            }
          });
      } catch (error) {
        if (requestRef.current?.requestId === requestId) requestRef.current = null;
        cancelledRequestIdsRef.current.delete(requestId);
        setAudioStatus(message.id, null);
        if (isAbortError(error)) return;
        throw error;
      }
    },
    [
      cacheAudioPreview,
      cancelAudioGeneration,
      conversationId,
      conversationKind,
      ensureAudioModels,
      ensureAudioProviders,
      ensureUserVoices,
      onTtsUsage,
      setAudioStatus,
      stopAudioPlayback,
      t,
    ],
  );

  return {
    audioStatusByMessage,
    playMessageAudio,
    stopMessageAudio,
    cancelMessageAudio,
    stopAudioPlayback,
    cancelAudioGeneration,
  };
}
