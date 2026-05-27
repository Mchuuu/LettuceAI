import type { WidgetNode } from "../../../../../core/storage/schemas";
import { WidgetRenderer } from "./WidgetRenderer";

interface WidgetListProps {
  nodes: WidgetNode[];
  side: "left" | "right";
}

export function WidgetList({ nodes, side }: WidgetListProps) {
  if (nodes.length === 0) {
    return (
      <div
        className="flex h-full w-full items-start justify-center px-3 pt-8 text-[10px] uppercase tracking-[0.25em] text-fg/20"
        aria-label={`${side} widget area, empty`}
      >
        {/* Empty state — intentionally light. Edit UI lands next pass. */}
      </div>
    );
  }
  return (
    <div className="flex h-full w-full flex-col gap-3 overflow-y-auto px-3 py-4">
      {nodes.map((node) => (
        <WidgetRenderer key={node.id} node={node} />
      ))}
    </div>
  );
}
