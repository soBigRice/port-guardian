import { useTranslation } from "../i18n";

interface Props {
  value: string;
  onChange: (v: string) => void;
}

export default function SearchBar({ value, onChange }: Props) {
  const { t } = useTranslation();
  return (
    <input
      className="search-input"
      type="text"
      placeholder={t("searchBar.placeholder")}
      value={value}
      onChange={(e) => onChange(e.target.value)}
    />
  );
}
