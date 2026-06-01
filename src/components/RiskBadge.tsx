import { SafetyLevel } from "../types";

const LABELS: Record<SafetyLevel, string> = {
  safe: "安全",
  caution: "谨慎",
  danger: "危险",
  unknown: "未知",
};

interface Props {
  level: SafetyLevel;
}

export default function RiskBadge({ level }: Props) {
  return <span className={`badge badge-${level}`}>{LABELS[level]}</span>;
}
