import { z } from "zod";

export type BoxVariant =
  | "default"
  | "subtle"
  | "info"
  | "warning"
  | "success"
  | "danger";

export type SelectorKind = "persona" | "model" | "fallback_model";

export type ButtonAction = "regenerate" | "swap_places" | "new_session";

export type ImageSource =
  | { kind: "character_avatar" }
  | { kind: "persona_avatar" }
  | { kind: "library"; path: string }
  | { kind: "upload"; path: string };

interface NodeBase {
  id: string;
}

export interface DividerNode extends NodeBase {
  type: "divider";
  style?: "line" | "space";
}

export interface BoxNode extends NodeBase {
  type: "box";
  variant?: BoxVariant;
  title?: string;
  description?: string;
  children: WidgetNode[];
}

export interface CharacterInfoNode extends NodeBase {
  type: "character_info";
}

export interface PersonaInfoNode extends NodeBase {
  type: "persona_info";
}

export interface ScratchPadNode extends NodeBase {
  type: "scratch_pad";
  title?: string;
  description?: string;
  content?: string;
}

export interface ImageNode extends NodeBase {
  type: "image";
  title?: string;
  description?: string;
  source: ImageSource;
}

export interface SelectorNode extends NodeBase {
  type: "selector";
  kind: SelectorKind;
  title?: string;
  description?: string;
}

export interface ButtonNode extends NodeBase {
  type: "button";
  action: ButtonAction;
  title?: string;
  description?: string;
}

export type WidgetNode =
  | DividerNode
  | BoxNode
  | CharacterInfoNode
  | PersonaInfoNode
  | ScratchPadNode
  | ImageNode
  | SelectorNode
  | ButtonNode;

const imageSourceSchema: z.ZodType<ImageSource> = z.union([
  z.object({ kind: z.literal("character_avatar") }),
  z.object({ kind: z.literal("persona_avatar") }),
  z.object({ kind: z.literal("library"), path: z.string() }),
  z.object({ kind: z.literal("upload"), path: z.string() }),
]);

export const widgetNodeSchema: z.ZodType<WidgetNode> = z.lazy(() =>
  z.discriminatedUnion("type", [
    z.object({
      id: z.string(),
      type: z.literal("divider"),
      style: z.enum(["line", "space"]).optional(),
    }),
    z.object({
      id: z.string(),
      type: z.literal("box"),
      variant: z
        .enum(["default", "subtle", "info", "warning", "success", "danger"])
        .optional(),
      title: z.string().optional(),
      description: z.string().optional(),
      children: z.array(widgetNodeSchema),
    }),
    z.object({
      id: z.string(),
      type: z.literal("character_info"),
    }),
    z.object({
      id: z.string(),
      type: z.literal("persona_info"),
    }),
    z.object({
      id: z.string(),
      type: z.literal("scratch_pad"),
      title: z.string().optional(),
      description: z.string().optional(),
      content: z.string().optional(),
    }),
    z.object({
      id: z.string(),
      type: z.literal("image"),
      title: z.string().optional(),
      description: z.string().optional(),
      source: imageSourceSchema,
    }),
    z.object({
      id: z.string(),
      type: z.literal("selector"),
      kind: z.enum(["persona", "model", "fallback_model"]),
      title: z.string().optional(),
      description: z.string().optional(),
    }),
    z.object({
      id: z.string(),
      type: z.literal("button"),
      action: z.enum(["regenerate", "swap_places", "new_session"]),
      title: z.string().optional(),
      description: z.string().optional(),
    }),
  ]),
);
