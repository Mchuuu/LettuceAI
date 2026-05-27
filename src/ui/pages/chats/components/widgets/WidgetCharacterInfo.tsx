import { useAvatar } from "../../../../hooks/useAvatar";
import { useWidgetContext } from "./WidgetContext";

export function WidgetCharacterInfo() {
  const { character } = useWidgetContext();
  const avatarUrl = useAvatar(
    "character",
    character?.id,
    character?.avatarPath,
    "round",
  );

  if (!character) {
    return (
      <section className="rounded-2xl border border-fg/10 bg-fg/3 px-3 py-3 text-[12px] italic text-fg/40">
        No character loaded.
      </section>
    );
  }

  return (
    <section className="flex flex-col gap-2 rounded-2xl border border-fg/12 bg-fg/4 px-3 py-3">
      <header className="flex items-center gap-3">
        <div className="h-12 w-12 shrink-0 overflow-hidden rounded-full bg-fg/10">
          {avatarUrl ? (
            <img
              src={avatarUrl}
              alt={character.name}
              className="h-full w-full object-cover"
            />
          ) : (
            <div className="flex h-full w-full items-center justify-center text-sm font-semibold text-fg/40">
              {character.name.slice(0, 1).toUpperCase()}
            </div>
          )}
        </div>
        <div className="min-w-0 flex-1">
          <div className="truncate text-sm font-semibold text-fg/85">
            {character.name}
          </div>
          {character.nickname && (
            <div className="truncate text-[11px] text-fg/50">
              {character.nickname}
            </div>
          )}
        </div>
      </header>
      {character.description && (
        <p className="text-[12px] leading-snug text-fg/65 line-clamp-6">
          {character.description}
        </p>
      )}
    </section>
  );
}
