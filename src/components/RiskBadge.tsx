import { SafetyLevel } from "../types";
import { useTranslation } from "../i18n";

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
  return <span className={`badge badge-${level}`}>{labels[level]}</span>;
}
