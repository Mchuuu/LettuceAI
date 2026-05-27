import { useMemo, useState } from "react";
import { ChevronDown, Check, Cpu, User, X } from "lucide-react";
import type { SelectorNode } from "../../../../../core/storage/chatWidgetSchemas";
import { BottomMenu, MenuButton } from "../../../../components";
import { useWidgetContext } from "./WidgetContext";

interface WidgetSelectorProps {
  node: SelectorNode;
}

const DEFAULT_LABEL: Record<SelectorNode["kind"], string> = {
  persona: "Persona",
  model: "Model",
  fallback_model: "Fallback model",
};

interface SelectorOption {
  id: string;
  label: string;
  sublabel?: string;
  isCurrent: boolean;
}

export function WidgetSelector({ node }: WidgetSelectorProps) {
  const ctx = useWidgetContext();
  const [open, setOpen] = useState(false);

  const { currentLabel, options, onSelect } = useMemo(() => {
    if (node.kind === "persona") {
      const currentId = ctx.persona?.id ?? null;
      const opts: SelectorOption[] = ctx.personas.map((p) => ({
        id: p.id,
        label: p.title,
        sublabel: p.nickname ?? undefined,
        isCurrent: p.id === currentId,
      }));
      return {
        currentLabel: ctx.persona?.title ?? "None",
        options: opts,
        onSelect: (id: string | null) => void ctx.onSelectPersona(id),
      };
    }
    if (node.kind === "model") {
      const currentId = ctx.currentModelId;
      const opts: SelectorOption[] = ctx.models.map((m) => ({
        id: m.id,
        label: m.name,
        sublabel: m.providerId ?? undefined,
        isCurrent: m.id === currentId,
      }));
      const current = ctx.models.find((m) => m.id === currentId);
      return {
        currentLabel: current?.name ?? "Not set",
        options: opts,
        onSelect: (id: string | null) => {
          if (id != null) void ctx.onSelectModel(id);
        },
      };
    }
    // fallback_model
    const currentId = ctx.fallbackModelId;
    const opts: SelectorOption[] = ctx.models.map((m) => ({
      id: m.id,
      label: m.name,
      sublabel: m.providerId ?? undefined,
      isCurrent: m.id === currentId,
    }));
    const current = ctx.models.find((m) => m.id === currentId);
    return {
      currentLabel: current?.name ?? "None",
      options: opts,
      onSelect: (id: string | null) => void ctx.onSelectFallbackModel(id),
    };
  }, [ctx, node.kind]);

  const label = node.title ?? DEFAULT_LABEL[node.kind];
  const allowClear = node.kind === "persona" || node.kind === "fallback_model";

  return (
    <section className="flex flex-col gap-1.5">
      <header className="flex flex-col gap-0.5 px-0.5">
        <h3 className="text-sm font-semibold text-fg/75">{label}</h3>
        {node.description && (
          <p className="text-[11px] leading-snug text-fg/45">{node.description}</p>
        )}
      </header>
      <button
        type="button"
        onClick={() => setOpen(true)}
        className="flex items-center justify-between gap-2 rounded-2xl border border-fg/12 bg-fg/4 px-3 py-2.5 text-left text-sm text-fg/80 transition hover:bg-fg/8"
      >
        <span className="min-w-0 flex-1 truncate">{currentLabel}</span>
        <ChevronDown size={14} strokeWidth={2.2} className="shrink-0 text-fg/50" />
      </button>
      <BottomMenu isOpen={open} onClose={() => setOpen(false)} title={label}>
        <div className="flex flex-col gap-2">
          {allowClear && (
            <MenuButton
              icon={X}
              title="Clear selection"
              onClick={() => {
                onSelect(null);
                setOpen(false);
              }}
            />
          )}
          {options.length === 0 ? (
            <div className="px-4 py-3 text-[12px] italic text-fg/40">
              No options available.
            </div>
          ) : (
            options.map((opt) => (
              <MenuButton
                key={opt.id}
                icon={node.kind === "persona" ? User : Cpu}
                title={opt.label}
                description={opt.sublabel}
                rightElement={
                  opt.isCurrent ? (
                    <Check className="h-4 w-4 text-accent" />
                  ) : undefined
                }
                onClick={() => {
                  onSelect(opt.id);
                  setOpen(false);
                }}
              />
            ))
          )}
        </div>
      </BottomMenu>
    </section>
  );
}
