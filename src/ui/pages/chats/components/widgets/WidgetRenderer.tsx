import type { WidgetNode } from "../../../../../core/storage/schemas";
import { WidgetDivider } from "./WidgetDivider";
import { WidgetBox } from "./WidgetBox";
import { WidgetScratchPad } from "./WidgetScratchPad";
import { WidgetCharacterInfo } from "./WidgetCharacterInfo";
import { WidgetPersonaInfo } from "./WidgetPersonaInfo";
import { WidgetImage } from "./WidgetImage";
import { WidgetButton } from "./WidgetButton";
import { WidgetSelector } from "./WidgetSelector";

interface WidgetRendererProps {
  node: WidgetNode;
}

export function WidgetRenderer({ node }: WidgetRendererProps) {
  switch (node.type) {
    case "divider":
      return <WidgetDivider node={node} />;
    case "box":
      return <WidgetBox node={node} />;
    case "scratch_pad":
      return <WidgetScratchPad node={node} />;
    case "character_info":
      return <WidgetCharacterInfo />;
    case "persona_info":
      return <WidgetPersonaInfo />;
    case "image":
      return <WidgetImage node={node} />;
    case "selector":
      return <WidgetSelector node={node} />;
    case "button":
      return <WidgetButton node={node} />;
  }
}
