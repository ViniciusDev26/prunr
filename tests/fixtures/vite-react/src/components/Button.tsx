import { formatCount } from "/src/lib/format";
import missingIcon from "/gone.svg";

export function Button({ label }: { label: string }) {
  return (
    <button type="button">
      <img src={missingIcon} alt="" />
      {label}: {formatCount(0)}
    </button>
  );
}

export const unusedSize = "small";
