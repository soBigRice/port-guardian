import { SafetyLevel } from "../types";
import { useTranslation } from "../i18n";
import { CircleIcon } from "./icons";

interface Props {
  level: SafetyLevel;
}

export default function RiskBadge({ level }: Props) {
  const { t } = useTranslation();
  const labels: Record<SafetyLevel, string> = {
    safe: t("riskBadge.safe"),
    caution: t("riskBadge.caution"),
    danger: t("riskBadge.danger"),
    unknown: t("riskBadge.unknown"),
  };
  return <span className={`risk-indicator risk-${level}`}><CircleIcon size={9} weight="fill" aria-hidden="true" />{labels[level]}</span>;
}
