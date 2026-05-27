import type {
  BoxNode,
  BoxVariant,
} from "../../../../../core/storage/chatWidgetSchemas";
import { WidgetRenderer } from "./WidgetRenderer";

interface WidgetBoxProps {
  node: BoxNode;
}

const VARIANT_STYLES: Record<BoxVariant, string> = {
  default: "border-fg/12 bg-fg/4",
  subtle: "border-fg/8 bg-fg/2",
  info: "border-info/30 bg-info/8",
  warning: "border-warning/30 bg-warning/8",
  success: "border-accent/30 bg-accent/8",
  danger: "border-danger/30 bg-danger/8",
};

const VARIANT_TITLE: Record<BoxVariant, string> = {
  default: "text-fg/75",
  subtle: "text-fg/55",
  info: "text-info",
  warning: "text-warning",
  success: "text-accent",
  danger: "text-danger",
};

export function WidgetBox({ node }: WidgetBoxProps) {
  const variant: BoxVariant = node.variant ?? "default";
  return (
    <section
      className={`flex flex-col gap-2 rounded-2xl border px-3 py-3 ${VARIANT_STYLES[variant]}`}
    >
      {(node.title || node.description) && (
        <header className="flex flex-col gap-0.5">
          {node.title && (
            <h3 className={`text-sm font-semibold ${VARIANT_TITLE[variant]}`}>
              {node.title}
            </h3>
          )}
          {node.description && (
            <p className="text-[11px] leading-snug text-fg/45">{node.description}</p>
          )}
        </header>
      )}
      {node.children.length > 0 && (
        <div className="flex flex-col gap-2">
          {node.children.map((child) => (
            <WidgetRenderer key={child.id} node={child} />
          ))}
        </div>
      )}
    </section>
  );
}
