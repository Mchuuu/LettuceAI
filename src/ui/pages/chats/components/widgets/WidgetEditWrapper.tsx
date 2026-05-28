import { Reorder, useDragControls } from "framer-motion";
import { GripVertical, Pencil, Trash2 } from "lucide-react";
import type { WidgetNode } from "../../../../../core/storage/schemas";
import { WidgetRenderer } from "./WidgetRenderer";

interface WidgetEditWrapperProps {
  node: WidgetNode;
  onEdit: () => void;
  onDelete: () => void;
}

export function WidgetEditWrapper({ node, onEdit, onDelete }: WidgetEditWrapperProps) {
  const controls = useDragControls();
  return (
    <Reorder.Item
      value={node}
      dragListener={false}
      dragControls={controls}
      dragMomentum={false}
      dragElastic={0}
      layout="position"
      initial={{ opacity: 0, y: 8, scale: 0.98 }}
      animate={{ opacity: 1, y: 0, scale: 1 }}
      exit={{ opacity: 0, scale: 0.96, transition: { duration: 0.14 } }}
      whileDrag={{ zIndex: 30, boxShadow: "0 16px 32px rgba(0,0,0,0.28)" }}
      transition={{ layout: { duration: 0.18, ease: "easeOut" }, duration: 0.2, ease: "easeOut" }}
      className="relative rounded-2xl border border-dashed border-fg/20 bg-fg/[0.02] p-2"
      style={{ position: "relative" }}
    >
      <div className="mb-1.5 flex items-center justify-between gap-2">
        <button
          type="button"
          onPointerDown={(e) => controls.start(e)}
          className="flex h-6 w-6 cursor-grab items-center justify-center rounded-md text-fg/40 transition hover:bg-fg/8 hover:text-fg/80 active:cursor-grabbing"
          aria-label="Drag to reorder"
        >
          <GripVertical size={14} />
        </button>
        <div className="flex items-center gap-0.5">
          <button
            type="button"
            onClick={onEdit}
            className="flex h-6 w-6 items-center justify-center rounded-md text-fg/50 transition hover:bg-fg/10 hover:text-fg"
            aria-label="Edit widget"
          >
            <Pencil size={13} strokeWidth={2.2} />
          </button>
          <button
            type="button"
            onClick={onDelete}
            className="flex h-6 w-6 items-center justify-center rounded-md text-fg/50 transition hover:bg-danger/15 hover:text-danger"
            aria-label="Delete widget"
          >
            <Trash2 size={13} strokeWidth={2.2} />
          </button>
        </div>
      </div>
      <div className="pointer-events-none select-none">
        <WidgetRenderer node={node} />
      </div>
    </Reorder.Item>
  );
}
