import { useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { invoke } from "@tauri-apps/api/core";
import { ArrowRight, ChevronRight, Globe2, Pencil, Power, Radio, Send, ShieldCheck, Sparkles } from "lucide-react";
import WorldMap from "../components/WorldMap";
import { useLanguage } from "../i18n/translations";

import { Flag, type VpnServer } from "./VpnPage";
import type { DpiStrategy } from "./DpiPage";
type Props = { onVpn: () => void; onVpnToggle: () => void; vpnConnected: boolean; isConnecting: boolean; vpnMode: "tun" | "proxy"; hasVpnItems: boolean; onDpi: () => void; vpnServer: VpnServer | null; dpiStrategies: DpiStrategy[]; onDpiStrategiesChange: (items: DpiStrategy[]) => void; activeDpiId: string; onActiveDpiChange: (id: string) => void; dpiEnabled: boolean; onDpiEnabledChange: (enabled: boolean) => void; tgEnabled: boolean; onTgEnabledChange: (enabled: boolean) => void };

export default function Home({ onVpn, onVpnToggle, vpnConnected: connected, isConnecting, vpnMode, hasVpnItems, onDpi, vpnServer, dpiStrategies, onDpiStrategiesChange, activeDpiId, onActiveDpiChange, dpiEnabled: dpi, onDpiEnabledChange, tgEnabled: telegram, onTgEnabledChange }: Props) {
  const { t } = useLanguage();
  const [startedAt, setStartedAt] = useState<number | null>(null);
  const [seconds, setSeconds] = useState(0);
  const [drawer, setDrawer] = useState(false);
  const [drawerMounted, setDrawerMounted] = useState(false);
  const [drawerVisible, setDrawerVisible] = useState(false);
  const [dpiState, setDpiState] = useState<{ running: boolean; preset: string | null; services_ok: number; services_total: number; has_configured_dpi: boolean; last_zapret_enabled: boolean; last_byedpi_enabled: boolean; selected_preset: string | null } | null>(null);
  const dpiPollPausedUntil = useRef(0);
  const [dpiSetupModal, setDpiSetupModal] = useState(false);
  useEffect(() => {
    let mounted = true;
    const refresh = async () => { if (Date.now() < dpiPollPausedUntil.current) return; try { const state = await invoke<{ running: boolean; preset: string | null; services_ok: number; services_total: number; has_configured_dpi: boolean; last_zapret_enabled: boolean; last_byedpi_enabled: boolean; selected_preset: string | null }>("get_dpi_state"); if (mounted && Date.now() >= dpiPollPausedUntil.current) setDpiState(state); } catch { if (mounted && Date.now() >= dpiPollPausedUntil.current) setDpiState(null); } };
    const onDpiToggle = () => { dpiPollPausedUntil.current = Date.now() + 2000; };
    window.addEventListener("altfy-dpi-toggle", onDpiToggle);
    void refresh();
    const timer = window.setInterval(refresh, 2000);
    return () => { mounted = false; window.clearInterval(timer); window.removeEventListener("altfy-dpi-toggle", onDpiToggle); };
  }, []);

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
    if (!connected) { setStartedAt(null); setSeconds(0); return; }
    setStartedAt(value => value ?? Date.now());
  }, [connected]);

  useEffect(() => {
    if (startedAt === null) return;
    const update = () => setSeconds(Math.floor((Date.now() - startedAt) / 1000));
    update();
    const interval = window.setInterval(update, 1000);
    return () => window.clearInterval(interval);
  }, [startedAt]);

  const toggleDpi = async () => {
    if (!dpi && !dpiState?.has_configured_dpi) { setDpiSetupModal(true); return; }
    dpiPollPausedUntil.current = Date.now() + 2000;
    onDpiEnabledChange(!dpi);
    setDpiState(current => current ? { ...current, running: !dpi } : current);
    try { await invoke("set_dpi_enabled", { enabled: !dpi }); }
    catch (error) { onDpiEnabledChange(dpi); setDpiState(current => current ? { ...current, running: dpi } : current); window.dispatchEvent(new CustomEvent("altfy-toast", { detail: String(error) })); }
  };
  const toggleConnection = onVpnToggle;
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
      .vpn-mode-badge { display: inline-flex; align-items: center; margin-top: 8px; padding: 4px 10px; border: 1px solid rgba(139, 92, 246, .28); border-radius: 999px; background: rgba(139, 92, 246, .1); color: #a78bfa; font-size: 11px; line-height: 1.3; }
      .home-value { display: inline-block; animation: home-value-in 300ms ease-out both; }
      @keyframes home-value-in { from { opacity: 0; transform: scale(.97); } to { opacity: 1; transform: scale(1); } }
      @media (prefers-reduced-motion: reduce) { .home-card, .home-card .switch span, .home-status-dot { transition: none; } .home-value { animation: none; } }
    `}</style>
    <section className="hero">
      <WorldMap />
      <div className="hero-content">
        <div className="eyebrow"><span className={connected ? "status-dot active" : "status-dot"} /> {connected ? t("СОЕДИНЕНИЕ АКТИВНО") : t("ВАШЕ ПОДКЛЮЧЕНИЕ")}</div>
        <button className={`power-button ${connected ? "connected" : ""}`} onClick={toggleConnection} disabled={isConnecting} aria-label={isConnecting ? t("Подключение...") : connected ? t("Отключить") : t("Подключить")} aria-pressed={connected}><Power size={39} strokeWidth={1.7} /></button>
        <h1>{isConnecting ? t("Подключение...") : connected ? t("Подключено") : t("Отключено")}</h1>
        <p>{isConnecting ? t("Запуск туннеля") : connected ? time : t("Нажмите, чтобы подключиться")}</p>
        {connected && <span className="vpn-mode-badge">Режим: {vpnMode === "tun" ? "TUN" : "Системный прокси"}</span>}
      </div>
    </section>
    <div className="section-heading"><span>{t("УПРАВЛЕНИЕ")}</span><span>{t("ВАШИ СЕРВИСЫ")}</span></div>
    <section className="cards">
      <div className={`card service-card home-card transition-all duration-300 ease-out active:scale-[0.97] hover:-translate-y-0.5 ${dpi ? "home-card-active" : ""}`}>
        <div className="card-top"><span className="icon-box"><ShieldCheck size={18} /></span><button className={`switch ${dpi ? "on" : ""}`} role="switch" aria-checked={dpi} aria-label="DPI Обход" onClick={() => void toggleDpi()}><span /></button></div>
        <h2>{t("DPI Обход")}</h2><p className="card-status"><span className={`home-status-dot ${dpi ? "is-active" : ""}`} /><span className="home-value" key={String(dpiState?.running ?? dpi)}>{(dpiState?.running ?? dpi) ? t("Активен") : t("Не активен")}</span></p>
        <div className="strategy-row">
          <button className="strategy-select" onClick={() => setDrawer(true)} title="Выбрать стратегию"><span className="home-value" key={presets[selected].name}>{dpiState?.preset || presets[selected].name}</span> <ChevronRight size={14} /></button>
          <button className="edit-button" aria-label="Переименовать стратегию" onClick={() => { setName(presets[selected].name); setEditing(true); }}><Pencil size={13} /></button>
        </div>
        <div className="card-status" style={{ whiteSpace: "nowrap" }}><span className="home-value" key={`${presets[selected].ping}-${presets[selected].services}`}>{presets[selected].ping} мс · {(dpiState?.services_ok || presets[selected].services)}/{dpiState?.services_total || 12} сервисов</span></div>
      </div>
      <div className={`card service-card home-card transition-all duration-300 ease-out active:scale-[0.97] hover:-translate-y-0.5 ${telegram ? "home-card-active" : ""}`}>
        <div className="card-top"><span className="icon-box telegram-icon"><Send size={17} /></span><button className={`switch ${telegram ? "on" : ""}`} role="switch" aria-checked={telegram} aria-label="TG-WS Proxy" onClick={() => onTgEnabledChange(!telegram)}><span /></button></div>
        <h2>Telegram</h2><p className="card-status"><span className={`home-status-dot ${telegram ? "is-active" : ""}`} /><span className="home-value" key={String(telegram)}>{telegram ? "Прокси включён" : "Прокси выключен"}</span></p>
        <div className="telegram-label"><Radio size={13} /> TG-WS Proxy</div>
      </div>
      <div className="card vpn-card home-card transition-all duration-300 ease-out active:scale-[0.97] hover:-translate-y-0.5">
        <button className="vpn-main" onClick={onVpn}>{vpnServer ? <Flag flag={vpnServer.flag} name={vpnServer.name} /> : <span className="w-10 h-10 rounded-full flex items-center justify-center overflow-hidden shrink-0"><Globe2 size={18} /></span>}<span className="vpn-copy"><strong>{t("VPN Сервер")}</strong><small>{vpnServer?.name || t("Отсутствует")}</small></span><ChevronRight size={17} /></button>
        {hasVpnItems ? <button className="w-full py-3 rounded-2xl bg-gradient-to-r from-violet-500 to-indigo-500 text-white font-semibold" onClick={toggleConnection} disabled={isConnecting}>{isConnecting ? t("Подключение...") : connected ? t("Отключить") : t("Подключить")}</button> : <button className="promo" onClick={onVpn}><Sparkles size={15} /><span>Попробовать наш VPN 3 дня бесплатно</span><ArrowRight size={15} /></button>}
      </div>
    </section>
    {drawerMounted && <div className="overlay fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4 backdrop-blur-sm transition-opacity duration-300" style={{ opacity: drawerVisible ? 1 : 0, pointerEvents: drawerVisible ? "auto" : "none", transition: "opacity 300ms ease-out" }} onClick={() => setDrawer(false)}><div className="drawer w-full max-w-[380px] max-h-[82vh] overflow-y-auto rounded-3xl" style={{ transform: drawerVisible ? "scale(1)" : "scale(.98)", opacity: drawerVisible ? 1 : 0, transition: "transform 200ms ease-out, opacity 200ms ease-out" }} role="dialog" aria-modal="true" aria-label="Выбор стратегии" onClick={(e) => e.stopPropagation()}><div className="drawer-handle" /><div className="dialog-heading"><div><h2>Стратегия DPI</h2><p>Выберите способ обхода</p></div><button className="close-button" onClick={() => setDrawer(false)} aria-label="Закрыть">×</button></div><button className="strategy-option" onClick={() => { onActiveDpiChange(presets[0].id); setDrawer(false); }}><Sparkles size={16} /> Автоподбор лучшего способа</button>{presets.map((strategy) => <button key={strategy.id} className="strategy-option" onClick={() => { onActiveDpiChange(strategy.id); setDrawer(false); }}><span>{strategy.name} • {strategy.engine}<br /><small>{strategy.ping} мс · {strategy.services}/14 сервисов</small></span><span className={`radio ${activeDpiId === strategy.id ? "selected" : ""}`} /></button>)}</div></div>}
    {dpiSetupModal && createPortal(<div className="overlay fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4 backdrop-blur-sm" onClick={() => setDpiSetupModal(false)}><div className="dialog w-full max-w-[380px] rounded-3xl" role="dialog" aria-modal="true" onClick={event => event.stopPropagation()}><h2>Настройка DPI Обхода</h2><p>Сначала настройте обход или проведите тест стратегий</p><div className="dialog-actions"><button type="button" onClick={() => setDpiSetupModal(false)}>Отмена</button><button type="button" className="primary-action" onClick={() => { setDpiSetupModal(false); onDpi(); }}>Проверить способы</button></div></div></div>, document.body)}
    {editing && <div className="overlay fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4 backdrop-blur-sm" onClick={() => setEditing(false)}><form className="dialog w-full max-w-[380px] max-h-[82vh] overflow-y-auto rounded-3xl" role="dialog" aria-modal="true" aria-label="Переименовать стратегию" onClick={(e) => e.stopPropagation()} onSubmit={(e) => { e.preventDefault(); saveName(); }}><h2>Название стратегии</h2><input autoFocus value={name} maxLength={40} onChange={(e) => setName(e.target.value)} /><div className="dialog-actions"><button type="button" onClick={() => setEditing(false)}>Отмена</button><button type="submit" className="primary-action">Сохранить</button></div></form></div>}
  </>;
}
