import { forwardRef } from "react";
import { useTranslation } from "../i18n";

interface Props {
  value: string;
  onChange: (v: string) => void;
}

const SearchBar = forwardRef<HTMLInputElement, Props>(({ value, onChange }, ref) => {
  const { t } = useTranslation();
  return (
    <input
      ref={ref}
      className="search-input"
      type="text"
      placeholder={t("searchBar.placeholder")}
      value={value}
      onChange={(e) => onChange(e.target.value)}
    />
  );
});

export default SearchBar;
