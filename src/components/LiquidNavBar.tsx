import { useEffect, useRef, useState, type PointerEvent } from "react";
import { CircleHelp, Home, LockKeyhole, Send, ShieldCheck } from "lucide-react";

const tabs = [
  { id: "home", label: "Главная", icon: Home },
  { id: "vpn", label: "VPN", icon: LockKeyhole },
  { id: "dpi", label: "DPI", icon: ShieldCheck },
  { id: "tg", label: "TG-WS", icon: Send },
  { id: "info", label: "Инфо", icon: CircleHelp },
] as const;

export type Tab = typeof tabs[number]["id"];

type Props = { tab: Tab; onChange: (tab: Tab) => void };

export default function LiquidNavBar({ tab, onChange }: Props) {
  const selected = tabs.findIndex((item) => item.id === tab);
  const target = useRef(selected);
  const position = useRef(selected);
  const pointer = useRef<number | null>(null);
  const [visual, setVisual] = useState({ position: selected, velocity: 0 });

  useEffect(() => {
    if (pointer.current === null) target.current = selected;
  }, [selected]);

  useEffect(() => {
    let frame: number;
    const animate = () => {
      const difference = target.current - position.current;
      const velocity = Math.abs(difference) < 0.001 ? difference : difference * 0.11;
      if (velocity !== 0) {
        position.current += velocity;
        setVisual({ position: position.current, velocity });
      } else {
        setVisual((previous) => previous.velocity === 0 ? previous : { ...previous, velocity: 0 });
      }
      frame = requestAnimationFrame(animate);
    };
    frame = requestAnimationFrame(animate);
    return () => cancelAnimationFrame(frame);
  }, []);

  const positionFromEvent = (event: PointerEvent<HTMLElement>) => {
    const bounds = event.currentTarget.getBoundingClientRect();
    const width = bounds.width - 12;
    return Math.max(0, Math.min(tabs.length - 1, (event.clientX - bounds.left - 6) / width * tabs.length - 0.5));
  };

  return <nav className="navigation overflow-hidden rounded-full bg-white/85 dark:bg-[#181B21]/90 border border-black/[0.06] dark:border-white/[0.08] backdrop-blur-xl touch-none" aria-label="Основная навигация"
    onPointerDown={(event) => {
      if (event.button !== 0 || pointer.current !== null) return;
      pointer.current = event.pointerId;
      event.currentTarget.setPointerCapture(event.pointerId);
      target.current = positionFromEvent(event);
    }}
    onPointerMove={(event) => { if (pointer.current === event.pointerId) target.current = positionFromEvent(event); }}
    onPointerUp={(event) => {
      if (pointer.current !== event.pointerId) return;
      const index = Math.round(positionFromEvent(event));
      pointer.current = null;
      target.current = index;
      onChange(tabs[index].id);
    }}
    onPointerCancel={(event) => { if (pointer.current === event.pointerId) { pointer.current = null; target.current = selected; } }}
    onLostPointerCapture={(event) => { if (pointer.current === event.pointerId) { pointer.current = null; target.current = selected; } }}>
    <div className="nav-indicator" aria-hidden="true" style={{ transform: `translateX(${visual.position * 100}%)` }}>
      <div className="nav-indicator-glass rounded-full bg-gradient-to-b from-white/95 via-violet-500/[0.1] to-slate-900/[0.08] border border-slate-900/15 dark:from-white/[0.16] dark:via-violet-500/[0.12] dark:to-white/[0.04] dark:border-white/20 shadow-[inset_0_1px_1px_rgba(255,255,255,0.9),0_2px_8px_rgba(139,92,246,0.18)] dark:shadow-[inset_0_1.5px_1px_rgba(255,255,255,0.9),0_2px_6px_rgba(139,92,246,0.15)]" style={{ transform: `scaleX(${1 + Math.min(Math.abs(visual.velocity) * 0.45, 0.35)})` }} />
    </div>
    {tabs.map(({ id, label, icon: Icon }, index) => {
      const proximity = Math.max(0, 1 - Math.abs(visual.position - index));
      const color = `rgb(${Math.round(170 + (139 - 170) * proximity)}, ${Math.round(166 + (92 - 166) * proximity)}, ${Math.round(183 + (246 - 183) * proximity)})`;
      return <button key={id} type="button" className="nav-item flex-1 h-14 flex flex-col items-center justify-center gap-1" style={{ color }} onClick={(event) => { if (event.detail === 0) { target.current = index; onChange(id); } }} aria-current={tab === id ? "page" : undefined}>
        <span className="w-6 h-6 flex items-center justify-center" style={{ transform: `translateX(${Math.max(-3, Math.min(3, (target.current - visual.position) * proximity * 2.5))}px) scaleX(${1 + Math.min(Math.abs(target.current - visual.position) * proximity * 0.12, 0.12)}) scale(${1 + proximity * 0.1})` }}><Icon size={19} strokeWidth={1.8 + proximity * 0.5} /></span>
        <span>{label}</span>
      </button>;
    })}
  </nav>;
}
