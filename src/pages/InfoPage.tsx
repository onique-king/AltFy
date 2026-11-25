import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useLanguage } from "../i18n/translations";
import { Check, Copy, ExternalLink, Heart, RefreshCw, Star, Terminal, Trash2, X } from "lucide-react";

type EngineLog = { engine?: string; stream?: string; message?: string };
type Component = { id: string; name: string; description: string; binary: string };

const components: Component[] = [
  { id: "sing-box.exe", name: "Sing-box", description: "Ядро VPN и обработка туннельных подключений", binary: "sing-box.exe" },
  { id: "winws.exe", name: "Zapret / Flowseal", description: "Обход DPI-блокировок и фильтрации трафика", binary: "winws.exe" },
  { id: "ciadpi.exe", name: "ByeDPI", description: "Обход DPI с помощью модификации сетевых пакетов", binary: "ciadpi.exe" },
  { id: "tg-ws-proxy.exe", name: "tg-ws-proxy", description: "Прокси для подключения Telegram через WebSocket", binary: "tg-ws-proxy.exe" },
];

export default function InfoPage() {
  const [open, setOpen] = useState(false), [copied, setCopied] = useState(false);
  const [logLines, setLogLines] = useState<string[]>([]);
  const [binaries, setBinaries] = useState<Record<string, boolean>>({});
  const [updating, setUpdating] = useState("");
  const [checkingUpdate, setCheckingUpdate] = useState(false);
  const logRef = useRef<HTMLPreElement>(null);
  const { language, t } = useLanguage();
  const en = language === "en";
  const logText = logLines.join("\n");
  const toast = (message: string) => window.dispatchEvent(new CustomEvent("altfy-toast", { detail: message }));

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    void listen<EngineLog>("engine-log", ({ payload }) => {
      const message = payload?.message?.trim();
      if (!message) return;
      const time = new Date().toLocaleTimeString([], { hour12: false });
      const engine = payload.engine || "engine";
      const stream = payload.stream ? `/${payload.stream}` : "";
      setLogLines(previous => [...previous.slice(-999), `[${time}] [${engine}${stream}] ${message}`]);
    }).then(fn => { unlisten = fn; }).catch(() => {});
    return () => unlisten?.();
  }, []);

  useEffect(() => {
    if (logRef.current) logRef.current.scrollTop = logRef.current.scrollHeight;
  }, [logLines, open]);

  useEffect(() => {
    let mounted = true;
    void invoke<Record<string, boolean>>("get_binary_status").then(value => { if (mounted) setBinaries(value); }).catch(() => {});
    return () => { mounted = false; };
  }, []);

  useEffect(() => { if (!open) return; const close = (e: KeyboardEvent) => { if (e.key === "Escape") setOpen(false); }; window.addEventListener("keydown", close); return () => window.removeEventListener("keydown", close); }, [open]);

  const checkUpdates = async () => {
    setCheckingUpdate(true);
    try {
      const latest = await invoke<string>("check_app_updates");
      toast(latest === "1.0.1" ? (en ? "AltFy is up to date (v1.0.1)" : "Установлена актуальная версия AltFy v1.0.1") : (en ? `AltFy ${latest} is available` : `Доступно обновление AltFy ${latest}`));
    } catch (error) { toast(en ? `Update check failed: ${String(error)}` : `Не удалось проверить обновления: ${String(error)}`); }
    finally { setCheckingUpdate(false); }
  };

  const updateComponent = async (component: Component) => {
    setUpdating(component.id);
    try {
      await invoke("update_binary", { name: component.binary });
      setBinaries(previous => ({ ...previous, [component.id]: true }));
      toast(en ? `${component.name} updated successfully` : `${component.name}: обновление завершено`);
    } catch (error) { toast(en ? `${component.name} update failed: ${String(error)}` : `${component.name}: ошибка обновления — ${String(error)}`); }
    finally { setUpdating(""); }
  };

  return <section className="flex flex-col gap-4 pt-4 pb-24">
    <div className="card p-5 text-center"><div className="logo text-3xl">Alt<span>Fy</span><i /></div><p className="mt-1 text-xs text-[var(--muted)]">v1.0.1</p><div className="mt-4 flex items-center justify-center gap-3"><a className="inline-flex items-center gap-2 text-xs font-bold text-violet-500" href="https://github.com/onique-king/AltFy" target="_blank" rel="noreferrer"><ExternalLink size={16} /> GitHub</a><a href="https://github.com/onique-king/AltFy" target="_blank" rel="noreferrer" aria-label={en ? "Support with a star on GitHub" : "Поддержать звездой на GitHub"} className="group relative text-yellow-400 transition-transform hover:scale-110"><Star size={17} className="fill-yellow-400" /><span className="pointer-events-none absolute left-1/2 top-full z-10 mt-2 w-max -translate-x-1/2 rounded-lg bg-slate-900 px-2.5 py-1.5 text-[10px] font-medium text-white opacity-0 shadow-lg transition-opacity group-hover:opacity-100">{en ? "Support with a star on GitHub" : "Поддержать звездой на GitHub"}</span></a></div><button className="mx-auto mt-3 inline-flex items-center gap-2 rounded-xl border border-violet-500/30 px-3 py-2 text-[11px] font-bold text-violet-500 transition-colors hover:bg-violet-500/10 disabled:opacity-50" onClick={checkUpdates} disabled={checkingUpdate}><RefreshCw size={13} className={checkingUpdate ? "animate-spin" : ""} />{en ? "Check for updates" : "Проверить обновления"}</button></div>
    <div className="card p-5"><h2 className="mb-2 text-sm font-extrabold">{en ? "Components and cores" : "Компоненты и ядра"}</h2>{components.map(component => <div key={component.id} className="flex items-center gap-2 border-t border-[var(--border)] py-3"><div className="min-w-0 flex-1"><div className="flex flex-wrap items-center gap-2"><strong className="text-xs">{component.name}</strong><span className={`rounded-full px-2 py-0.5 text-[9px] font-bold ${binaries[component.id] ? "bg-emerald-500/10 text-emerald-500" : "bg-[var(--bg)] text-[var(--muted)]"}`}>{binaries[component.id] ? en ? "Installed" : "Установлен" : en ? "Not installed" : "Не установлен"}</span></div><p className="mt-1 text-[10px] leading-relaxed text-[var(--muted)]">{component.description}</p></div><button className="close-button shrink-0" onClick={() => void updateComponent(component)} disabled={Boolean(updating)} aria-label={`${en ? "Update" : "Обновить"} ${component.name}`} title={en ? "Download latest release" : "Скачать свежую версию"}><RefreshCw size={15} className={updating === component.id ? "animate-spin text-violet-500" : ""} /></button></div>)}<p className="mt-2 text-[9px] leading-relaxed text-[var(--muted)]">{en ? "Thanks to the authors and contributors of these open-source projects." : "Благодарим авторов и участников этих открытых проектов."}</p></div>
    <button className="dpi-primary" onClick={() => { setCopied(false); setOpen(true); }}><Terminal size={16} />{en ? "Live core log" : "Живой лог работы"}</button>
    <div className="card flex items-center gap-3 p-4"><span className="icon-box"><Heart size={18} /></span><div><strong className="text-xs">{en ? "Support the project" : "Поддержать проект"}</strong><p className="mt-1 text-[10px] text-[var(--muted)]">{en ? "Thank you for supporting AltFy!" : "Спасибо за поддержку AltFy!"}</p></div></div>
    {open && <div className="overlay fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4 backdrop-blur-sm" onClick={() => setOpen(false)}><div className="dialog w-full max-w-[380px] max-h-[82vh] overflow-y-auto rounded-3xl" role="dialog" aria-modal="true" aria-labelledby="log-title" onClick={e => e.stopPropagation()}><div className="dialog-heading"><h2 id="log-title">{en ? "Live core log" : "Живой лог работы"}</h2><button className="close-button" onClick={() => setOpen(false)} aria-label={en ? "Close" : "Закрыть"}><X size={20} /></button></div><p className="mt-2 text-xs text-[var(--muted)]">{en ? "sing-box, winws.exe, ciadpi.exe and tg-ws-proxy.exe" : "sing-box, winws.exe, ciadpi.exe и tg-ws-proxy.exe"}</p><pre ref={logRef} className="my-4 max-h-72 min-h-28 overflow-auto whitespace-pre-wrap rounded-xl bg-[var(--bg)] p-3 text-[10px] leading-relaxed select-text">{logText || (en ? "Waiting for engine logs…" : "Ожидание логов от ядер…")}</pre><div className="flex gap-2"><button className="dpi-primary" onClick={async () => { try { await navigator.clipboard.writeText(logText); setCopied(true); } catch { setCopied(false); } }} disabled={!logText}>{copied ? <Check size={16} /> : <Copy size={16} />}{copied ? en ? "Copied" : "Скопировано" : en ? "Copy log" : "Копировать лог"}</button><button className="dpi-primary" onClick={() => { setLogLines([]); setCopied(false); }} disabled={!logText}><Trash2 size={16} />{en ? "Clear" : "Очистить"}</button></div></div></div>}
  </section>;
}
