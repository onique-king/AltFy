import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { Check, ChevronDown } from "lucide-react";
import "./CustomSelect.css";

export type CustomSelectOption = { value: string; label: string };
type Props = {
  value: string;
  options: CustomSelectOption[];
  onChange: (value: string) => void;
  className?: string;
  ariaLabel?: string;
};

export default function CustomSelect({ value, options, onChange, className = "", ariaLabel }: Props) {
  const [open, setOpen] = useState(false);
  const [menuPosition, setMenuPosition] = useState({ top: 0, left: 0, width: 0 });
  const root = useRef<HTMLDivElement>(null);
  const trigger = useRef<HTMLButtonElement>(null);
  const menu = useRef<HTMLDivElement>(null);
  const selected = options.find(option => option.value === value) ?? options[0];
  useLayoutEffect(() => {
    if (!open) return;
    const updatePosition = () => {
      const rect = trigger.current?.getBoundingClientRect();
      if (rect) setMenuPosition({ top: rect.bottom + 7, left: rect.left, width: rect.width });
    };
    updatePosition();
    window.addEventListener("resize", updatePosition);
    window.addEventListener("scroll", updatePosition, true);
    return () => { window.removeEventListener("resize", updatePosition); window.removeEventListener("scroll", updatePosition, true); };
  }, [open]);
  useEffect(() => {
    const close = (event: PointerEvent) => {
      const target = event.target as Node;
      if (!root.current?.contains(target) && !menu.current?.contains(target)) setOpen(false);
    };
    const key = (event: KeyboardEvent) => { if (event.key === "Escape") setOpen(false); if (event.key === "ArrowDown" || event.key === "ArrowUp") { if (!open) { event.preventDefault(); setOpen(true); } } };
    document.addEventListener("pointerdown", close);
    document.addEventListener("keydown", key);
    return () => { document.removeEventListener("pointerdown", close); document.removeEventListener("keydown", key); };
  }, [open]);
  const menuContent = open && createPortal(<div ref={menu} className="custom-select-menu is-open" role="listbox" aria-hidden={false} style={{ position: "fixed", top: menuPosition.top, left: menuPosition.left, width: menuPosition.width }}>
    {options.map(option => <button type="button" role="option" aria-selected={option.value === value} className={option.value === value ? "selected" : ""} key={option.value} onClick={() => { onChange(option.value); setOpen(false); }}>
      <span>{option.label}</span>{option.value === value && <Check size={15} />}
    </button>)}
  </div>, document.body);
  return <div className={`custom-select ${className}`} ref={root}>
    <button ref={trigger} type="button" className="custom-select-trigger" aria-label={ariaLabel} aria-haspopup="listbox" aria-expanded={open} onClick={() => setOpen(current => !current)}>
      <span>{selected?.label ?? value}</span><ChevronDown size={16} className={open ? "is-open" : ""} />
    </button>
    {menuContent}
  </div>;
}
