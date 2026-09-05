interface UsageCarrier {
  id: string;
  usage?: object | null;
}

interface MessageWithVariants extends UsageCarrier {
  variants?: UsageCarrier[];
  selectedVariantId?: string | null;
}

export function applyTtsUsageToMessage<TMessage extends MessageWithVariants>(
  message: TMessage,
  messageId: string,
  variantId: string | undefined,
  ttsCharacters: number,
): TMessage {
  if (message.id !== messageId) return message;

  const withUsage = (usage: object | null | undefined) => ({ ...usage, ttsCharacters });
  const effectiveVariantId =
    message.selectedVariantId ?? message.variants?.[message.variants.length - 1]?.id;
  const variants = variantId
    ? message.variants?.map((variant) =>
        variant.id === variantId ? { ...variant, usage: withUsage(variant.usage) } : variant,
      )
    : message.variants;

  return {
    ...message,
    usage:
      variantId == null || effectiveVariantId === variantId
        ? withUsage(message.usage)
        : message.usage,
    variants,
  } as TMessage;
}
