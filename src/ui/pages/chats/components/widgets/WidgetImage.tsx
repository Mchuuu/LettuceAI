import type { ImageNode } from "../../../../../core/storage/chatWidgetSchemas";
import { useAvatar } from "../../../../hooks/useAvatar";
import { useImageData } from "../../../../hooks/useImageData";
import { useWidgetContext } from "./WidgetContext";

interface WidgetImageProps {
  node: ImageNode;
}

export function WidgetImage({ node }: WidgetImageProps) {
  const { character, persona } = useWidgetContext();
  const characterAvatarUrl = useAvatar(
    "character",
    character?.id,
    character?.avatarPath,
    "base",
  );
  const personaAvatarUrl = useAvatar(
    "persona",
    persona?.id,
    persona?.avatarPath,
    "base",
  );
  const libraryPath = node.source.kind === "library" ? node.source.path : undefined;
  const uploadPath = node.source.kind === "upload" ? node.source.path : undefined;
  const libraryUrl = useImageData(libraryPath);
  const uploadUrl = useImageData(uploadPath);

  let url: string | undefined;
  let alt = node.title ?? "Widget image";
  switch (node.source.kind) {
    case "character_avatar":
      url = characterAvatarUrl;
      alt = character?.name ?? alt;
      break;
    case "persona_avatar":
      url = personaAvatarUrl;
      alt = persona?.title ?? alt;
      break;
    case "library":
      url = libraryUrl;
      break;
    case "upload":
      url = uploadUrl;
      break;
  }

  return (
    <section className="flex flex-col gap-1.5">
      {(node.title || node.description) && (
        <header className="flex flex-col gap-0.5 px-0.5">
          {node.title && (
            <h3 className="text-sm font-semibold text-fg/75">{node.title}</h3>
          )}
          {node.description && (
            <p className="text-[11px] leading-snug text-fg/45">{node.description}</p>
          )}
        </header>
      )}
      <div className="overflow-hidden rounded-2xl border border-fg/10 bg-fg/3">
        {url ? (
          <img src={url} alt={alt} className="block h-auto w-full object-cover" />
        ) : (
          <div className="flex aspect-video items-center justify-center text-[12px] italic text-fg/35">
            No image
          </div>
        )}
      </div>
    </section>
  );
}
