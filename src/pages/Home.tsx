import { useEffect, useState } from "react";
import { ArrowRight, ChevronRight, Globe2, Pencil, Power, Radio, Send, ShieldCheck, Sparkles } from "lucide-react";
import WorldMap from "../components/WorldMap";

import type { VpnServer } from "./VpnPage";
import type { DpiStrategy } from "./DpiPage";
type Props = { onVpn: () => void; vpnServer: VpnServer | null; dpiStrategies: DpiStrategy[]; onDpiStrategiesChange: (items: DpiStrategy[]) => void; activeDpiId: string; onActiveDpiChange: (id: string) => void; dpiEnabled: boolean; onDpiEnabledChange: (enabled: boolean) => void; tgEnabled: boolean; onTgEnabledChange: (enabled: boolean) => void };

export default function Home({ onVpn, vpnServer, dpiStrategies, onDpiStrategiesChange, activeDpiId, onActiveDpiChange, dpiEnabled: dpi, onDpiEnabledChange, tgEnabled: telegram, onTgEnabledChange }: Props) {
  const [connected, setConnected] = useState(false);
  const [startedAt, setStartedAt] = useState<number | null>(null);
  const [seconds, setSeconds] = useState(0);
  const [drawer, setDrawer] = useState(false);
  const [drawerMounted, setDrawerMounted] = useState(false);
  const [drawerVisible, setDrawerVisible] = useState(false);

  useEffect(() => {
    if (drawer) {
      setDrawerMounted(true);
      let secondFrame = 0;
      const frame = requestAnimationFrame(() => { secondFrame = requestAnimationFrame(() => setDrawerVisible(true)); });
      return () => { cancelAnimationFrame(frame); cancelAnimationFrame(secondFrame); };
    }
    setDrawerVisible(false);
    const timeout = window.setTimeout(() => setDrawerMounted(false), 300);
    return () => window.clearTimeout(timeout);
  }, [drawer]);
  const presets: DpiStrategy[] = dpiStrategies.length ? dpiStrategies : [
    { id: "zapret", name: "Способ 1", engine: "Zapret", ping: 42, services: 14, args: "" },
    { id: "zapret-alt", name: "Способ 2", engine: "Zapret Alt", ping: 58, services: 13, args: "" },
    { id: "byedpi", name: "Способ 3", engine: "ByeDPI", ping: 71, services: 12, args: "" },
  ];
  const selected = Math.max(0, presets.findIndex((strategy) => strategy.id === activeDpiId));
  const [editing, setEditing] = useState(false);
  const [name, setName] = useState("");

  useEffect(() => {
    if (startedAt === null) return;
    const update = () => setSeconds(Math.floor((Date.now() - startedAt) / 1000));
    update();
    const interval = window.setInterval(update, 1000);
    return () => window.clearInterval(interval);
  }, [startedAt]);

  const toggleConnection = () => {
    if (connected) { setConnected(false); setStartedAt(null); setSeconds(0); }
    else { setConnected(true); setStartedAt(Date.now()); }
  };
  const time = [Math.floor(seconds / 3600), Math.floor(seconds / 60) % 60, seconds % 60].map((part) => String(part).padStart(2, "0")).join(":");
  const saveName = () => {
    const value = name.trim();
    if (value) {
      if (dpiStrategies.length) {
        const updated = dpiStrategies.map((item) => item.id === activeDpiId ? { ...item, name: value } : item);
        onDpiStrategiesChange(updated);
      } else onDpiStrategiesChange(presets.map((item, index) => index === selected ? { ...item, name: value } : item));
    }
    setEditing(false);
  };

  return <>
    <style>{`
      .home-card { transition: transform 300ms ease-out, border-color 500ms, box-shadow 500ms, background-color 500ms; }
      .home-card:hover { transform: translateY(-2px); }
      .home-card:active { transform: scale(.97); }
      .home-card.home-card-active { border-color: rgba(139, 92, 246, .55); box-shadow: 0 0 20px rgba(139, 92, 246, .15); background-image: linear-gradient(135deg, rgba(139, 92, 246, .12), transparent 70%); }
      .home-card .switch span { transition: transform 300ms ease-out; }
      .home-status-dot { display: inline-block; width: 6px; height: 6px; margin-right: 5px; border-radius: 50%; background: #8b5cf6; opacity: 0; transform: scale(.6); transition: opacity 500ms, transform 500ms; }
      .home-status-dot.is-active { opacity: 1; transform: scale(1); }
      .home-value { display: inline-block; animation: home-value-in 300ms ease-out both; }
      @keyframes home-value-in { from { opacity: 0; transform: scale(.97); } to { opacity: 1; transform: scale(1); } }
      @media (prefers-reduced-motion: reduce) { .home-card, .home-card .switch span, .home-status-dot { transition: none; } .home-value { animation: none; } }
    `}</style>
    <section className="hero">
      <WorldMap />
      <div className="hero-content">
        <div className="eyebrow"><span className={connected ? "status-dot active" : "status-dot"} /> {connected ? "СОЕДИНЕНИЕ АКТИВНО" : "ВАШЕ ПОДКЛЮЧЕНИЕ"}</div>
        <button className={`power-button ${connected ? "connected" : ""}`} onClick={toggleConnection} aria-label={connected ? "Отключить" : "Подключить"} aria-pressed={connected}><Power size={39} strokeWidth={1.7} /></button>
        <h1>{connected ? "Подключено" : "Отключено"}</h1>
        <p>{connected ? time : "Нажмите, чтобы подключиться"}</p>
      </div>
    </section>
    <div className="section-heading"><span>УПРАВЛЕНИЕ</span><span>ВАШИ СЕРВИСЫ</span></div>
    <section className="cards">
      <div className={`card service-card home-card transition-all duration-300 ease-out active:scale-[0.97] hover:-translate-y-0.5 ${dpi ? "home-card-active" : ""}`}>
        <div className="card-top"><span className="icon-box"><ShieldCheck size={18} /></span><button className={`switch ${dpi ? "on" : ""}`} role="switch" aria-checked={dpi} aria-label="DPI Обход" onClick={() => onDpiEnabledChange(!dpi)}><span /></button></div>
        <h2>DPI Обход</h2><p className="card-status"><span className={`home-status-dot ${dpi ? "is-active" : ""}`} /><span className="home-value" key={String(dpi)}>{dpi ? "Активен" : "Не активен"}</span></p>
        <div className="strategy-row">
          <button className="strategy-select" onClick={() => setDrawer(true)} title="Выбрать стратегию"><span className="home-value" key={presets[selected].name}>{presets[selected].name}</span> <ChevronRight size={14} /></button>
          <button className="edit-button" aria-label="Переименовать стратегию" onClick={() => { setName(presets[selected].name); setEditing(true); }}><Pencil size={13} /></button>
        </div>
        <div className="card-status" style={{ whiteSpace: "nowrap" }}><span className="home-value" key={`${presets[selected].ping}-${presets[selected].services}`}>{presets[selected].ping} мс · {presets[selected].services}/14 сервисов</span></div>
      </div>
      <div className={`card service-card home-card transition-all duration-300 ease-out active:scale-[0.97] hover:-translate-y-0.5 ${telegram ? "home-card-active" : ""}`}>
        <div className="card-top"><span className="icon-box telegram-icon"><Send size={17} /></span><button className={`switch ${telegram ? "on" : ""}`} role="switch" aria-checked={telegram} aria-label="TG-WS Proxy" onClick={() => onTgEnabledChange(!telegram)}><span /></button></div>
        <h2>Telegram</h2><p className="card-status"><span className={`home-status-dot ${telegram ? "is-active" : ""}`} /><span className="home-value" key={String(telegram)}>{telegram ? "Прокси включён" : "Прокси выключен"}</span></p>
        <div className="telegram-label"><Radio size={13} /> TG-WS Proxy</div>
      </div>
      <div className="card vpn-card home-card transition-all duration-300 ease-out active:scale-[0.97] hover:-translate-y-0.5">
        <button className="vpn-main" onClick={onVpn}><span className="icon-box"><Globe2 size={18} /></span><span className="vpn-copy"><strong>VPN Сервер</strong><small>{vpnServer?.name || "Отсутствует"}</small></span><ChevronRight size={17} /></button>
        <button className="promo" onClick={onVpn}><Sparkles size={15} /><span>Попробовать наш VPN 3 дня бесплатно</span><ArrowRight size={15} /></button>
      </div>
    </section>
    {drawerMounted && <div className="overlay backdrop-blur-sm bg-black/40 transition-opacity duration-300" style={{ opacity: drawerVisible ? 1 : 0, pointerEvents: drawerVisible ? "auto" : "none", backgroundColor: "rgba(0, 0, 0, .4)", backdropFilter: "blur(4px)", transition: "opacity 300ms ease-out" }} onClick={() => setDrawer(false)}><div className="drawer" style={{ transform: drawerVisible ? "translateY(0)" : "translateY(100%)", transition: "transform 300ms ease-out" }} role="dialog" aria-modal="true" aria-label="Выбор стратегии" onClick={(e) => e.stopPropagation()}><div className="drawer-handle" /><div className="dialog-heading"><div><h2>Стратегия DPI</h2><p>Выберите способ обхода</p></div><button className="close-button" onClick={() => setDrawer(false)} aria-label="Закрыть">×</button></div><button className="strategy-option" onClick={() => { onActiveDpiChange(presets[0].id); setDrawer(false); }}><Sparkles size={16} /> Автоподбор лучшего способа</button>{presets.map((strategy) => <button key={strategy.id} className="strategy-option" onClick={() => { onActiveDpiChange(strategy.id); setDrawer(false); }}><span>{strategy.name} • {strategy.engine}<br /><small>{strategy.ping} мс · {strategy.services}/14 сервисов</small></span><span className={`radio ${activeDpiId === strategy.id ? "selected" : ""}`} /></button>)}</div></div>}
    {editing && <div className="overlay" onClick={() => setEditing(false)}><form className="dialog" role="dialog" aria-modal="true" aria-label="Переименовать стратегию" onClick={(e) => e.stopPropagation()} onSubmit={(e) => { e.preventDefault(); saveName(); }}><h2>Название стратегии</h2><input autoFocus value={name} maxLength={40} onChange={(e) => setName(e.target.value)} /><div className="dialog-actions"><button type="button" onClick={() => setEditing(false)}>Отмена</button><button type="submit" className="primary-action">Сохранить</button></div></form></div>}
  </>;
}
