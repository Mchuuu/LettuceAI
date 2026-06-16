import { MissingModelRequirementsSheet } from "./MissingModelRequirementsSheet";
import type { ModelRequirement } from "../modelRequirements";
import { useI18n } from "../../core/i18n/context";

interface MissingCompanionModelsSheetProps {
  isOpen: boolean;
  missing: ModelRequirement[];
  onClose: () => void;
  onDownload: () => void;
}

export function MissingCompanionModelsSheet({
  isOpen,
  missing,
  onClose,
  onDownload,
}: MissingCompanionModelsSheetProps) {
  const { t } = useI18n();
  const count = missing.length;
  const subtitle =
    count === 1
      ? t("components.extra.companionSetupSubtitleSingle")
      : t("components.extra.companionSetupSubtitleMany", { count });

  return (
    <MissingModelRequirementsSheet
      isOpen={isOpen}
      title={t("components.extra.companionSetupTitle")}
      description={`${t("components.extra.companionSetupBody")} ${subtitle}`}
      missing={missing}
      onClose={onClose}
      onDownload={onDownload}
      closeLabel={t("components.extra.companionUseRoleplay")}
    />
  );
}
