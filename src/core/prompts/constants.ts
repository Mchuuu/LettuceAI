import type { TranslationKey } from "../i18n/context";
import type { PromptTemplateType } from "../storage/schemas";

export const APP_DEFAULT_TEMPLATE_ID = "prompt_app_default";
export const APP_LOCAL_ROLEPLAY_TEMPLATE_ID = "prompt_app_local_roleplay";
export const APP_COMPANION_TEMPLATE_ID = "prompt_app_companion";
export const APP_DYNAMIC_SUMMARY_TEMPLATE_ID = "prompt_app_dynamic_summary";
export const APP_DYNAMIC_MEMORY_TEMPLATE_ID = "prompt_app_dynamic_memory";
export const APP_DYNAMIC_MEMORY_LOCAL_TEMPLATE_ID = "prompt_app_dynamic_memory_local";
export const APP_HELP_ME_REPLY_TEMPLATE_ID = "prompt_app_help_me_reply";
export const APP_HELP_ME_REPLY_CONVERSATIONAL_TEMPLATE_ID =
  "prompt_app_help_me_reply_conversational";
export const APP_LOREBOOK_ENTRY_WRITER_TEMPLATE_ID = "prompt_app_lorebook_entry_writer";
export const LEGACY_APP_LOREBOOK_ENTRY_GENERATOR_TEMPLATE_ID =
  "prompt_app_lorebook_entry_generator";
export const APP_LOREBOOK_KEYWORD_GENERATOR_TEMPLATE_ID =
  "prompt_app_lorebook_keyword_generator";
export const APP_LOREBOOK_GENERATOR_PLANNER_TEMPLATE_ID =
  "prompt_app_lorebook_generator_planner";
export const APP_LOREBOOK_GENERATOR_WRITER_TEMPLATE_ID =
  "prompt_app_lorebook_generator_writer";
export const APP_LOREBOOK_GENERATOR_REFINE_TEMPLATE_ID =
  "prompt_app_lorebook_generator_refine";
export const APP_LOREBOOK_GENERATOR_COHERENCE_TEMPLATE_ID =
  "prompt_app_lorebook_generator_coherence";
export const APP_GROUP_CHAT_TEMPLATE_ID = "prompt_app_group_chat";
export const APP_GROUP_CHAT_ROLEPLAY_TEMPLATE_ID = "prompt_app_group_chat_roleplay";
export const APP_AVATAR_GENERATION_TEMPLATE_ID = "prompt_app_avatar_generation";
export const APP_AVATAR_EDIT_TEMPLATE_ID = "prompt_app_avatar_edit";
export const APP_SCENE_GENERATION_TEMPLATE_ID = "prompt_app_scene_generation";
export const APP_SCENE_PROMPT_WRITER_TEMPLATE_ID = "prompt_app_scene_prompt_writer";
export const APP_DESIGN_REFERENCE_TEMPLATE_ID = "prompt_app_design_reference";
export const APP_COMPANION_SOUL_WRITER_TEMPLATE_ID = "prompt_app_companion_soul_writer";
export const APP_COMPANION_GROWTHCYCLE_TEMPLATE_ID = "prompt_app_companion_growthcycle";
export const APP_COMPANION_CONSOLIDATION_TEMPLATE_ID = "prompt_app_companion_consolidation";

