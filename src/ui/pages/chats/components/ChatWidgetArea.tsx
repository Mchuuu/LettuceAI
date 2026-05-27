import type { ReactNode } from "react";
import type { ChatWidgetLayout } from "../utils/chatWidgetLayout";
import type { WidgetNode } from "../../../../core/storage/schemas";
import { WidgetList } from "./widgets";

interface WidgetAreaPanelProps {
  side: "left" | "right";
  nodes: WidgetNode[];
}

function WidgetAreaPanel({ side, nodes }: WidgetAreaPanelProps) {
  return (
    <aside
      className="relative z-10 flex flex-1 basis-0 flex-col self-stretch"
      style={{ minWidth: 0 }}
      aria-label={`${side} widget area`}
    >
      <WidgetList nodes={nodes} side={side} />
    </aside>
  );
}

interface ChatWidgetAreaProps {
  widgetLayout: ChatWidgetLayout;
  leftNodes: WidgetNode[];
  rightNodes: WidgetNode[];
  children: ReactNode;
}

export function ChatWidgetArea({
  widgetLayout,
  leftNodes,
  rightNodes,
  children,
}: ChatWidgetAreaProps) {
  if (!widgetLayout.enabled || widgetLayout.columnPx == null) {
    return <>{children}</>;
  }
  return (
    <div className="relative z-10 flex min-h-0 flex-1 flex-row">
      {widgetLayout.showLeft && <WidgetAreaPanel side="left" nodes={leftNodes} />}
      <div
        className="flex shrink-0 flex-col"
        style={{ width: widgetLayout.columnPx, maxWidth: "100%" }}
      >
        {children}
      </div>
      {widgetLayout.showRight && <WidgetAreaPanel side="right" nodes={rightNodes} />}
    </div>
  );
}
