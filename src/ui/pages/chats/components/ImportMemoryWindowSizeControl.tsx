import { cn, radius } from "../../../design-tokens";
import { useI18n } from "../../../../core/i18n/context";

export const IMPORT_MEMORY_WINDOW_MIN = 50;
export const IMPORT_MEMORY_WINDOW_MAX = 200;
export const IMPORT_MEMORY_WINDOW_DEFAULT = 100;

export function clampImportMemoryWindowSize(value: number) {
  if (!Number.isFinite(value)) return IMPORT_MEMORY_WINDOW_DEFAULT;
  return Math.max(
    IMPORT_MEMORY_WINDOW_MIN,
    Math.min(IMPORT_MEMORY_WINDOW_MAX, Math.round(value)),
  );
}

interface ImportMemoryWindowSizeControlProps {
  value: number;
  onChange: (value: number) => void;
  disabled?: boolean;
}

export function ImportMemoryWindowSizeControl({
  value,
  onChange,
  disabled = false,
}: ImportMemoryWindowSizeControlProps) {
  const { t } = useI18n();
  const normalized = clampImportMemoryWindowSize(value);

  const handleChange = (raw: string) => {
    const next = Number(raw);
    if (!Number.isNaN(next)) {
      onChange(clampImportMemoryWindowSize(next));
    }
  };

  return (
    <div className={cn(radius.md, "border border-fg/10 bg-fg/4 p-3")}>
      <div className="flex items-center justify-between gap-3">
        <div className="min-w-0">
          <div className="text-sm font-medium text-fg">
            {t("chats.settings.importMemoryWindowSize")}
          </div>
          <div className="mt-0.5 text-xs leading-relaxed text-fg/55">
            {t("chats.settings.importMemoryWindowSizeDesc")}
          </div>
        </div>
        <input
          type="number"
          min={IMPORT_MEMORY_WINDOW_MIN}
          max={IMPORT_MEMORY_WINDOW_MAX}
          step={10}
          value={normalized}
          onChange={(event) => handleChange(event.target.value)}
          disabled={disabled}
          className="h-9 w-20 rounded-lg border border-fg/10 bg-fg/5 px-2 text-center text-sm text-fg outline-none transition focus:border-fg/25 disabled:opacity-50"
        />
      </div>
      <input
        type="range"
        min={IMPORT_MEMORY_WINDOW_MIN}
        max={IMPORT_MEMORY_WINDOW_MAX}
        step={10}
        value={normalized}
        onChange={(event) => handleChange(event.target.value)}
        disabled={disabled}
        className="mt-3 w-full accent-emerald-400 disabled:opacity-50"
      />
    </div>
  );
}
