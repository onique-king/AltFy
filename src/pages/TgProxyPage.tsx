import { useEffect, useState } from "react";
import { createPortal } from "react-dom";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Copy, Eye, EyeOff, ExternalLink, RefreshCw, Send } from "lucide-react";
import "./DpiPage.css";
import "./TgProxyPage.css";
import { useLanguage } from "../i18n/translations";

type Config = { port: number; autoDc: boolean; poolSize: number; secret: string; useCfCdn: boolean; cfPriority: boolean; cfServer: string };
const servers = [
  ["162.159.192.1", "Амстердам", "Amsterdam"], ["162.159.193.1", "Франкфурт", "Frankfurt"],
  ["188.114.96.1", "Варшава", "Warsaw"], ["188.114.97.1", "Хельсинки", "Helsinki"],
  ["188.114.98.1", "Стокгольм", "Stockholm"], ["188.114.99.1", "Лондон", "London"],
  ["104.16.132.229", "Париж", "Paris"], ["104.16.133.229", "Прага", "Prague"],
  ["172.67.70.1", "Вена", "Vienna"], ["104.18.2.1", "Рига", "Riga"],
  ["104.18.3.1", "Таллин", "Tallinn"], ["104.19.192.1", "Вильнюс", "Vilnius"],
  ["104.20.1.1", "Цюрих", "Zurich"], ["104.21.1.1", "Милан", "Milan"],
  ["104.22.1.1", "Мадрид", "Madrid"], ["104.24.1.1", "Бухарест", "Bucharest"],
  ["104.25.1.1", "София", "Sofia"], ["104.26.1.1", "Стамбул", "Istanbul"],
  ["104.27.1.1", "Алматы", "Almaty"], ["1.1.1.1", "Cloudflare Anycast", "Cloudflare Anycast"],
] as const;
const defaults: Config = { port: 1443, autoDc: true, poolSize: 4, secret: "", useCfCdn: false, cfPriority: false, cfServer: "" };
const readConfig = (): Config => { try { return { ...defaults, ...JSON.parse(localStorage.getItem("altfy-tg-config") || "{}") }; } catch { return defaults; } };
type Props = { enabled: boolean; onEnabledChange: (enabled: boolean) => void };

