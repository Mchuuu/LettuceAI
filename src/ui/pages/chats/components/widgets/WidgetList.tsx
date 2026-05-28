import { useState } from "react";
import { AnimatePresence, motion, Reorder } from "framer-motion";
import { Check, Pencil, Plus, RotateCcw } from "lucide-react";
import type { WidgetNode } from "../../../../../core/storage/schemas";
import { WidgetRenderer } from "./WidgetRenderer";
import { WidgetEditWrapper } from "./WidgetEditWrapper";
import { WidgetEmptyState } from "./WidgetEmptyState";
import { useWidgetEdit, type WidgetSide } from "./WidgetEditContext";
import { WidgetTypePickerSheet } from "./editor/WidgetTypePickerSheet";
import { WidgetConfigSheet } from "./editor/WidgetConfigSheet";
import { createWidgetNode } from "./editor/widgetFactories";

interface WidgetListProps {
  nodes: WidgetNode[];
  side: WidgetSide;
}

export function WidgetList({ nodes, side }: WidgetListProps) {
  const edit = useWidgetEdit();
  const [pickerOpen, setPickerOpen] = useState(false);
  const [editingNode, setEditingNode] = useState<WidgetNode | null>(null);

  const displayNodes = edit.editing ? edit.getNodes(side) : nodes;

  return (
    <div className="flex h-full w-full flex-col overflow-y-auto">
      <Toolbar
        editing={edit.editing}
        saving={edit.saving}
        onEnter={edit.enterEdit}
        onAdd={() => setPickerOpen(true)}
        onRevert={edit.revert}
        onDone={edit.done}
      />

      {edit.editing ? (
        <Reorder.Group
          axis="y"
          values={displayNodes}
          onReorder={(next) => edit.setNodes(side, next)}
          className="flex min-h-0 flex-1 flex-col gap-3 px-3 pb-4"
        >
          <AnimatePresence initial={false}>
            {displayNodes.map((node) => (
              <WidgetEditWrapper
                key={node.id}
                node={node}
                onEdit={() => setEditingNode(node)}
                onDelete={() => edit.removeNode(side, node.id)}
              />
            ))}
          </AnimatePresence>
          {displayNodes.length === 0 && (
            <WidgetEmptyState editing onAdd={() => setPickerOpen(true)} />
          )}
        </Reorder.Group>
      ) : displayNodes.length === 0 ? (
        <AnimatePresence>
          <WidgetEmptyState key="empty" editing={false} />
        </AnimatePresence>
      ) : (
        <div className="flex flex-col gap-3 px-3 pb-4">
          <AnimatePresence initial={false} mode="popLayout">
            {displayNodes.map((node) => (
              <motion.div
                key={node.id}
                layout
                initial={{ opacity: 0, y: 8 }}
                animate={{ opacity: 1, y: 0 }}
                exit={{ opacity: 0, y: -6, transition: { duration: 0.14 } }}
                transition={{ duration: 0.22, ease: "easeOut" }}
              >
                <WidgetRenderer node={node} />
              </motion.div>
            ))}
          </AnimatePresence>
        </div>
      )}

      <WidgetTypePickerSheet
        open={pickerOpen}
        onClose={() => setPickerOpen(false)}
        onPick={(type) => edit.addNode(side, createWidgetNode(type))}
      />
      <WidgetConfigSheet
        open={editingNode !== null}
        node={editingNode}
        onClose={() => setEditingNode(null)}
        onSave={(next) => edit.updateNode(side, next)}
      />
    </div>
  );
}

interface ToolbarProps {
  editing: boolean;
  saving: boolean;
  onEnter: () => void;
  onAdd: () => void;
  onRevert: () => void;
  onDone: () => void;
}

function Toolbar({ editing, saving, onEnter, onAdd, onRevert, onDone }: ToolbarProps) {
  return (
    <div className="flex min-h-[44px] items-center justify-between gap-2 px-2 py-2">
      <AnimatePresence mode="wait" initial={false}>
        {editing ? (
          <motion.div
            key="edit"
            initial={{ opacity: 0, y: -4 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: -4 }}
            transition={{ duration: 0.16, ease: "easeOut" }}
            className="flex w-full items-center justify-between gap-2"
          >
            <button
              type="button"
              onClick={onAdd}
              className="flex items-center gap-1 rounded-md border border-accent/30 bg-accent/10 px-2 py-1 text-[11px] font-medium text-accent transition hover:bg-accent/20"
            >
              <Plus size={12} strokeWidth={2.4} />
              Add
            </button>
            <div className="flex items-center gap-1">
              <button
                type="button"
                onClick={onRevert}
                disabled={saving}
                className="flex items-center gap-1 rounded-md border border-fg/10 bg-fg/5 px-2 py-1 text-[11px] text-fg/65 transition hover:bg-fg/10 disabled:opacity-50"
              >
                <RotateCcw size={11} strokeWidth={2.2} />
                Revert
              </button>
              <button
                type="button"
                onClick={onDone}
                disabled={saving}
                className="flex items-center gap-1 rounded-md border border-accent/40 bg-accent/15 px-2 py-1 text-[11px] font-medium text-accent transition hover:bg-accent/25 disabled:opacity-50"
              >
                <Check size={12} strokeWidth={2.4} />
                Done
              </button>
            </div>
          </motion.div>
        ) : (
          <motion.div
            key="view"
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            transition={{ duration: 0.16, ease: "easeOut" }}
            className="flex w-full justify-end"
          >
            <button
              type="button"
              onClick={onEnter}
              className="flex h-7 w-7 items-center justify-center rounded-full border border-fg/10 bg-fg/5 text-fg/45 transition hover:bg-fg/10 hover:text-fg/80"
              aria-label="Edit widgets"
            >
              <Pencil size={13} strokeWidth={2.2} />
            </button>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}
