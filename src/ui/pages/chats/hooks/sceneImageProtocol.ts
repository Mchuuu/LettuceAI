const SCENE_TAG_OPEN = "<img>";
const SCENE_CLOSE_TOKENS = ["</img>", "[continue]", "[/continue]"];
const VOICE_EXP_TAG_OPEN = "<voice_exp>";
const VOICE_EXP_CLOSE_TOKENS = ["</voice_exp>"];
type DirectiveKind = "scene" | "voice_exp";

export type SceneDirectiveStreamState = {
  carry: string;
  activeDirective: DirectiveKind | null;
  promptBuffer: string;
  extractedPrompt: string | null;
  voiceExpressionBuffer: string;
  extractedVoiceExpression: string | null;
};

export function createSceneDirectiveStreamState(): SceneDirectiveStreamState {
  return {
    carry: "",
    activeDirective: null,
    promptBuffer: "",
    extractedPrompt: null,
    voiceExpressionBuffer: "",
    extractedVoiceExpression: null,
  };
}

function longestSuffixPrefixLength(value: string, token: string): number {
  const valueLower = value.toLowerCase();
  const tokenLower = token.toLowerCase();
  const maxLength = Math.min(value.length, token.length - 1);
  for (let length = maxLength; length > 0; length--) {
    if (valueLower.endsWith(tokenLower.slice(0, length))) {
      return length;
    }
  }
  return 0;
}

function findEarliestTokenIndex(value: string, tokens: string[]): { index: number; token: string } | null {
  const valueLower = value.toLowerCase();
  let bestIndex = -1;
  let bestToken = "";

  for (const token of tokens) {
    const index = valueLower.indexOf(token.toLowerCase());
    if (index === -1) continue;
    if (bestIndex === -1 || index < bestIndex) {
      bestIndex = index;
      bestToken = token;
    }
  }

  return bestIndex >= 0 ? { index: bestIndex, token: bestToken } : null;
}

function longestAnyTokenPrefixSuffixLength(value: string, tokens: string[]): number {
  return tokens.reduce(
    (best, token) => Math.max(best, longestSuffixPrefixLength(value, token)),
    0,
  );
}

export function consumeSceneDirectiveDelta(
  state: SceneDirectiveStreamState,
  text: string,
): { content: string; prompt: string | null; voiceExpression: string | null } {
  let remaining = state.carry + text;
  let visibleContent = "";
  state.carry = "";

  while (remaining.length > 0) {
    if (!state.activeDirective) {
      const openMatch = findEarliestTokenIndex(remaining, [SCENE_TAG_OPEN, VOICE_EXP_TAG_OPEN]);
      if (openMatch) {
        visibleContent += remaining.slice(0, openMatch.index);
        remaining = remaining.slice(openMatch.index + openMatch.token.length);
        state.activeDirective =
          openMatch.token.toLowerCase() === VOICE_EXP_TAG_OPEN ? "voice_exp" : "scene";
        if (state.activeDirective === "scene") state.promptBuffer = "";
        else state.voiceExpressionBuffer = "";
        continue;
      }

      const partialLength = longestAnyTokenPrefixSuffixLength(remaining, [
        SCENE_TAG_OPEN,
        VOICE_EXP_TAG_OPEN,
      ]);
      const visibleLength = remaining.length - partialLength;
      if (visibleLength > 0) {
        visibleContent += remaining.slice(0, visibleLength);
      }
      state.carry = remaining.slice(visibleLength);
      break;
    }

    const closeTokens =
      state.activeDirective === "scene" ? SCENE_CLOSE_TOKENS : VOICE_EXP_CLOSE_TOKENS;
    const closeMatch = findEarliestTokenIndex(remaining, closeTokens);
    if (closeMatch) {
      const value = remaining.slice(0, closeMatch.index);
      if (state.activeDirective === "scene") {
        state.promptBuffer += value;
        const prompt = state.promptBuffer.trim();
        if (!state.extractedPrompt && prompt) state.extractedPrompt = prompt;
        state.promptBuffer = "";
      } else {
        state.voiceExpressionBuffer += value;
        const expression = state.voiceExpressionBuffer.trim();
        if (expression) state.extractedVoiceExpression = expression;
        state.voiceExpressionBuffer = "";
      }
      remaining = remaining.slice(closeMatch.index + closeMatch.token.length);
      state.activeDirective = null;
      continue;
    }

    const partialLength = longestAnyTokenPrefixSuffixLength(remaining, closeTokens);
    const valueLength = remaining.length - partialLength;
    if (valueLength > 0) {
      if (state.activeDirective === "scene") {
        state.promptBuffer += remaining.slice(0, valueLength);
      } else {
        state.voiceExpressionBuffer += remaining.slice(0, valueLength);
      }
    }
    state.carry = remaining.slice(valueLength);
    break;
  }

  return {
    content: visibleContent,
    prompt: state.extractedPrompt,
    voiceExpression: state.extractedVoiceExpression,
  };
}

export function finalizeSceneDirectiveStream(
  state: SceneDirectiveStreamState,
): { content: string; prompt: string | null; voiceExpression: string | null } {
  if (state.activeDirective) {
    state.carry = "";
    state.promptBuffer = "";
    state.voiceExpressionBuffer = "";
    state.activeDirective = null;
    return {
      content: "",
      prompt: state.extractedPrompt,
      voiceExpression: state.extractedVoiceExpression,
    };
  }

  const tail = state.carry;
  state.carry = "";
  return {
    content: tail,
    prompt: state.extractedPrompt,
    voiceExpression: state.extractedVoiceExpression,
  };
}

export function sanitizeAssistantSceneDirective(content: string): {
  cleanContent: string;
  scenePrompt: string | null;
  ttsContextText: string | null;
} {
  const streamState = createSceneDirectiveStreamState();
  const firstPass = consumeSceneDirectiveDelta(streamState, content);
  const tail = finalizeSceneDirectiveStream(streamState);

  return {
    cleanContent: `${firstPass.content}${tail.content}`.trim(),
    scenePrompt: tail.prompt?.trim() || firstPass.prompt?.trim() || null,
    ttsContextText:
      tail.voiceExpression?.trim() || firstPass.voiceExpression?.trim() || null,
  };
}
