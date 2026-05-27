import { useMemo } from "react";
import { ArrowLeftRight, Plus, RefreshCw } from "lucide-react";
import type { ButtonNode } from "../../../../../core/storage/chatWidgetSchemas";
import { useWidgetContext } from "./WidgetContext";

interface WidgetButtonProps {
  node: ButtonNode;
}

const DEFAULT_LABEL: Record<ButtonNode["action"], string> = {
  regenerate: "Regenerate last reply",
  swap_places: "Swap places",
  new_session: "New session",
};

function ActionIcon({ action }: { action: ButtonNode["action"] }) {
  if (action === "regenerate") return <RefreshCw size={14} strokeWidth={2.2} />;
  if (action === "swap_places") return <ArrowLeftRight size={14} strokeWidth={2.2} />;
  return <Plus size={14} strokeWidth={2.2} />;
}

export function WidgetButton({ node }: WidgetButtonProps) {
  const ctx = useWidgetContext();
  const { handler, disabled, isToggle, toggled } = useMemo(() => {
    switch (node.action) {
      case "regenerate":
        return {
          handler: ctx.onRegenerate,
          disabled: !ctx.canRegenerate,
          isToggle: false,
          toggled: false,
        };
      case "swap_places":
        return {
          handler: ctx.onToggleSwapPlaces,
          disabled: false,
          isToggle: true,
          toggled: ctx.swapPlacesActive,
        };
      case "new_session":
        return {
          handler: ctx.onNewSession,
          disabled: !ctx.character,
          isToggle: false,
          toggled: false,
        };
    }
  }, [ctx, node.action]);

  const label = node.title ?? DEFAULT_LABEL[node.action];
  return (
    <section className="flex flex-col gap-1.5">
      <button
        type="button"
        onClick={() => void handler()}
        disabled={disabled}
        className={`flex items-center justify-between gap-2 rounded-2xl border px-3 py-2.5 text-left text-sm transition ${
          disabled
            ? "cursor-not-allowed border-fg/8 bg-fg/2 text-fg/30"
            : isToggle && toggled
              ? "border-accent/40 bg-accent/12 text-accent"
              : "border-fg/12 bg-fg/4 text-fg/80 hover:bg-fg/8"
        }`}
      >
        <span className="min-w-0 flex-1 truncate">{label}</span>
        <span className="shrink-0">
          <ActionIcon action={node.action} />
        </span>
      </button>
      {node.description && (
        <p className="px-0.5 text-[11px] leading-snug text-fg/45">{node.description}</p>
      )}
    </section>
  );
}
