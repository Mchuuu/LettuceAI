import { createContext, useContext, type ReactNode } from "react";
import type {
  Character,
  Model,
  Persona,
  Session,
} from "../../../../../core/storage/schemas";

export interface WidgetActionContext {
  character: Character | null;
  persona: Persona | null;
  session: Session | null;
  personas: Persona[];
  models: Model[];
  currentModelId: string | null;
  fallbackModelId: string | null;
  swapPlacesActive: boolean;
  canRegenerate: boolean;
  onSelectPersona: (personaId: string | null) => void | Promise<void>;
  onSelectModel: (modelId: string) => void | Promise<void>;
  onSelectFallbackModel: (modelId: string | null) => void | Promise<void>;
  onRegenerate: () => void | Promise<void>;
  onToggleSwapPlaces: () => void;
  onNewSession: () => void | Promise<void>;
}

const WidgetContext = createContext<WidgetActionContext | null>(null);

export function WidgetContextProvider({
  value,
  children,
}: {
  value: WidgetActionContext;
  children: ReactNode;
}) {
  return <WidgetContext.Provider value={value}>{children}</WidgetContext.Provider>;
}

export function useWidgetContext(): WidgetActionContext {
  const ctx = useContext(WidgetContext);
  if (!ctx) {
    throw new Error("useWidgetContext used outside WidgetContextProvider");
  }
  return ctx;
}
