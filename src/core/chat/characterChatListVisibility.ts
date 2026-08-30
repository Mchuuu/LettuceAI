import type { SessionPreview } from "../storage/repo";
import { listSessionPreviews } from "../storage/repo";
import {
  getCharacterChatListVisibility,
  initializeCharacterChatListVisibility,
} from "../storage/appState";

export function deriveLegacyHiddenCharacterIds(previews: SessionPreview[]): Set<string> {
  const latestByCharacter = new Map<string, SessionPreview>();

  for (const preview of previews) {
    const current = latestByCharacter.get(preview.characterId);
    if (!current || preview.updatedAt > current.updatedAt) {
      latestByCharacter.set(preview.characterId, preview);
    }
  }

  return new Set(
    Array.from(latestByCharacter.values())
      .filter((preview) => preview.archived)
      .map((preview) => preview.characterId),
  );
}

export async function getHiddenCharacterIds(): Promise<Set<string>> {
  const visibility = await getCharacterChatListVisibility();
  if (visibility.migrated) return new Set(visibility.hiddenCharacterIds);

  const previews = await listSessionPreviews().catch(() => [] as SessionPreview[]);
  const legacyHiddenIds = deriveLegacyHiddenCharacterIds(previews);
  const hiddenCharacterIds = await initializeCharacterChatListVisibility(legacyHiddenIds);
  return new Set(hiddenCharacterIds);
}
