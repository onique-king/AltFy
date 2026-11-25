import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Moon, Settings, Sun } from "lucide-react";
import LiquidNavBar, { type Tab } from "./components/LiquidNavBar";
import Home from "./pages/Home";
import DpiPage, { type DpiStrategy } from "./pages/DpiPage";
import TgProxyPage from "./pages/TgProxyPage";
import InfoPage from "./pages/InfoPage";
import VpnPage, { type VpnServer } from "./pages/VpnPage";
import SettingsModal from "./components/SettingsModal";
import { useLanguage } from "./i18n/translations";
import "./index.css";

export default function App() {
  const { t } = useLanguage();
  const [tab, setTab] = useState<Tab>("home");
  const [dark, setDark] = useState(() => { try { return localStorage.getItem("altfy-theme") === "dark"; } catch { return false; } });
  const [settings, setSettings] = useState(false);
  const [vpnServer, setVpnServer] = useState<VpnServer | null>(() => { try { const saved = localStorage.getItem("altfy-selected-vpn"); return saved ? JSON.parse(saved) : null; } catch { return null; } });
  const [vpnConnected, setVpnConnected] = useState(false);
  const [isConnecting, setIsConnecting] = useState(false);
  const vpnConnecting = useRef(false);
  const vpnPollPausedUntil = useRef(0);
  const dpiPollPausedUntil = useRef(0);
  const [vpnMode, setVpnMode] = useState<"tun" | "proxy">(() => { try { return JSON.parse(localStorage.getItem("altfy-settings") || "{}").mode === "proxy" ? "proxy" : "tun"; } catch { return "tun"; } });
  const [hasVpnItems, setHasVpnItems] = useState(false);
  const [dpiStrategies, setDpiStrategies] = useState<DpiStrategy[]>(() => { try { const saved = localStorage.getItem("altfy-dpi-strategies"); return saved ? JSON.parse(saved) : []; } catch { return []; } });
  const [activeDpiId, setActiveDpiId] = useState(() => { try { return localStorage.getItem("altfy-active-dpi") || "zapret"; } catch { return "zapret"; } });
  const [dpiAutoSelect, setDpiAutoSelect] = useState(() => { try { return localStorage.getItem("altfy-dpi-auto") === "true"; } catch { return false; } });
  useEffect(() => { try { localStorage.setItem("altfy-dpi-auto", String(dpiAutoSelect)); } catch { /* Storage may be unavailable. */ } }, [dpiAutoSelect]);
  const [dpiEnabled, setDpiEnabled] = useState(() => { try { return localStorage.getItem("altfy-dpi-enabled") === "true"; } catch { return false; } });
  const changeDpiEnabled = (enabled: boolean) => { dpiPollPausedUntil.current = Date.now() + 2000; setDpiEnabled(enabled); window.dispatchEvent(new Event("altfy-dpi-toggle")); };
  useEffect(() => {
    let mounted = true;
    const refresh = async () => { if (Date.now() < dpiPollPausedUntil.current) return; try { const state = await invoke<{ running: boolean }>("get_dpi_state"); if (mounted && Date.now() >= dpiPollPausedUntil.current) setDpiEnabled(state.running); } catch { /* Backend unavailable in browser preview. */ } };
    void refresh();
    const timer = window.setInterval(refresh, 2000);
    return () => { mounted = false; window.clearInterval(timer); };
  }, []);
  const [tgEnabled, setTgEnabled] = useState(false);
  const [toast, setToast] = useState<{ id: number; message: string } | null>(null);
  const tgBusy = useRef(false);
  useEffect(() => {
    const show = (event: Event) => setToast({ id: Date.now() + Math.random(), message: (event as CustomEvent<string>).detail });
    window.addEventListener("altfy-toast", show);
    let unlisten: (() => void) | undefined;
    void listen<string>("altfy-toast", event => setToast({ id: Date.now() + Math.random(), message: event.payload })).then(stop => { unlisten = stop; });
    return () => { window.removeEventListener("altfy-toast", show); unlisten?.(); };
  }, []);
  useEffect(() => {
    if (!toast) return;
    const timer = window.setTimeout(() => setToast(null), 4000);
    return () => window.clearTimeout(timer);
  }, [toast]);
  useEffect(() => {
    let mounted = true;
    const refresh = async () => {
      if (tgBusy.current) return;
      try {
        const running = await invoke<boolean>("get_tg_proxy_status");
        if (mounted && !tgBusy.current) setTgEnabled(running);
      } catch { if (mounted && !tgBusy.current) setTgEnabled(false); }
    };
    void refresh();
    const timer = window.setInterval(refresh, 2000);
    return () => { mounted = false; window.clearInterval(timer); };
  }, []);
  const changeTgEnabled = async (enabled: boolean) => {
    if (tgBusy.current) return;
    tgBusy.current = true;
    try {
      if (enabled) {
        let config = { port: 1443, autoDc: true, poolSize: 4, secret: "", useCfCdn: false, cfPriority: false, cfServer: "" };
        try { config = { ...config, ...JSON.parse(localStorage.getItem("altfy-tg-config") || "{}") }; } catch { /* Storage may be unavailable. */ }
        if (!config.secret) {
          config.secret = await invoke<string>("generate_tg_secret");
          localStorage.setItem("altfy-tg-config", JSON.stringify(config));
          window.dispatchEvent(new Event("altfy-tg-config"));
        }
        if (!Number.isInteger(config.port) || config.port < 1 || config.port > 65535) throw new Error("Недопустимый порт прокси");
        await invoke("start_tg_proxy", { config });
      } else await invoke("stop_tg_proxy");
      setTgEnabled(await invoke<boolean>("get_tg_proxy_status"));
    } catch (error) {
      window.dispatchEvent(new CustomEvent("altfy-toast", { detail: String(error) }));
      try { setTgEnabled(await invoke<boolean>("get_tg_proxy_status")); } catch { setTgEnabled(false); }
    } finally { tgBusy.current = false; }
  };
  useEffect(() => { try { if (vpnServer) localStorage.setItem("altfy-selected-vpn", JSON.stringify(vpnServer)); else localStorage.removeItem("altfy-selected-vpn"); } catch { /* Storage may be unavailable. */ } }, [vpnServer]);
  useEffect(() => {
    let mounted = true;
    const refreshVpn = async () => {
      if (vpnConnecting.current || Date.now() < vpnPollPausedUntil.current) return;
      try {
        const state = await invoke<{ servers: VpnServer[]; subscriptions: unknown[]; selected_server_id?: string | null }>("get_vpn_state");
        if (!mounted) return;
        const selected = state.servers.find(server => server.id === state.selected_server_id) || state.servers.find(server => server.id === vpnServer?.id) || null;
        if (selected) setVpnServer(current => current?.id === selected.id ? current : selected);
        else if (!state.servers.length) setVpnServer(null);
        setHasVpnItems(state.servers.length > 0 || state.subscriptions.length > 0);
        const connected = await invoke<boolean>("get_vpn_status");
        if (mounted && Date.now() >= vpnPollPausedUntil.current) setVpnConnected(connected);
      } catch { /* Backend unavailable in browser preview. */ }
    };
    void refreshVpn();
    const timer = window.setInterval(refreshVpn, 1000);
    return () => { mounted = false; window.clearInterval(timer); };
  }, [vpnServer?.id]);
  const changeVpnMode = async (mode: "tun" | "proxy") => {
    const previousMode = vpnMode;
    vpnPollPausedUntil.current = Date.now() + 2000;
    setVpnMode(mode);
    if (!vpnConnected || !vpnServer) return;
    try {
      await invoke("disconnect_vpn");
      const running = await invoke<boolean>("get_vpn_status");
      if (running) throw new Error("Не удалось отключить VPN для смены режима");
      await invoke("connect_vpn", { id: vpnServer.id, routing: "system" });
      setVpnConnected(await invoke<boolean>("get_vpn_status"));
    } catch (error) {
      setVpnMode(previousMode);
      window.dispatchEvent(new CustomEvent("altfy-toast", { detail: String(error) }));
      try { setVpnConnected(await invoke<boolean>("get_vpn_status")); } catch { setVpnConnected(false); }
    }
  };
  const toggleVpn = async () => {
    const previous = vpnConnected;
    if (vpnConnecting.current) return;
    vpnPollPausedUntil.current = Date.now() + 2000;
    if (!previous && !vpnServer) { setTab("vpn"); window.dispatchEvent(new CustomEvent("altfy-toast", { detail: "Сначала выберите VPN-сервер" })); return; }
    if (previous) {
      setVpnConnected(false);
      try { await invoke("disconnect_vpn"); }
      catch (error) { setVpnConnected(true); window.dispatchEvent(new CustomEvent("altfy-toast", { detail: String(error) })); }
      return;
    }
    vpnConnecting.current = true;
    setIsConnecting(true);
    const startedAt = Date.now();
    try {
      let routing = "system";
      try { const saved = JSON.parse(localStorage.getItem("altfy-settings") || "{}"); if (["system", "direct_apps", "vpn_apps"].includes(saved.routing)) routing = saved.routing; } catch { /* Storage unavailable. */ }
      await invoke("connect_vpn", { id: vpnServer!.id, routing });
      setVpnConnected(true);
    } catch (error) {
      setVpnConnected(false);
      window.dispatchEvent(new CustomEvent("altfy-toast", { detail: String(error) }));
    } finally {
      await new Promise(resolve => window.setTimeout(resolve, Math.max(0, 1200 - (Date.now() - startedAt))));
      vpnConnecting.current = false;
      setIsConnecting(false);
    }
  };
  useEffect(() => { try { localStorage.setItem("altfy-dpi-strategies", JSON.stringify(dpiStrategies)); } catch { /* Storage may be unavailable. */ } }, [dpiStrategies]);
  useEffect(() => { try { localStorage.setItem("altfy-active-dpi", activeDpiId); localStorage.setItem("altfy-dpi-enabled", String(dpiEnabled)); localStorage.setItem("altfy-tg-enabled", String(tgEnabled)); } catch { /* Storage may be unavailable. */ } }, [activeDpiId, dpiEnabled, tgEnabled]);
  useEffect(() => { document.documentElement.classList.toggle("dark", dark); try { localStorage.setItem("altfy-theme", dark ? "dark" : "light"); } catch { /* Storage may be unavailable. */ } }, [dark]);
  useEffect(() => {
    const root = document.getElementById("root");
    if (root) root.classList.add("w-full");
    document.documentElement.style.cssText += ";width:100%;height:100%;margin:0";
    document.body.style.cssText += ";width:100%;height:100%;margin:0;padding:0";
    if (root) root.style.cssText += ";width:100%;height:100%;margin:0;padding:0;max-width:none";
  }, []);
  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => { if (event.key === "Escape") setSettings(false); };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, []);

  useEffect(() => { void (async () => { try { await invoke("apply_startup_settings"); const prefs = await invoke<Record<string, unknown>>("get_app_settings"); if (prefs.autoConnect === true) { const state = await invoke<{ selected_server_id?: string | null }>("get_vpn_state"); if (state.selected_server_id) { await invoke("connect_vpn", { id: state.selected_server_id, routing: prefs.routing }); setVpnConnected(true); } } } catch { /* Startup connection is best-effort. */ } })(); }, []);
  const tabs: Tab[] = ["home", "vpn", "dpi", "tg", "info"];
  const activeIndex = tabs.indexOf(tab);
  const pageClass = "absolute inset-0 w-full h-full overflow-y-auto overflow-x-hidden transition-transform duration-300 ease-out";
  const pageStyle = (page: Tab) => ({ position: "absolute" as const, inset: 0, width: "100%", height: "100%", overflowY: "auto" as const, overflowX: "hidden" as const, boxSizing: "border-box" as const, padding: "0 22px 100px", scrollbarWidth: "none" as const, transform: `translateX(${(tabs.indexOf(page) - activeIndex) * 100}%)`, transition: "transform 300ms ease-out" });
  const pageRef = (page: Tab) => (element: HTMLDivElement | null) => { if (element) element.toggleAttribute("inert", tab !== page); };

  return <main id="app-root" className="app-shell w-full h-screen overflow-hidden flex flex-col">
    <header className="header"><button className="header-button" onClick={() => setSettings(true)} aria-label={t("Настройки")}><Settings size={20} /></button><div className="logo">Alt<span>Fy</span><i /></div><button className="header-button" onClick={() => setDark(!dark)} aria-label={dark ? "Включить светлую тему" : "Включить тёмную тему"}>{dark ? <Sun size={20} /> : <Moon size={20} />}</button></header>
    <div className="content relative w-full flex-1 overflow-hidden" style={{ position: "relative", flex: "1 1 0%", width: "100%", overflow: "hidden", padding: 0, minHeight: 0, minWidth: 0 }}>
        <div className={pageClass} style={pageStyle("home")} ref={pageRef("home")} aria-hidden={tab !== "home"}><Home onVpn={() => setTab("vpn")} onDpi={() => setTab("dpi")} vpnServer={vpnServer} vpnConnected={vpnConnected} vpnMode={vpnMode} hasVpnItems={hasVpnItems} onVpnToggle={() => void toggleVpn()} isConnecting={isConnecting} dpiStrategies={dpiStrategies} onDpiStrategiesChange={setDpiStrategies} activeDpiId={activeDpiId} onActiveDpiChange={setActiveDpiId} dpiEnabled={dpiEnabled} onDpiEnabledChange={changeDpiEnabled} tgEnabled={tgEnabled} onTgEnabledChange={changeTgEnabled} /></div>
        <div className={pageClass} style={pageStyle("vpn")} ref={pageRef("vpn")} aria-hidden={tab !== "vpn"}><VpnPage selectedId={vpnServer?.id || null} onSelect={server => setVpnServer(server)} connected={vpnConnected} isConnecting={isConnecting} onToggleConnection={() => void toggleVpn()} /></div>
        <div className={pageClass} style={pageStyle("dpi")} ref={pageRef("dpi")} aria-hidden={tab !== "dpi"}><DpiPage strategies={dpiStrategies} onStrategiesChange={setDpiStrategies} activeId={activeDpiId} onActiveChange={setActiveDpiId} autoSelect={dpiAutoSelect} onAutoSelectChange={setDpiAutoSelect} enabled={dpiEnabled} onEnabledChange={changeDpiEnabled} /></div>
        <div className={pageClass} style={pageStyle("tg")} ref={pageRef("tg")} aria-hidden={tab !== "tg"}><TgProxyPage enabled={tgEnabled} onEnabledChange={changeTgEnabled} /></div>
        <div className={pageClass} style={pageStyle("info")} ref={pageRef("info")} aria-hidden={tab !== "info"}><InfoPage /></div>
    </div>
    <LiquidNavBar tab={tab} onChange={setTab} />
    {toast && <div className="app-toast" role="status" onClick={() => setToast(null)}>{toast.message}</div>}
    {settings && <SettingsModal dark={dark} onThemeChange={setDark} onClose={() => setSettings(false)} onModeChange={changeVpnMode} />}
  </main>;
}
