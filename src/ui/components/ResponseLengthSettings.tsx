import type { AdvancedModelSettings } from "../../core/storage/schemas";
import { useI18n, type TranslationKey } from "../../core/i18n/context";
import { cn } from "../design-tokens";
import { ADVANCED_RESPONSE_LENGTH_RANGE } from "./AdvancedModelSettingsForm";
import { NumberInput } from "./NumberInput";

type ResponseLengthPreset = NonNullable<AdvancedModelSettings["responseLengthPreset"]>;

interface ResponseLengthSettingsProps {
  settings: AdvancedModelSettings;
  onChange: (settings: AdvancedModelSettings) => void;
  disabled?: boolean;
  className?: string;
}

const PRESETS: Array<{ value: ResponseLengthPreset; labelKey: TranslationKey }> = [
  { value: "auto", labelKey: "components.advancedModelSettings.responseLengthAuto" },
  { value: "short", labelKey: "components.advancedModelSettings.responseLengthShort" },
  { value: "medium", labelKey: "components.advancedModelSettings.responseLengthMedium" },
  { value: "long", labelKey: "components.advancedModelSettings.responseLengthLong" },
  { value: "custom", labelKey: "components.advancedModelSettings.responseLengthCustom" },
];

export function ResponseLengthSettings({
  settings,
  onChange,
  disabled = false,
  className,
}: ResponseLengthSettingsProps) {
  const { t } = useI18n();
  const selected = settings.responseLengthPreset ?? "auto";

  return (
    <div className={cn("space-y-3", className)}>
      <div>
        <p className="text-[13px] font-medium text-fg/75">
          {t("components.advancedModelSettings.responseLength")}
        </p>
        <p className="mt-0.5 text-[11px] leading-relaxed text-fg/40">
          {t("components.advancedModelSettings.responseLengthDesc")}
        </p>
      </div>

      <div className="grid grid-cols-5 gap-1.5">
        {PRESETS.map((option) => (
          <button
            key={option.value}
            type="button"
            disabled={disabled}
            onClick={() =>
              onChange({
                ...settings,
                responseLengthPreset: option.value,
                responseLengthChars:
                  option.value === "custom" ? (settings.responseLengthChars ?? 80) : null,
              })
            }
            className={cn(
              "min-h-9 rounded-lg border px-1 text-[11px] font-medium transition disabled:opacity-50",
              selected === option.value
                ? "border-accent/40 bg-accent/15 text-accent"
                : "border-fg/10 bg-fg/[0.03] text-fg/55 hover:bg-fg/[0.07]",
            )}
          >
            {t(option.labelKey)}
          </button>
        ))}
      </div>

      {selected === "custom" && (
        <div className="flex items-center gap-3">
          <NumberInput
            min={ADVANCED_RESPONSE_LENGTH_RANGE.min}
            max={ADVANCED_RESPONSE_LENGTH_RANGE.max}
            step={1}
            value={settings.responseLengthChars ?? 80}
            onChange={(value) =>
              onChange({
                ...settings,
                responseLengthPreset: "custom",
                responseLengthChars: value == null ? 80 : Math.round(value),
              })
            }
            disabled={disabled}
            className="min-w-0 flex-1 rounded-lg border border-fg/10 bg-fg/[0.04] px-3 py-2 text-sm text-fg focus:border-fg/25 focus:outline-none"
          />
          <span className="shrink-0 text-xs text-fg/45">
            {t("components.advancedModelSettings.responseLengthUnit")}
          </span>
        </div>
      )}
    </div>
  );
}
