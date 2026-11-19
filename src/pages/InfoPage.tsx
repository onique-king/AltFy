import { useEffect, useState } from "react";
import { Check, Copy, ExternalLink, Heart, Terminal, X } from "lucide-react";

const logs = "[12:00:01] [sing-box] Demo: initialized\n[12:00:02] [zapret] Demo: ready\n[12:00:03] [tg-ws-proxy] Demo: listening on 127.0.0.1:1080";

export default function InfoPage() {
  const [open, setOpen] = useState(false), [copied, setCopied] = useState(false);
  const [en] = useState(() => { try { return JSON.parse(localStorage.getItem("altfy-settings") || "{}").language === "en"; } catch { return false; } });
  useEffect(() => { if (!open) return; const close = (e: KeyboardEvent) => { if (e.key === "Escape") setOpen(false); }; window.addEventListener("keydown", close); return () => window.removeEventListener("keydown", close); }, [open]);
  return <section className="flex flex-col gap-4 pt-4">
    <div className="card p-5 text-center"><div className="logo text-3xl">Alt<span>Fy</span><i /></div><p className="mt-1 text-xs text-[var(--muted)]">v0.1.0</p><a className="mt-4 inline-flex items-center gap-2 text-xs font-bold text-violet-500" href="https://github.com" target="_blank" rel="noreferrer"><ExternalLink size={16} /> GitHub</a></div>
    <div className="card p-5"><h2 className="mb-3 text-sm font-extrabold">{en ? "Acknowledgements" : "Благодарности"}</h2>{["Sing-box", "Zapret / Flowseal", "ByeDPI", "tg-ws-proxy"].map(name => <div key={name} className="border-t border-[var(--border)] py-3 text-xs font-bold">{name}</div>)}</div>
    <button className="dpi-primary" onClick={() => { setCopied(false); setOpen(true); }}><Terminal size={16} />{en ? "Live core log" : "Живой лог ядер"}</button>
    <div className="card flex items-center gap-3 p-4"><span className="icon-box"><Heart size={18} /></span><div><strong className="text-xs">{en ? "Support the project" : "Поддержать проект"}</strong><p className="mt-1 text-[10px] text-[var(--muted)]">{en ? "Thank you for supporting AltFy!" : "Спасибо за поддержку AltFy!"}</p></div></div>
    {open && <div className="overlay" onClick={() => setOpen(false)}><div className="dialog" role="dialog" aria-modal="true" aria-labelledby="log-title" onClick={e => e.stopPropagation()}><div className="dialog-heading"><h2 id="log-title">{en ? "Live core log" : "Живой лог ядер"}</h2><button className="close-button" onClick={() => setOpen(false)} aria-label={en ? "Close" : "Закрыть"}><X size={20} /></button></div><p className="mt-2 text-xs text-[var(--muted)]">{en ? "Demo logs" : "Демо-логи"}</p><pre className="my-4 overflow-auto whitespace-pre-wrap rounded-xl bg-[var(--bg)] p-3 text-[10px] leading-relaxed select-text">{logs}</pre><button className="dpi-primary" onClick={async () => { try { await navigator.clipboard.writeText(logs); setCopied(true); } catch { setCopied(false); } }}>{copied ? <Check size={16} /> : <Copy size={16} />}{copied ? en ? "Copied" : "Скопировано" : en ? "Copy" : "Копировать"}</button></div></div>}
  </section>;
}
