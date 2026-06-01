interface Props {
  value: string;
  onChange: (v: string) => void;
}

export default function SearchBar({ value, onChange }: Props) {
  return (
    <input
      className="search-input"
      type="text"
      placeholder="搜索端口、进程名、命令、目录..."
      value={value}
      onChange={(e) => onChange(e.target.value)}
    />
  );
}