const PROTECTED_TEMPLATE_IDS = new Set([
  APP_DEFAULT_TEMPLATE_ID,
  APP_LOCAL_ROLEPLAY_TEMPLATE_ID,
  APP_COMPANION_TEMPLATE_ID,
  APP_DYNAMIC_SUMMARY_TEMPLATE_ID,
  APP_DYNAMIC_MEMORY_TEMPLATE_ID,
  APP_DYNAMIC_MEMORY_LOCAL_TEMPLATE_ID,
  APP_HELP_ME_REPLY_TEMPLATE_ID,
  APP_HELP_ME_REPLY_CONVERSATIONAL_TEMPLATE_ID,
  APP_LOREBOOK_ENTRY_WRITER_TEMPLATE_ID,
  LEGACY_APP_LOREBOOK_ENTRY_GENERATOR_TEMPLATE_ID,
  APP_LOREBOOK_KEYWORD_GENERATOR_TEMPLATE_ID,
  APP_LOREBOOK_GENERATOR_PLANNER_TEMPLATE_ID,
  APP_LOREBOOK_GENERATOR_WRITER_TEMPLATE_ID,
  APP_LOREBOOK_GENERATOR_REFINE_TEMPLATE_ID,
  APP_LOREBOOK_GENERATOR_COHERENCE_TEMPLATE_ID,
  APP_GROUP_CHAT_TEMPLATE_ID,
  APP_GROUP_CHAT_ROLEPLAY_TEMPLATE_ID,
  APP_AVATAR_GENERATION_TEMPLATE_ID,
  APP_AVATAR_EDIT_TEMPLATE_ID,
  APP_SCENE_GENERATION_TEMPLATE_ID,
  APP_SCENE_PROMPT_WRITER_TEMPLATE_ID,
  APP_DESIGN_REFERENCE_TEMPLATE_ID,
  APP_COMPANION_SOUL_WRITER_TEMPLATE_ID,
  APP_COMPANION_GROWTHCYCLE_TEMPLATE_ID,
  APP_COMPANION_CONSOLIDATION_TEMPLATE_ID,
]);

const NON_SYSTEM_TEMPLATE_IDS = new Set([
  APP_DYNAMIC_SUMMARY_TEMPLATE_ID,
  APP_DYNAMIC_MEMORY_TEMPLATE_ID,
  APP_DYNAMIC_MEMORY_LOCAL_TEMPLATE_ID,
  APP_HELP_ME_REPLY_TEMPLATE_ID,
  APP_HELP_ME_REPLY_CONVERSATIONAL_TEMPLATE_ID,
  APP_LOREBOOK_ENTRY_WRITER_TEMPLATE_ID,
  LEGACY_APP_LOREBOOK_ENTRY_GENERATOR_TEMPLATE_ID,
  APP_LOREBOOK_KEYWORD_GENERATOR_TEMPLATE_ID,
  APP_LOREBOOK_GENERATOR_PLANNER_TEMPLATE_ID,
  APP_LOREBOOK_GENERATOR_WRITER_TEMPLATE_ID,
  APP_LOREBOOK_GENERATOR_REFINE_TEMPLATE_ID,
  APP_LOREBOOK_GENERATOR_COHERENCE_TEMPLATE_ID,
  APP_GROUP_CHAT_TEMPLATE_ID,
  APP_GROUP_CHAT_ROLEPLAY_TEMPLATE_ID,
  APP_AVATAR_GENERATION_TEMPLATE_ID,
  APP_AVATAR_EDIT_TEMPLATE_ID,
  APP_SCENE_GENERATION_TEMPLATE_ID,
  APP_SCENE_PROMPT_WRITER_TEMPLATE_ID,
  APP_DESIGN_REFERENCE_TEMPLATE_ID,
  APP_COMPANION_SOUL_WRITER_TEMPLATE_ID,
  APP_COMPANION_GROWTHCYCLE_TEMPLATE_ID,
  APP_COMPANION_CONSOLIDATION_TEMPLATE_ID,
]);

type PromptTranslator = (key: TranslationKey) => string;

type BuiltInPromptTemplateName = {
  key: TranslationKey;
  defaultNames: readonly string[];
};

