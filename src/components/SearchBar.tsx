import { forwardRef } from "react";
import { useTranslation } from "../i18n";
import { SearchIcon } from "./icons";

interface Props {
  value: string;
  onChange: (v: string) => void;
}

const SearchBar = forwardRef<HTMLInputElement, Props>(({ value, onChange }, ref) => {
  const { t } = useTranslation();
  return (
    <div className="workspace-search">
    <SearchIcon size={19} aria-hidden="true" />
    <input
      ref={ref}
      className="search-input"
      type="text"
      placeholder={t("searchBar.placeholder")}
      aria-label={t("searchBar.placeholder")}
      value={value}
      onChange={(e) => onChange(e.target.value)}
    />
    </div>
  );
});

export default SearchBar;
