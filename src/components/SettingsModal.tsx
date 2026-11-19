import { useEffect, useState } from "react";
import { ChevronDown, X } from "lucide-react";

type Settings = {
  language: "ru" | "en";
  autostart: boolean;
  minimizeToTray: boolean;
  mode: "tun" | "proxy";
  lan: boolean;
  ipv6: boolean;
  routing: "all" | "blocked" | "split";
  hideHwid: boolean;
  userAgent: string;
  autoUpdate: boolean;
  packetLength: string;
  packetInterval: string;
};

const defaults: Settings = {
  language: "ru", autostart: false, minimizeToTray: false, mode: "tun",
  lan: false, ipv6: false, routing: "blocked", hideHwid: false, userAgent: "AltFy/1.0", autoUpdate: false,
  packetLength: "", packetInterval: "",
};
const storageKey = "altfy-settings";

function loadSettings(): Settings {
  try {
    const saved: unknown = JSON.parse(localStorage.getItem(storageKey) || "null");
    if (!saved || typeof saved !== "object" || Array.isArray(saved)) return { ...defaults };
    const values = saved as Record<string, unknown>;
    const result = { ...defaults };
    for (const key of Object.keys(defaults) as (keyof Settings)[]) {
      const value = values[key];
      if (typeof value !== typeof defaults[key]) continue;
      if (key === "language" && value !== "ru" && value !== "en") continue;
      if (key === "mode" && value !== "tun" && value !== "proxy") continue;
      if (key === "routing" && value !== "all" && value !== "blocked" && value !== "split") continue;
      Object.assign(result, { [key]: value });
    }
    return result;
  } catch { return { ...defaults }; }
}

function Toggle({ label, description, checked, onChange }: { label: string; description?: string; checked: boolean; onChange: (value: boolean) => void }) {
  return <div className="settings-row"><div className="settings-copy"><span>{label}</span>{description && <small>{description}</small>}</div><button type="button" className={`switch${checked ? " on" : ""}`} role="switch" aria-label={label} aria-checked={checked} onClick={() => onChange(!checked)}><span /></button></div>;
}

export default function SettingsModal({ dark, onThemeChange, onClose }: { dark: boolean; onThemeChange: (dark: boolean) => void; onClose: () => void }) {
  const [settings, setSettings] = useState(loadSettings);
  const [fragmentationOpen, setFragmentationOpen] = useState(false);
  useEffect(() => { try { localStorage.setItem(storageKey, JSON.stringify(settings)); } catch { /* Storage may be unavailable. */ } }, [settings]);
  const update = <K extends keyof Settings>(key: K, value: Settings[K]) => setSettings((previous) => ({ ...previous, [key]: value }));
  const reset = () => { setSettings({ ...defaults }); onThemeChange(false); };
  const en = settings.language === "en";

  return <div className="overlay" onClick={onClose}>
    <div className="dialog settings-dialog" role="dialog" aria-modal="true" aria-labelledby="settings-title" onClick={(event) => event.stopPropagation()}>
      <div className="dialog-heading"><h2 id="settings-title">{en ? "Settings" : "Настройки"}</h2><button className="close-button" onClick={onClose} aria-label="Закрыть"><X size={20} /></button></div>
      <div className="settings-scroll">
        <section className="settings-group"><h3>{en ? "Appearance and General" : "Внешний вид и Общие"}</h3><div className="settings-card">
          <label className="settings-row"><span>Тема</span><select value={dark ? "dark" : "light"} onChange={(event) => onThemeChange(event.target.value === "dark")}><option value="light">Светлая</option><option value="dark">Темная</option></select></label>
          <label className="settings-row"><span>Язык</span><select value={settings.language} onChange={(event) => update("language", event.target.value as Settings["language"])}><option value="ru">Русский</option><option value="en">English</option></select></label>
          <Toggle label="Автозапуск с Windows" checked={settings.autostart} onChange={(value) => update("autostart", value)} />
          <Toggle label="Сворачивать в трей при закрытии" checked={settings.minimizeToTray} onChange={(value) => update("minimizeToTray", value)} />
        </div></section>
        <section className="settings-group"><h3>{en ? "Mode and Network" : "Режим работы и Сеть"}</h3><div className="settings-card">
          <div className="settings-block"><span>Режим туннелирования</span><div className="settings-segment" role="group" aria-label="Режим туннелирования"><button type="button" className={settings.mode === "tun" ? "selected" : ""} aria-pressed={settings.mode === "tun"} onClick={() => update("mode", "tun")}>TUN <small>Виртуальный адаптер</small></button><button type="button" className={settings.mode === "proxy" ? "selected" : ""} aria-pressed={settings.mode === "proxy"} onClick={() => update("mode", "proxy")}>Системный прокси</button></div></div>
          <Toggle label="Разрешить локальную сеть (LAN)" checked={settings.lan} onChange={(value) => update("lan", value)} />
          <Toggle label="Поддержка IPv6" checked={settings.ipv6} onChange={(value) => update("ipv6", value)} />
          <div className="settings-block"><label htmlFor="routing">{en ? "Routing rules" : "Правила маршрутизации"}</label><select id="routing" value={settings.routing} onChange={(event) => update("routing", event.target.value as Settings["routing"])}><option value="blocked">{en ? "Default (Russia direct)" : "По умолчанию (РФ напрямую)"}</option><option value="all">{en ? "All traffic through VPN" : "Весь трафик через VPN"}</option><option value="split">{en ? "Custom rules" : "Свои правила"}</option></select><label htmlFor="user-agent">User-Agent</label><input id="user-agent" type="text" value={settings.userAgent} onChange={(event) => update("userAgent", event.target.value)} /></div>
        </div></section>
        <section className="settings-group"><h3>{en ? "Subscriptions and Security" : "Подписки и Безопасность"}</h3><div className="settings-card">
          <Toggle label={en ? "Do not send device HWID" : "Не передавать HWID устройства"} description={en ? "Hide the hardware identifier when updating subscriptions" : "Скрывать идентификатор железа при обновлении подписок"} checked={settings.hideHwid} onChange={(value) => update("hideHwid", value)} />
          <Toggle label="Автообновление подписок при запуске" checked={settings.autoUpdate} onChange={(value) => update("autoUpdate", value)} />
          <button type="button" className="settings-row settings-expand" aria-expanded={fragmentationOpen} aria-controls="fragmentation-settings" onClick={() => setFragmentationOpen(!fragmentationOpen)}>Расширенные настройки фрагментации<ChevronDown size={17} className={fragmentationOpen ? "expanded" : ""} /></button>
          {fragmentationOpen && <div id="fragmentation-settings" className="settings-fragment"><label>Длина пакетов<input type="number" min="0" value={settings.packetLength} onChange={(event) => update("packetLength", event.target.value)} /></label><label>Интервал пакетов<input type="number" min="0" value={settings.packetInterval} onChange={(event) => update("packetInterval", event.target.value)} /></label></div>}
        </div></section>
        <div className="settings-actions"><button type="button" className="settings-reset" onClick={reset}>Сбросить настройки по умолчанию</button><button type="button" className="primary-action" onClick={onClose}>Закрыть</button></div>
      </div>
    </div>
  </div>;
}