const BUILT_IN_PROMPT_TEMPLATE_NAMES: Readonly<Record<string, BuiltInPromptTemplateName>> = {
  [APP_DEFAULT_TEMPLATE_ID]: {
    key: "systemPrompts.builtInTemplates.appDefault",
    defaultNames: ["App Default"],
  },
  [APP_LOCAL_ROLEPLAY_TEMPLATE_ID]: {
    key: "systemPrompts.builtInTemplates.localRoleplayDefault",
    defaultNames: ["Local RP Default"],
  },
  [APP_COMPANION_TEMPLATE_ID]: {
    key: "systemPrompts.builtInTemplates.companionDefault",
    defaultNames: ["Companion Default"],
  },
  [APP_DYNAMIC_SUMMARY_TEMPLATE_ID]: {
    key: "systemPrompts.builtInTemplates.dynamicMemorySummarizer",
    defaultNames: ["Dynamic Memory: Summarizer"],
  },
  [APP_DYNAMIC_MEMORY_TEMPLATE_ID]: {
    key: "systemPrompts.builtInTemplates.dynamicMemoryManager",
    defaultNames: ["Dynamic Memory: Memory Manager"],
  },
  [APP_DYNAMIC_MEMORY_LOCAL_TEMPLATE_ID]: {
    key: "systemPrompts.builtInTemplates.dynamicMemoryManagerLocal",
    defaultNames: ["Dynamic Memory: Memory Manager (Local LLM)"],
  },
  [APP_HELP_ME_REPLY_TEMPLATE_ID]: {
    key: "systemPrompts.builtInTemplates.replyHelper",
    defaultNames: ["Reply Helper"],
  },
  [APP_HELP_ME_REPLY_CONVERSATIONAL_TEMPLATE_ID]: {
    key: "systemPrompts.builtInTemplates.replyHelperConversational",
    defaultNames: ["Reply Helper (Conversational)"],
  },
  [APP_LOREBOOK_ENTRY_WRITER_TEMPLATE_ID]: {
    key: "systemPrompts.builtInTemplates.lorebookEntryWriter",
    defaultNames: ["Lorebook Entry Writer", "Lorebook Entry Generator"],
  },
  [LEGACY_APP_LOREBOOK_ENTRY_GENERATOR_TEMPLATE_ID]: {
    key: "systemPrompts.builtInTemplates.lorebookEntryWriter",
    defaultNames: ["Lorebook Entry Writer", "Lorebook Entry Generator"],
  },
  [APP_LOREBOOK_KEYWORD_GENERATOR_TEMPLATE_ID]: {
    key: "systemPrompts.builtInTemplates.lorebookKeywordGenerator",
    defaultNames: ["Lorebook Keyword Generator"],
  },
  [APP_LOREBOOK_GENERATOR_PLANNER_TEMPLATE_ID]: {
    key: "systemPrompts.builtInTemplates.lorebookGeneratorPlanner",
    defaultNames: ["Lorebook Generator: Planner"],
  },
  [APP_LOREBOOK_GENERATOR_WRITER_TEMPLATE_ID]: {
    key: "systemPrompts.builtInTemplates.lorebookGeneratorWriter",
    defaultNames: ["Lorebook Generator: Writer"],
  },
  [APP_LOREBOOK_GENERATOR_REFINE_TEMPLATE_ID]: {
    key: "systemPrompts.builtInTemplates.lorebookGeneratorRefine",
    defaultNames: ["Lorebook Generator: Refine"],
  },
  [APP_LOREBOOK_GENERATOR_COHERENCE_TEMPLATE_ID]: {
    key: "systemPrompts.builtInTemplates.lorebookGeneratorCoherence",
    defaultNames: ["Lorebook Generator: Coherence"],
  },
  [APP_GROUP_CHAT_TEMPLATE_ID]: {
    key: "systemPrompts.builtInTemplates.groupChatConversational",
    defaultNames: ["Group Chat (Conversation)"],
  },
  [APP_GROUP_CHAT_ROLEPLAY_TEMPLATE_ID]: {
    key: "systemPrompts.builtInTemplates.groupChatRoleplay",
    defaultNames: ["Group Chat (Roleplay)"],
  },
  [APP_AVATAR_GENERATION_TEMPLATE_ID]: {
    key: "systemPrompts.builtInTemplates.avatarGeneration",
    defaultNames: ["Avatar Generation"],
  },
  [APP_AVATAR_EDIT_TEMPLATE_ID]: {
    key: "systemPrompts.builtInTemplates.avatarEdit",
    defaultNames: ["Avatar Image Edit"],
  },
  [APP_SCENE_GENERATION_TEMPLATE_ID]: {
    key: "systemPrompts.builtInTemplates.sceneGeneration",
    defaultNames: ["Scene Generation"],
  },
  [APP_SCENE_PROMPT_WRITER_TEMPLATE_ID]: {
    key: "systemPrompts.builtInTemplates.scenePromptWriter",
    defaultNames: ["Scene Prompt Writer"],
  },
  [APP_DESIGN_REFERENCE_TEMPLATE_ID]: {
    key: "systemPrompts.builtInTemplates.designReferenceWriter",
    defaultNames: ["Design Reference Writer"],
  },
  [APP_COMPANION_SOUL_WRITER_TEMPLATE_ID]: {
    key: "systemPrompts.builtInTemplates.companionSoulWriter",
    defaultNames: ["Companion Soul Writer"],
  },
  [APP_COMPANION_GROWTHCYCLE_TEMPLATE_ID]: {
    key: "systemPrompts.builtInTemplates.companionGrowthcycle",
    defaultNames: ["Companion Growthcycle"],
  },
  [APP_COMPANION_CONSOLIDATION_TEMPLATE_ID]: {
    key: "systemPrompts.builtInTemplates.companionConsolidation",
    defaultNames: ["Companion Consolidation"],
  },
};