export default function TgProxyPage({ enabled, onEnabledChange }: Props) {
  const [config, setConfig] = useState<Config>(readConfig);
  const [visible, setVisible] = useState(false);
  const [ping, setPing] = useState<number | null>(null);
  const [download, setDownload] = useState("");
  const [testing, setTesting] = useState(false);
  const [serverDialog, setServerDialog] = useState(false);
  const [results, setResults] = useState<Record<string, number | null>>({});
  const { t } = useLanguage();
  const notify = (message: string) => window.dispatchEvent(new CustomEvent("altfy-toast", { detail: message }));
  const update = (patch: Partial<Config>) => setConfig(current => { const next = { ...current, ...patch }; try { localStorage.setItem("altfy-tg-config", JSON.stringify(next)); } catch { notify(t("Не удалось сохранить настройки", "Could not save settings")); } return next; });
  useEffect(() => {
    const sync = () => setConfig(readConfig());
    window.addEventListener("altfy-tg-config", sync);
    if (!config.secret) invoke<string>("generate_tg_secret").then(secret => { const current = readConfig(); if (!current.secret) { current.secret = secret; try { localStorage.setItem("altfy-tg-config", JSON.stringify(current)); } catch { /* Storage may be unavailable. */ } setConfig(current); } }).catch(error => notify(String(error)));
    return () => window.removeEventListener("altfy-tg-config", sync);
  }, []);
  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;
    listen<{ name: string; message: string; done: boolean }>("download-progress", ({ payload }) => {
      if (payload.name !== "tg-ws-proxy.exe") return;
      if (payload.done) { setDownload(""); notify(payload.message); }
      else setDownload(payload.message);
    }).then(stop => { if (disposed) stop(); else unlisten = stop; }).catch(error => notify(String(error)));
    return () => { disposed = true; unlisten?.(); };
  }, []);
  useEffect(() => {
    let active = true;
    const check = () => { if (!enabled) { setPing(null); return; } invoke<number>("check_tg_proxy_ping", { port: config.port }).then(value => { if (active) setPing(value); }).catch(() => { if (active) setPing(null); }); };
    check(); const timer = window.setInterval(check, 5000);
    return () => { active = false; window.clearInterval(timer); };
  }, [enabled, config.port]);
  const link = config.secret ? `tg://proxy?server=127.0.0.1&port=${config.port}&secret=${encodeURIComponent(config.secret)}` : `tg://socks?server=127.0.0.1&port=${config.port}`;
  const validPort = Number.isInteger(config.port) && config.port > 0 && config.port <= 65535;
  const copy = async (value: string) => { try { await navigator.clipboard.writeText(value); notify(t("Скопировано", "Copied")); } catch (error) { notify(String(error)); } };
  const testServers = async () => { setTesting(true); try { setResults(await invoke<Record<string, number | null>>("ping_cf_servers")); } catch (error) { notify(String(error)); } finally { setTesting(false); } };
  const toggle = (label: string, value: boolean, change: (value: boolean) => void, disabled = enabled) => <label className="tg-row"><span>{label}</span><input type="checkbox" checked={value} disabled={disabled} onChange={event => change(event.target.checked)} /></label>;
  return <section className="tg-page">
    <div className={`tg-status-card ${enabled ? "enabled" : ""}`}>
      <div className="tg-status-top"><span className="tg-large-icon"><Send size={25} /></span><span className="tg-ping">{ping === null ? "--" : ping} {t("мс", "ms")}</span></div>
      <div className="tg-status-copy"><h1>TG-WS Proxy</h1><p>127.0.0.1:{config.port}</p></div>
      <button className={`tg-power ${enabled ? "on" : ""}`} role="switch" aria-checked={enabled} aria-label="TG-WS Proxy" onClick={() => onEnabledChange(!enabled)}><span /></button>
      <div className="tg-actions"><button disabled={!validPort} onClick={() => invoke("open_external_url", { url: link }).catch(() => notify(t("Не удалось открыть Telegram. Скопируйте ссылку и откройте её вручную.", "Could not open Telegram. Copy the link and open it manually.")))}><ExternalLink size={15} />{t("Добавить в Telegram", "Add to Telegram")}</button><button disabled={!validPort} onClick={() => void copy(link)}><Copy size={15} />{t("Ссылка", "Link")}</button></div>
    </div>
    {download && createPortal(<div role="status" style={{ position: "fixed", zIndex: 1000, bottom: 92, left: "50%", transform: "translateX(-50%)", width: "max-content", maxWidth: "calc(100% - 32px)", padding: "10px 16px", borderRadius: 12, background: "#6d28d9", color: "white", boxShadow: "0 6px 24px #0004", fontSize: 12, overflowWrap: "anywhere" }}>{download}</div>, document.body)}
    <div className="card tg-settings"><h2>{t("Подключение", "Connection")}</h2>
      <label className="tg-port-label">{t("Локальный порт", "Local port")}<span className="tg-port-input"><span className="whitespace-nowrap shrink-0">127.0.0.1 :</span><input type="number" min="1" max="65535" value={config.port || ""} disabled={enabled} onChange={event => update({ port: Number(event.target.value) })} /></span></label>
      {toggle(t("Автовыбор IP-адресов DC", "Auto-select DC IP addresses"), config.autoDc, autoDc => update({ autoDc }))}
      <div className="tg-pool"><span>{t("Пул WebSocket-соединений", "WebSocket connection pool")}</span><div>{[2, 4, 6].map(size => <button key={size} disabled={enabled} className={config.poolSize === size ? "selected" : ""} onClick={() => update({ poolSize: size })}>{size}</button>)}</div></div>
    </div>
    <div className="card tg-settings"><h2>{t("Безопасность", "Security")}</h2><label className="tg-port-label">{t("Секретный ключ", "Secret key")}<span className="tg-port-input"><input type={visible ? "text" : "password"} value={config.secret} readOnly /><button aria-label={visible ? t("Скрыть", "Hide") : t("Показать", "Show")} onClick={() => setVisible(!visible)}>{visible ? <EyeOff size={16} /> : <Eye size={16} />}</button></span></label>
      <div className="tg-actions"><button onClick={() => void copy(config.secret)} disabled={!config.secret}><Copy size={15} />{t("Копировать", "Copy")}</button><button disabled={enabled} onClick={() => invoke<string>("generate_tg_secret").then(secret => update({ secret })).catch(error => notify(String(error)))}><RefreshCw size={15} />{t("Пересоздать", "Regenerate")}</button></div>
    </div>
    <div className="card tg-settings"><h2>Cloudflare CDN Proxy</h2>
      {toggle(t("Использовать Cloudflare CDN", "Use Cloudflare CDN"), config.useCfCdn, useCfCdn => update({ useCfCdn }))}
      {toggle(t("Приоритет Cloudflare", "Prioritize Cloudflare"), config.cfPriority, cfPriority => update({ cfPriority }), enabled || !config.useCfCdn)}
      <button className="tg-row w-full text-left" disabled={enabled || !config.useCfCdn} onClick={() => setServerDialog(true)}><span>{t("Серверы Cloudflare", "Cloudflare servers")}</span><span style={{ color: "#8B5CF6" }}>{config.cfServer || t("Авто", "Auto")} ›</span></button>
      {serverDialog && createPortal(<div style={{ position: "fixed", inset: 0, zIndex: 900, background: "#0008", display: "flex", alignItems: "center", justifyContent: "center", padding: 16 }} onClick={() => setServerDialog(false)}>
        <div role="dialog" aria-modal="true" aria-label={t("Серверы Cloudflare", "Cloudflare servers")} onClick={event => event.stopPropagation()} style={{ width: "100%", maxWidth: 390, maxHeight: "80vh", display: "flex", flexDirection: "column", padding: 16, borderRadius: 18, background: "var(--surface)", color: "var(--text)" }}>
          <div className="flex justify-between items-center"><strong>Cloudflare CDN</strong><button onClick={() => setServerDialog(false)} aria-label={t("Закрыть", "Close")}>✕</button></div>
          <p className="text-xs my-3" style={{ color: "var(--muted)" }}>{t("Названия — ориентиры; фактическая локация Anycast может отличаться. Пинг: TCP к порту 443.", "Labels are indicative; actual Anycast locations may differ. Ping: TCP to port 443.")}</p>
          <button className="tg-test" disabled={testing} onClick={() => void testServers()}>{testing ? t("Проверка…", "Testing…") : t("Проверить все", "Test all")}</button>
          <div style={{ overflowY: "auto", marginTop: 8 }}><button className="tg-row w-full text-left" onClick={() => { update({ cfServer: "" }); setServerDialog(false); }}><span>{t("Авто", "Auto")}</span>{!config.cfServer && <span>✓</span>}</button>
            {servers.map(([ip, ru, english]) => <button key={ip} className="tg-row w-full text-left" onClick={() => { update({ cfServer: ip }); setServerDialog(false); }}><span>{t(ru, english)}<br /><small style={{ color: "var(--muted)" }}>{ip}</small></span><span style={{ color: "#8B5CF6", whiteSpace: "nowrap" }}>{ip in results ? results[ip] === null ? t("Недоступен", "Unavailable") : `${results[ip]} ${t("мс", "ms")}` : "--"}{config.cfServer === ip ? " ✓" : ""}</span></button>)}
          </div>
        </div>
      </div>, document.body)}
    </div>
  </section>;
}
