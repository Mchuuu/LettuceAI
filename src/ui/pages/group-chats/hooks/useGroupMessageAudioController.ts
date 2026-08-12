import { useCallback, useEffect, useRef } from "react";

import type { Character, GroupMessage } from "../../../../core/storage/schemas";
import { splitThinkTags } from "../../../../core/utils/thinkTags";
import { useMessageAudioController } from "../../chats/audio/useMessageAudioController";
import { replaceCharacterPlaceholders } from "../utils/replaceCharacterPlaceholders";

function resolveSelectedVariant(message: GroupMessage) {
  const variants = message.variants ?? [];
  if (message.selectedVariantId) {
    return variants.find((variant) => variant.id === message.selectedVariantId);
  }
  return variants[variants.length - 1];
}

function resolvePlaybackText(message: GroupMessage, characters: Character[]): string {
  const selectedVariant = resolveSelectedVariant(message);
  const content = splitThinkTags(selectedVariant?.content ?? message.content).content;
  return message.role === "scene" && characters.length > 0
    ? replaceCharacterPlaceholders(content, characters)
    : content;
}

function findLatestReply(messages: readonly GroupMessage[]): GroupMessage | undefined {
  for (let index = messages.length - 1; index >= 0; index -= 1) {
    const message = messages[index];
    if (
      (message.role === "assistant" || message.role === "scene") &&
      !message.id.startsWith("placeholder") &&
      !message.id.startsWith("temp-")
    ) {
      return message;
    }
  }
  return undefined;
}

interface GroupMessageAudioControllerOptions {
  scopeKey?: string | null;
  characters: Character[];
}

export function useGroupMessageAudioController({
  scopeKey,
  characters,
}: GroupMessageAudioControllerOptions) {
  const controller = useMessageAudioController(scopeKey);
  const autoplaySignatureRef = useRef<string | null>(null);

  useEffect(() => {
    autoplaySignatureRef.current = null;
  }, [scopeKey]);

  const getMessageCharacter = useCallback(
    (message: GroupMessage): Character | undefined => {
      const selectedVariant = resolveSelectedVariant(message);
      const characterId = selectedVariant?.speakerCharacterId ?? message.speakerCharacterId;
      if (!characterId) return undefined;
      return characters.find((character) => character.id === characterId);
    },
    [characters],
  );

  const playMessageAudio = useCallback(
    async (message: GroupMessage, text: string) => {
      await controller.playMessageAudio(message, text, getMessageCharacter(message));
    },
    [controller.playMessageAudio, getMessageCharacter],
  );

  const autoplayMessageAudio = useCallback(
    (message: GroupMessage | undefined) => {
      if (!message || (message.role !== "assistant" && message.role !== "scene")) return;
      if (message.id.startsWith("placeholder") || message.id.startsWith("temp-")) return;

      const character = getMessageCharacter(message);
      if (!character?.voiceAutoplay || !character.voiceConfig) return;

      const text = resolvePlaybackText(message, characters).trim();
      if (!text) return;

      const signature = [
        scopeKey ?? "",
        message.id,
        message.selectedVariantId ?? "",
        character.id,
        text,
      ].join(":");
      if (autoplaySignatureRef.current === signature) return;
      autoplaySignatureRef.current = signature;

      console.info("GroupChatPage: autoplaying finalized message audio", {
        messageId: message.id,
        characterId: character.id,
      });
      void controller.playMessageAudio(message, text, character).catch((error) => {
        console.error("GroupChatPage: failed to autoplay finalized message audio", error);
      });
    },
    [characters, controller.playMessageAudio, getMessageCharacter, scopeKey],
  );

  const autoplayLatestMessageAudio = useCallback(
    (messages: readonly GroupMessage[]) => {
      autoplayMessageAudio(findLatestReply(messages));
    },
    [autoplayMessageAudio],
  );

  return {
    audioStatusByMessage: controller.audioStatusByMessage,
    playMessageAudio,
    autoplayMessageAudio,
    autoplayLatestMessageAudio,
    stopMessageAudio: controller.stopMessageAudio,
    cancelMessageAudio: controller.cancelMessageAudio,
    getMessageCharacter,
  };
}