const PROMPT_TYPE_NAME_KEYS: Readonly<Record<PromptTemplateType, TranslationKey>> = {
  undefined: "editPrompt.promptTypes.undefined",
  directChat: "editPrompt.promptTypes.directChat",
  companionChat: "editPrompt.promptTypes.companionChat",
  groupChatRoleplay: "editPrompt.promptTypes.groupChatRoleplay",
  groupChatConversational: "editPrompt.promptTypes.groupChatConversational",
  dynamicMemorySummarizer: "editPrompt.promptTypes.dynamicMemorySummarizer",
  dynamicMemoryManager: "editPrompt.promptTypes.dynamicMemoryManager",
  replyHelperRoleplay: "editPrompt.promptTypes.replyHelperRoleplay",
  replyHelperConversational: "editPrompt.promptTypes.replyHelperConversational",
  lorebookEntryWriter: "editPrompt.promptTypes.lorebookEntryWriter",
  lorebookKeywordGenerator: "editPrompt.promptTypes.lorebookKeywordGenerator",
  lorebookGeneratorPlanner: "editPrompt.promptTypes.lorebookGeneratorPlanner",
  lorebookGeneratorWriter: "editPrompt.promptTypes.lorebookGeneratorWriter",
  lorebookGeneratorRefine: "editPrompt.promptTypes.lorebookGeneratorRefine",
  lorebookGeneratorCoherence: "editPrompt.promptTypes.lorebookGeneratorCoherence",
  avatarGeneration: "editPrompt.promptTypes.avatarGeneration",
  avatarEditRequest: "editPrompt.promptTypes.avatarEditRequest",
  sceneGeneration: "editPrompt.promptTypes.sceneGeneration",
  scenePromptWriter: "editPrompt.promptTypes.scenePromptWriter",
  designReferenceWriter: "editPrompt.promptTypes.designReferenceWriter",
  companionSoulWriter: "editPrompt.promptTypes.companionSoulWriter",
  companionGrowthcycle: "editPrompt.promptTypes.companionGrowthcycle",
  companionConsolidation: "editPrompt.promptTypes.companionConsolidation",
};

export function isProtectedPromptTemplate(id: string): boolean {
  return PROTECTED_TEMPLATE_IDS.has(id);
}

export function isSystemPromptTemplate(id: string): boolean {
  return !NON_SYSTEM_TEMPLATE_IDS.has(id);
}

export function getPromptTemplateDisplayName(
  t: PromptTranslator,
  id: string,
  fallbackName: string,
): string {
  const builtIn = BUILT_IN_PROMPT_TEMPLATE_NAMES[id];
  if (!builtIn) return fallbackName;

  const normalizedName = fallbackName.trim();
  if (normalizedName && !builtIn.defaultNames.includes(normalizedName)) {
    return fallbackName;
  }

  return t(builtIn.key);
}

export function getPromptTypeName(t: PromptTranslator, type: PromptTemplateType): string {
  return t(PROMPT_TYPE_NAME_KEYS[type] ?? PROMPT_TYPE_NAME_KEYS.undefined);
}
