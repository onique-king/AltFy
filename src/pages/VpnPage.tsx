import { useEffect, useRef, useState, type ReactNode } from "react";
import { createPortal } from "react-dom";
import { invoke } from "@tauri-apps/api/core";
import "../components/VpnPage.css";
import CustomSelect from "../components/CustomSelect";
import { useLanguage } from "../i18n/translations";
import { Activity, Camera, Check, ChevronLeft, ChevronRight, Clipboard, Copy, Folder, ImagePlus, Link, LoaderCircle, MoreVertical, Plus, RefreshCw, Server, Trash2, X, Gauge, Settings2 } from "lucide-react";

export type VpnServer = { id: string; flag: string; name: string; protocol: string; ping: number; group?: string; address?: string; port?: string; key?: string; source?: string };
type Props = { selectedId: string | null; onSelect: (server: VpnServer | null) => void; connected: boolean; isConnecting: boolean; onToggleConnection: () => void };
type Subscription = { id: string; name: string; source: string; upload: number; download: number; total: number; expire: number; description?: string };
function loadServers(): VpnServer[] {
  try {
    const value = JSON.parse(localStorage.getItem("altfy-vpn-servers") || "[]");
    return Array.isArray(value) ? value.filter((server: VpnServer) => !["nl", "de", "fi", "travel-us", "travel-jp", "work-gb", "work-ch"].includes(server.id)) : [];
  } catch { /* storage unavailable */ }
  return [];
}
const flagCountries = ["eu", "nl", "se", "ru", "de", "fi", "us", "tr", "kz", "gb", "pl", "fr", "lv", "ee", "ch", "jp", "sg"] as const;
type FlagCountry = typeof flagCountries[number];
const flagKeywords: [RegExp, FlagCountry][] = [
  [/(?<![\p{L}\p{N}])(?:авто|auto|eu|европа|europe)(?![\p{L}\p{N}])/iu, "eu"],
  [/(?<![\p{L}\p{N}])(?:нидерланды|netherlands|nl)(?![\p{L}\p{N}])/iu, "nl"], [/(?<![\p{L}\p{N}])(?:швеция|sweden|se)(?![\p{L}\p{N}])/iu, "se"],
  [/(?<![\p{L}\p{N}])(?:россия|russia|ru)(?![\p{L}\p{N}])/iu, "ru"], [/(?<![\p{L}\p{N}])(?:германия|germany|de)(?![\p{L}\p{N}])/iu, "de"],
  [/(?<![\p{L}\p{N}])(?:финляндия|finland|fi)(?![\p{L}\p{N}])/iu, "fi"], [/(?<![\p{L}\p{N}])(?:сша|usa|us|united states)(?![\p{L}\p{N}])/iu, "us"],
  [/(?<![\p{L}\p{N}])(?:турция|turkey|tr)(?![\p{L}\p{N}])/iu, "tr"], [/(?<![\p{L}\p{N}])(?:казахстан|kazakhstan|kz)(?![\p{L}\p{N}])/iu, "kz"],
  [/(?<![\p{L}\p{N}])(?:великобритания|united kingdom|uk|gb)(?![\p{L}\p{N}])/iu, "gb"], [/(?<![\p{L}\p{N}])(?:польша|poland|pl)(?![\p{L}\p{N}])/iu, "pl"],
  [/(?<![\p{L}\p{N}])(?:франция|france|fr)(?![\p{L}\p{N}])/iu, "fr"], [/(?<![\p{L}\p{N}])(?:латвия|latvia|lv)(?![\p{L}\p{N}])/iu, "lv"],
  [/(?<![\p{L}\p{N}])(?:эстония|estonia|ee)(?![\p{L}\p{N}])/iu, "ee"], [/(?<![\p{L}\p{N}])(?:швейцария|switzerland|ch)(?![\p{L}\p{N}])/iu, "ch"],
  [/(?<![\p{L}\p{N}])(?:япония|japan|jp)(?![\p{L}\p{N}])/iu, "jp"], [/(?<![\p{L}\p{N}])(?:сингапур|singapore|sg)(?![\p{L}\p{N}])/iu, "sg"],
];
function countryCode(value: string) {
  const emoji = value.match(/[🇦-🇿]{2}/u)?.[0];
  if (emoji) return Array.from(emoji, letter => String.fromCharCode(letter.codePointAt(0)! - 0x1f1e6 + 65)).join("").toLowerCase();
  const normalized = value.replace(/[_-]+/g, " ");
  const namedCountry = flagKeywords.find(([pattern]) => pattern.test(normalized))?.[1];
  if (namedCountry) return namedCountry;
  return normalized.match(/(?<![\p{L}\p{N}])([a-z]{2})(?![\p{L}\p{N}])/iu)?.[1]?.toLowerCase() || "";
}
export function Flag({ flag, name }: { flag: string; name: string }) {
  const code = countryCode(`${flag} ${name}`);
  const clip = `flag-${code}`;
  const stripe = (y: number, color: string, height = 12) => <rect x="0" y={y} width="36" height={height} fill={color} />;
  const vertical = (x: number, color: string, width: number) => <rect x={x} y="0" width={width} height="36" fill={color} />;
  let design: ReactNode = null;
  switch (code) {
    case "nl": design = <>{stripe(0, "#ae1c28")}{stripe(12, "#fff")}{stripe(24, "#21468b")}</>; break;
    case "eu": design = <><rect width="36" height="36" fill="#003399" />{Array.from({ length: 12 }, (_, i) => <circle key={i} cx={18 + Math.cos(i * Math.PI / 6) * 9} cy={18 + Math.sin(i * Math.PI / 6) * 9} r="1.15" fill="#ffcc00" />)}</>; break;
    case "se": design = <><rect width="36" height="36" fill="#006aa7" /><rect x="10" width="4" height="36" fill="#fecc00" /><rect y="16" width="36" height="4" fill="#fecc00" /></>; break;
    case "ru": design = <>{stripe(0, "#fff")}{stripe(12, "#0039a6")}{stripe(24, "#d52b1e")}</>; break;
    case "de": design = <>{stripe(0, "#000", 12)}{stripe(12, "#dd0000", 12)}{stripe(24, "#ffce00", 12)}</>; break;
    case "fi": design = <><rect width="36" height="36" fill="#fff" /><rect x="10" width="5" height="36" fill="#003580" /><rect y="15" width="36" height="5" fill="#003580" /></>; break;
    case "us": design = <>{Array.from({ length: 13 }, (_, i) => stripe(i * 36 / 13, i % 2 ? "#fff" : "#b22234", 36 / 13))}<rect width="17" height="19" fill="#3c3b6e" />{Array.from({ length: 3 }, (_, row) => Array.from({ length: 3 }, (_, col) => <circle key={`${row}-${col}`} cx={3 + col * 5} cy={3 + row * 6} r=".8" fill="#fff" />))}</>; break;
    case "tr": design = <><rect width="36" height="36" fill="#e30a17" /><circle cx="14" cy="18" r="8" fill="#fff" /><circle cx="17" cy="18" r="6.3" fill="#e30a17" /><path d="m23 13 1.5 3.5 3.8.2-3 2.3 1 3.7-3.3-2.1-3.2 2.1 1-3.7-3-2.3 3.8-.2z" fill="#fff" /></>; break;
    case "kz": design = <><rect width="36" height="36" fill="#00afca" /><circle cx="18" cy="18" r="5" fill="#f7d116" /><path d="M18 10v16M10 18h16M12.3 12.3l11.4 11.4m0-11.4L12.3 23.7" stroke="#f7d116" strokeWidth="1.2" /><rect x="5" width="2" height="36" fill="#f7d116" /></>; break;
    case "gb": design = <><rect width="36" height="36" fill="#012169" /><path d="m0 0 36 36M36 0 0 36" stroke="#fff" strokeWidth="8" /><path d="m0 0 36 36M36 0 0 36" stroke="#c8102e" strokeWidth="3.5" /><path d="M18 0v36M0 18h36" stroke="#fff" strokeWidth="11" /><path d="M18 0v36M0 18h36" stroke="#c8102e" strokeWidth="6" /></>; break;
    case "pl": design = <>{stripe(0, "#fff", 18)}{stripe(18, "#dc143c", 18)}</>; break;
    case "fr": design = <>{vertical(0, "#0055a4", 12)}{vertical(12, "#fff", 12)}{vertical(24, "#ef4135", 12)}</>; break;
    case "lv": design = <>{stripe(0, "#9e3039", 15)}{stripe(15, "#fff", 6)}{stripe(21, "#9e3039", 15)}</>; break;
    case "ee": design = <>{stripe(0, "#4891d9")}{stripe(12, "#000")}{stripe(24, "#fff")}</>; break;
    case "ch": design = <><rect width="36" height="36" fill="#d52b1e" /><rect x="15" y="7" width="6" height="22" rx="1" fill="#fff" /><rect x="7" y="15" width="22" height="6" rx="1" fill="#fff" /></>; break;
    case "jp": design = <><rect width="36" height="36" fill="#fff" /><circle cx="18" cy="18" r="8" fill="#bc002d" /></>; break;
    case "sg": design = <><rect width="36" height="18" fill="#ed2939" />{stripe(18, "#fff", 18)}<circle cx="11" cy="9" r="5" fill="#fff" /><circle cx="13" cy="9" r="4" fill="#ed2939" />{Array.from({ length: 5 }, (_, i) => <circle key={i} cx={17 + Math.cos((i * 72 - 90) * Math.PI / 180) * 3.3} cy={9 + Math.sin((i * 72 - 90) * Math.PI / 180) * 3.3} r=".85" fill="#fff" />)}</>; break;
  }
  const [failed, setFailed] = useState(false);
  return <span className="server-flag w-10 h-10 rounded-full flex items-center justify-center overflow-hidden shrink-0">{!code || failed ? <span>🌐</span> : design ? <svg className="w-10 h-10 rounded-full overflow-hidden shrink-0" viewBox="0 0 36 36" role="img" aria-label={code.toUpperCase()}><defs><clipPath id={clip}><circle cx="18" cy="18" r="18" /></clipPath></defs><g clipPath={`url(#${clip})`}>{design}</g></svg> : <img className="w-10 h-10 rounded-full overflow-hidden shrink-0" src={`https://flagcdn.com/w80/${code}.png`} alt={code.toUpperCase()} onError={() => setFailed(true)} />}</span>;
}
type Detector = { detect: (source: ImageBitmapSource) => Promise<{ rawValue: string }[]> };
type DetectorConstructor = new (options: { formats: string[] }) => Detector;
export default function VpnPage({ selectedId, onSelect, connected, isConnecting, onToggleConnection }: Props) {
  const [servers, setServers] = useState<VpnServer[]>(loadServers);
  const [subscriptionsState, setSubscriptionsState] = useState<Subscription[]>([]);
  const [index, setIndex] = useState(0);
  const [descriptionExpanded, setDescriptionExpanded] = useState(false);
  const [busy, setBusy] = useState(false);
  const [updated, setUpdated] = useState("");
  const { language, t: translate } = useLanguage();
  const en = language === "en";
  const t = (ru: string, english: string) => translate(ru, english);
  const [modal, setModal] = useState<"menu" | "link" | "manual" | "qr" | null>(null);
  const [addMenuPosition, setAddMenuPosition] = useState({ top: 0, left: 0 });
  const [addMenuOrigin, setAddMenuOrigin] = useState<HTMLElement | null>(null);
  const [serverMenu, setServerMenu] = useState<{ id: string; top: number; left: number } | null>(null);
  const [serverDetails, setServerDetails] = useState<VpnServer | null>(null);
  const [confirmDelete, setConfirmDelete] = useState<{ id: string; kind: "subscription" | "server" } | null>(null);
  const addButton = useRef<HTMLButtonElement>(null);
  const addMenu = useRef<HTMLDivElement>(null);
  const [error, setError] = useState("");
  const [link, setLink] = useState("");
  const [protocol, setProtocol] = useState("VLESS");
  const [address, setAddress] = useState("");
  const [port, setPort] = useState("");
  const [key, setKey] = useState("");
  const [scanning, setScanning] = useState(false);
  const video = useRef<HTMLVideoElement>(null);
  const startX = useRef<number | null>(null);
  const subscriptions = subscriptionsState.map(item => ({...item, english:item.name, traffic:item.total ? `${(((item.upload||0)+(item.download||0))/1024**3).toFixed(1)} / ${(item.total/1024**3).toFixed(1)} GB` : "", percent:item.total ? Math.min(100,Math.round(((item.upload||0)+(item.download||0))*100/item.total)) : 0 }));
  const activeSubIndex = subscriptions.length ? Math.min(Math.max(index, 0), subscriptions.length - 1) : 0;
  const current = subscriptions[activeSubIndex];
  const visibleServers = current ? servers.filter(server => server.group === current.id) : servers;
  const loadState = async () => { try { const state = await invoke<{servers: VpnServer[]; subscriptions: Subscription[]}>('get_vpn_state'); setServers(state.servers || []); setSubscriptionsState(state.subscriptions || []); } catch { /* Backend unavailable in browser preview. */ } };
  useEffect(() => { void loadState(); }, []);
  useEffect(() => { setIndex(value => subscriptions.length ? Math.min(Math.max(value, 0), subscriptions.length - 1) : 0); }, [subscriptions.length]);
  useEffect(() => { try { localStorage.setItem("altfy-vpn-servers", JSON.stringify(servers)); localStorage.setItem("altfy-vpn-groups-v1", "1"); } catch { /* storage unavailable */ } }, [servers]);
  useEffect(() => {
    if (modal !== "menu" && !serverMenu) return;
    const closeOutside = (event: PointerEvent) => {
      const target = event.target as Node;
      if (modal === "menu" && !addMenu.current?.contains(target) && !addButton.current?.contains(target) && !addMenuOrigin?.contains(target)) setModal(null);
      if (serverMenu && !(target instanceof Element && target.closest(`[data-server-menu="${serverMenu.id}"]`))) setServerMenu(null);
    };
    const closeEscape = (event: KeyboardEvent) => { if (event.key === "Escape") { setModal(null); setServerMenu(null); } };
    document.addEventListener("pointerdown", closeOutside);
    document.addEventListener("keydown", closeEscape);
    return () => { document.removeEventListener("pointerdown", closeOutside); document.removeEventListener("keydown", closeEscape); };
  }, [modal, serverMenu, addMenuOrigin]);
  useEffect(() => {
    if (!scanning || modal !== "qr") return;
    let stream: MediaStream | undefined;
    let frame = 0;
    let stopped = false;
    const run = async () => {
      const DetectorClass = (window as unknown as { BarcodeDetector?: DetectorConstructor }).BarcodeDetector;
      if (!DetectorClass) { setError(t("Сканирование QR не поддерживается этим устройством.", "QR scanning is not supported on this device.")); setScanning(false); return; }
      try {
        stream = await navigator.mediaDevices.getUserMedia({ video: { facingMode: "environment" } });
        if (stopped) { stream.getTracks().forEach(track => track.stop()); return; }
        if (!video.current) return;
        video.current.srcObject = stream;
        await video.current.play();
        const detector = new DetectorClass({ formats: ["qr_code"] });
        const scan = async () => {
          if (stopped || !video.current) return;
          try { const result = await detector.detect(video.current); if (result[0]?.rawValue) { setLink(result[0].rawValue); setScanning(false); setModal("link"); return; } } catch { /* wait for next frame */ }
          if (!stopped) frame = requestAnimationFrame(scan);
        };
        void scan();
      } catch { if (!stopped) { setError(t("Не удалось открыть камеру.", "Could not open the camera.")); setScanning(false); } }
    };
    void run();
    return () => { stopped = true; cancelAnimationFrame(frame); stream?.getTracks().forEach(track => track.stop()); };
  }, [scanning, modal, en]);
  const move = (step: number) => { if (subscriptions.length) setIndex(() => (activeSubIndex + step + subscriptions.length) % subscriptions.length); setDescriptionExpanded(false); };
  const open = (view: typeof modal) => { setError(""); setScanning(false); setModal(view); };
  const toggleAddMenu = (target?: HTMLElement) => {
    if (modal === "menu") { setModal(null); return; }
    const origin = target || addButton.current;
    setAddMenuOrigin(origin);
    const rect = origin?.getBoundingClientRect();
    if (rect) setAddMenuPosition({ top: rect.bottom + 8, left: Math.max(8, Math.min(rect.left, window.innerWidth - 250)) });
    open("menu");
  };
  const addLink = async (input: string) => {
    const value = input.trim();
    if (!/^(vless|vmess|trojan|ss|hy2|hysteria2|wg|wireguard|https?):\/\//i.test(value)) { setError(t("Введите корректную ссылку сервера или URL подписки.", "Enter a valid server link or subscription URL.")); return; }
    setBusy(true); try { const result = await invoke<{state:{servers:VpnServer[]; subscriptions:Subscription[]}}>('import_vpn_source', {input:value}); setServers(result.state.servers); setSubscriptionsState(result.state.subscriptions); setIndex(Math.max(0, result.state.subscriptions.length - 1)); setLink(""); open(null); window.dispatchEvent(new CustomEvent('altfy-toast',{detail:t('VPN-источник добавлен','VPN source imported')})); } catch(error) { setError(String(error)); } finally { setBusy(false); }
  };
  const addFromClipboard = async () => {
    setModal(null);
    try {
      const text = (await navigator.clipboard.readText()).trim();
      if (!text) { window.dispatchEvent(new CustomEvent('altfy-toast', { detail: t('Буфер обмена пуст', 'Clipboard is empty') })); return; }
      await addLink(text);
    } catch (error) { window.dispatchEvent(new CustomEvent('altfy-toast', { detail: String(error) })); }
  };
  const scanFile = async (file?: File) => {
    if (!file) return;
    const DetectorClass = (window as unknown as { BarcodeDetector?: DetectorConstructor }).BarcodeDetector;
    if (!DetectorClass) { setError(t("Сканирование QR не поддерживается этим устройством.", "QR scanning is not supported on this device.")); return; }
    try {
      const image = await createImageBitmap(file);
      try { const result = await new DetectorClass({ formats: ["qr_code"] }).detect(image); if (!result[0]?.rawValue) throw new Error(); setLink(result[0].rawValue); open("link"); } finally { image.close(); }
    } catch { setError(t("QR-код не найден на изображении.", "No QR code found in the image.")); }
  };
  const addManual = () => {
    if (!address.trim() || !key.trim() || !/^\d+$/.test(port) || Number(port) < 1 || Number(port) > 65535) { setError(t("Заполните адрес и ключ, укажите порт от 1 до 65535.", "Enter an address, key and port from 1 to 65535.")); return; }
    const scheme = protocol === "Shadowsocks" ? "ss" : protocol === "Hysteria2" ? "hysteria2" : protocol.toLowerCase();
    const source = `${scheme}://${encodeURIComponent(key.trim())}@${address.trim()}:${port}#${encodeURIComponent(address.trim())}`;
    void invoke<{state:{servers:VpnServer[]; subscriptions:Subscription[]}}>('import_vpn_source',{input:source}).then(result=>{setServers(result.state.servers);setSubscriptionsState(result.state.subscriptions);setIndex(Math.max(0,result.state.subscriptions.length-1));setAddress("");setPort("");setKey("");open(null);}).catch(error=>setError(String(error)));
  };
  const pingAll = async () => { setBusy(true); try { setServers(await invoke<VpnServer[]>('ping_vpn_servers')); } catch(error) { window.dispatchEvent(new CustomEvent('altfy-toast',{detail:String(error)})); } finally { setBusy(false); } };
  const refresh = async () => { const sub = current && subscriptionsState.find(item => item.id === current.id); if (!sub) return; setBusy(true); try { await invoke('update_vpn_subscription',{subId:sub.id}); await loadState(); setUpdated(new Date().toLocaleString(en ? "en-US" : "ru-RU", { day: "numeric", month: "short", hour: "2-digit", minute: "2-digit" })); } catch(error) { window.dispatchEvent(new CustomEvent('altfy-toast',{detail:String(error)})); } finally { setBusy(false); } };
  const card = (item: Subscription, active = false) => <article className={`subscription-card w-full min-w-full shrink-0 block ${active && descriptionExpanded ? "h-auto" : "h-[185px]"} flex flex-col justify-between`} style={{ background: "linear-gradient(135deg, rgba(139,92,246,.16), rgba(99,102,241,.08)), var(--card, var(--bg, #fff))", border: "1px solid rgba(139,92,246,.35)", boxShadow: "0 0 18px rgba(139,92,246,.12)" }}>
    <div className="subscription-heading"><span className="subscription-folder"><Folder size={17} /></span><span className="subscription-name"><strong>{en ? item.english : item.name}</strong><small>{updated && active ? `${t("Обновлено", "Updated")} · ${updated}` : t("VPN-подписка", "VPN subscription")}</small></span>{active && <><button className="refresh-button" onClick={refresh} disabled={busy || !subscriptionsState.some(sub => sub.id === item.id)} aria-label={t("Обновить", "Refresh")}><RefreshCw size={15} className={busy ? "vpn-spin" : ""} /></button><button className="refresh-button" onClick={() => setConfirmDelete({ id: item.id, kind: "subscription" })} aria-label={t("Удалить подписку", "Delete subscription")}><Trash2 size={15} /></button></>}</div>
    <span className="inline-flex rounded-full bg-violet-500/10 px-2 py-1 text-xs font-medium text-violet-500">{servers.filter(server => server.group === item.id).length} {t("серверов", "servers")}</span>
    {item.description && <div className="subscription-description flex items-end gap-1"><p className="text-xs" style={{ margin: 0, flex: 1, overflow: "hidden", display: "-webkit-box", WebkitBoxOrient: "vertical", WebkitLineClamp: active && descriptionExpanded ? "unset" : 2, maskImage: active && descriptionExpanded ? undefined : "linear-gradient(to bottom, #000 55%, transparent 100%)" }}>{item.description}</p>{active && <button onClick={() => setDescriptionExpanded(value => !value)} aria-expanded={descriptionExpanded} aria-label={descriptionExpanded ? t("Свернуть описание", "Collapse description") : t("Развернуть описание", "Expand description")}><ChevronRight size={16} style={{ transform: descriptionExpanded ? "rotate(-90deg)" : "rotate(90deg)", transition: "transform .2s ease" }} /></button>}</div>}
    {item.id !== "manual" ? <><div className="traffic-row"><span>{t("Трафик", "Traffic")}</span><strong>{item.traffic}</strong></div><div className="traffic-track" style={{ height: 4, borderRadius: 999, overflow: "hidden" }}><i style={{ width: `${item.percent}%`, background: "linear-gradient(90deg, #8b5cf6, #6366f1)" }} /></div><div className="subscription-foot"><span>{item.expire ? `${t("Истекает:", "Expires:")} ${new Date(item.expire * 1000).toLocaleDateString("ru-RU")}` : t("Истекает: не указано", "Expires: not specified")}</span><span>{item.percent}%</span></div></> : <div className="subscription-foot manual-foot">{t("Добавленные вручную серверы и ссылки", "Manually added servers and links")}</div>}
  </article>;
  return <section className="vpn-page">
    <div className="vpn-actions"><button ref={addButton} onClick={() => toggleAddMenu()}><Plus size={16} />{t("Добавить", "Add")}</button><button onClick={pingAll} disabled={busy}>{busy ? <LoaderCircle className="vpn-spin" size={16} /> : <Activity size={16} />}{t("Пинг всех", "Ping all")}</button>{modal === "menu" && createPortal(<div ref={addMenu} className="add-popover" role="menu" style={{ position: "fixed", zIndex: 100, top: addMenuPosition.top, left: addMenuPosition.left, width: 242, borderRadius: 16, padding: 8, background: "var(--card, var(--bg, #fff))", boxShadow: "0 12px 36px #0003", animation: "vpn-pop-in .18s ease-out" }}><div className="add-options">
      <button className="w-full flex items-center justify-start gap-3 px-4 py-2.5 text-left" onClick={addFromClipboard}><Clipboard className="w-5 h-5 shrink-0" /><span className="truncate text-left">{t("Из буфера обмена", "From clipboard")}</span></button>
      <button className="w-full flex items-center justify-start gap-3 px-4 py-2.5 text-left" onClick={() => open("qr")}><Camera className="w-5 h-5 shrink-0" /><span className="truncate text-left">{t("Сканировать QR / Фото", "Scan QR / Photo")}</span></button>
      <button className="w-full flex items-center justify-start gap-3 px-4 py-2.5 text-left" onClick={() => open("link")}><Link className="w-5 h-5 shrink-0" /><span className="truncate text-left">{t("Ввести ссылку / URL", "Enter link / URL")}</span></button>
      <button className="w-full flex items-center justify-start gap-3 px-4 py-2.5 text-left" onClick={() => open("manual")}><Server className="w-5 h-5 shrink-0" /><span className="truncate text-left">{t("Настроить вручную", "Set up manually")}</span></button>
      <button className="w-full flex items-center justify-start gap-3 px-4 py-2.5 text-left" onClick={() => open(null)}><svg className="w-5 h-5 shrink-0" viewBox="0 0 24 26" fill="none" aria-hidden="true"><path d="M12 1.5 21 5v7.2c0 5.3-3.8 9.6-9 12.3-5.2-2.7-9-7-9-12.3V5l9-3.5Z" stroke="currentColor" strokeWidth="1.5"/><ellipse cx="12" cy="12" rx="8" ry="3" transform="rotate(-30 12 12)" stroke="currentColor" strokeWidth="1.3" /></svg><span className="truncate text-left">AltVPN</span></button>
    </div></div>, document.body)}</div>
    <div className="flex items-center justify-between rounded-2xl border border-violet-500/20 px-4 py-3"><span className="text-sm font-medium">{t("Статус VPN", "VPN status")}</span><strong className={connected ? "text-emerald-500" : "opacity-60"}>{connected ? t("Подключено", "Connected") : t("Отключено", "Disconnected")}</strong></div>
    <button className="w-full rounded-2xl py-3 font-semibold text-white" style={{ background: "linear-gradient(90deg, #8b5cf6, #6366f1)" }} onClick={onToggleConnection} disabled={isConnecting}>{isConnecting ? t("Подключение...", "Connecting...") : connected ? t("Отключить", "Disconnect") : t("Подключить", "Connect")}</button>
    <div className="vpn-section-title">{t("ПОДПИСКИ", "SUBSCRIPTIONS")}</div>
    {subscriptions.length ? <div className="subscription-carousel w-full overflow-hidden">
      <div className="carousel-viewport w-full" onPointerDown={e => { if (!(e.target as HTMLElement).closest("button")) startX.current = e.clientX; }} onPointerUp={e => { if (startX.current !== null && Math.abs(e.clientX - startX.current) > 45) move(e.clientX < startX.current ? 1 : -1); startX.current = null; }} onPointerCancel={() => { startX.current = null; }}>
        {current && card(current, true)}
      </div>
      {subscriptions.length > 1 && <div className="flex items-center justify-center gap-2 mt-3">
        <button className="flex h-7 w-7 items-center justify-center rounded-full" onClick={() => move(-1)} aria-label={t("Предыдущая подписка", "Previous subscription")}><ChevronLeft size={16} /></button>
        {subscriptions.map((item, i) => <button key={item.id} className="rounded-full transition-all duration-300 ease-out" style={{ width: i === activeSubIndex ? 20 : 6, height: 6, background: i === activeSubIndex ? "#8b5cf6" : "rgba(139,92,246,.3)" }} onClick={() => { setIndex(Math.min(Math.max(i, 0), subscriptions.length - 1)); setDescriptionExpanded(false); }} aria-label={`${t("Подписка", "Subscription")}: ${en ? item.english : item.name}`} aria-current={i === activeSubIndex ? "true" : undefined} />)}
        <button className="flex h-7 w-7 items-center justify-center rounded-full" onClick={() => move(1)} aria-label={t("Следующая подписка", "Next subscription")}><ChevronRight size={16} /></button>
      </div>}
    </div> : <div className="subscription-empty-card"><span className="subscription-empty-icon"><Plus size={22} /></span><strong>{t("Добавить подписку", "Add subscription")}</strong><button onClick={event => toggleAddMenu(event.currentTarget)}><Plus size={15} />{t("Добавить подписку или ключ", "Add subscription or key")}</button></div>}
    <div className="vpn-section-title server-title">{t("СЕРВЕРЫ", "SERVERS")} <span>{visibleServers.length}</span></div>
    <div className="vpn-server-list">{visibleServers.map(server => <article key={server.id} className={`vpn-server ${selectedId === server.id ? "chosen" : ""}`}><button className="server-select" onClick={() => { const switching = connected && selectedId !== server.id; onSelect(server); void invoke('select_vpn_server',{id:server.id}).then(async () => { if (switching) { let routing = "system"; try { const saved = JSON.parse(localStorage.getItem("altfy-settings") || "{}"); if (["system", "direct_apps", "vpn_apps"].includes(saved.routing)) routing = saved.routing; } catch { /* storage unavailable */ } await invoke('connect_vpn',{id:server.id,routing}); window.dispatchEvent(new CustomEvent('altfy-toast',{detail:t(`Переключено на ${server.name}`,`Switched to ${server.name}`)})); } }).catch(error=>window.dispatchEvent(new CustomEvent('altfy-toast',{detail:String(error)}))); }}><Flag flag={server.flag} name={server.name} /><span className="server-info"><strong>{server.name}</strong><small>{server.protocol}</small></span><span className="server-ping">{server.ping > 0 ? <span className={server.ping < 70 ? "text-emerald-500" : server.ping <= 150 ? "text-amber-500" : "text-rose-500"}>{server.ping} ms</span> : "—"}</span>{selectedId === server.id && <span className="server-check"><Check size={14} /></span>}</button><button className="server-menu-trigger" data-server-menu={server.id} aria-label={t("Меню сервера", "Server menu")} onClick={event => { const rect = event.currentTarget.getBoundingClientRect(); setServerMenu(serverMenu?.id === server.id ? null : { id: server.id, top: rect.bottom + 4, left: Math.max(8, Math.min(rect.right - 190, window.innerWidth - 198)) }); }}><MoreVertical size={18} /></button>{serverMenu?.id === server.id && createPortal(<div className="server-context-menu" data-server-menu={server.id} role="menu" style={{ position: "fixed", top: serverMenu.top, left: serverMenu.left, zIndex: 110 }}><button onClick={async () => { setServerMenu(null); setBusy(true); try { setServers(await invoke<VpnServer[]>('ping_vpn_servers')); } catch(error) { window.dispatchEvent(new CustomEvent('altfy-toast',{detail:String(error)})); } finally { setBusy(false); } }}><Gauge size={15} />{t("Проверить пинг", "Check ping")}</button><button onClick={() => { void navigator.clipboard.writeText(server.key || "").then(() => window.dispatchEvent(new CustomEvent('altfy-toast',{detail:t("Ключ скопирован", "Key copied")}))).catch(error => window.dispatchEvent(new CustomEvent('altfy-toast',{detail:String(error)}))); setServerMenu(null); }}><Copy size={15} />{t("Скопировать ключ", "Copy key")}</button><button onClick={() => { setServerDetails(server); setServerMenu(null); }}><Settings2 size={15} />{t("Параметры сервера", "Server settings")}</button><button className="danger" onClick={() => { setServerMenu(null); setConfirmDelete({ id: server.id, kind: "server" }); }}><Trash2 size={15} />{t("Удалить", "Delete")}</button></div>, document.body)}</article>)}{!visibleServers.length && <div className="vpn-empty">{t("Здесь пока нет серверов", "No servers here yet")}</div>}</div>
    {serverDetails && createPortal(<div className="vpn-overlay" onClick={() => setServerDetails(null)}><div className="vpn-sheet" role="dialog" aria-modal="true" onClick={event => event.stopPropagation()}><div className="sheet-heading"><strong>{t("Параметры сервера", "Server settings")}</strong><button onClick={() => setServerDetails(null)} aria-label={t("Закрыть", "Close")}><X size={18} /></button></div><div className="vpn-server-details"><strong>{serverDetails.name}</strong><span>{serverDetails.protocol}</span><span>{serverDetails.address}:{serverDetails.port}</span><textarea readOnly value={serverDetails.key || ""} /></div></div></div>, document.body)}
    {modal && modal !== "menu" && createPortal(<div className="vpn-overlay" onClick={() => open(null)}><div className="vpn-sheet" role="dialog" aria-modal="true" aria-label={t("Добавить VPN", "Add VPN")} onClick={e => e.stopPropagation()}><div className="sheet-heading"><strong>{modal === "link" ? t("Ссылка или URL подписки", "Link or subscription URL") : modal === "manual" ? t("Настроить сервер", "Set up server") : t("Сканировать QR-код", "Scan QR code")}</strong><button onClick={() => open(null)} aria-label={t("Закрыть", "Close")}><X size={18} /></button></div>
      {modal === "link" && <form className="vpn-form" onSubmit={e => { e.preventDefault(); addLink(link); }}><label>{t("Ссылка", "Link")}<textarea value={link} onChange={e => setLink(e.target.value)} placeholder="vless://… / https://…" /></label><small>{t("Поддерживаются VPN-ссылки и URL HTTPS-подписок.", "VPN links and HTTPS subscription URLs are supported.")}</small><button className="vpn-submit">{t("Сохранить", "Save")}</button></form>}
      {modal === "manual" && <form className="vpn-form" onSubmit={e => { e.preventDefault(); addManual(); }}><label>{t("Протокол", "Protocol")}<CustomSelect value={protocol} ariaLabel={t("Протокол", "Protocol")} options={["VLESS", "VMess", "Trojan", "WireGuard", "Shadowsocks", "Hysteria2"].map(value => ({ value, label: value }))} onChange={setProtocol} /></label><label>{t("Адрес", "Address")}<input value={address} onChange={e => setAddress(e.target.value)} placeholder="example.com" /></label><label>{t("Порт", "Port")}<input value={port} onChange={e => setPort(e.target.value)} inputMode="numeric" placeholder="443" /></label><label>{t("Ключ", "Key")}<input value={key} onChange={e => setKey(e.target.value)} type="password" autoComplete="off" /></label><button className="vpn-submit">{t("Сохранить", "Save")}</button></form>}
      {modal === "qr" && <div className="vpn-form">{scanning && <video ref={video} className="qr-video" muted playsInline />}<button className="qr-action" onClick={() => setScanning(!scanning)}><Camera size={17} />{scanning ? t("Остановить камеру", "Stop camera") : t("Открыть камеру", "Open camera")}</button><label className="qr-action"><ImagePlus size={17} />{t("Загрузить фото", "Upload photo")}<input type="file" accept="image/*" onChange={e => { void scanFile(e.target.files?.[0]); e.target.value = ""; }} /></label></div>}
      {error && <div className="vpn-error" role="alert">{error}</div>}
    </div></div>, document.body)}
  {confirmDelete && createPortal(<div className="fixed inset-0 z-[120] flex items-center justify-center bg-black/50 p-4 backdrop-blur-sm" onClick={() => setConfirmDelete(null)}><div className="w-full max-w-[360px] rounded-3xl border border-violet-500/20 bg-[var(--card,var(--bg,#fff))] p-6 shadow-2xl" role="alertdialog" aria-modal="true" onClick={event => event.stopPropagation()}><h2 className="mb-2 text-lg font-semibold">{confirmDelete.kind === "subscription" ? t("Удалить подписку", "Delete subscription") : t("Удалить сервер", "Delete server")}</h2><p className="mb-6 text-sm opacity-75">{confirmDelete.kind === "subscription" ? t("Вы уверены, что хотите удалить эту подписку?", "Are you sure you want to delete this subscription?") : t("Вы уверены, что хотите удалить этот сервер?", "Are you sure you want to delete this server?")}</p><div className="flex justify-end gap-3"><button className="rounded-2xl px-4 py-2 text-sm" onClick={() => setConfirmDelete(null)}>{t("Отмена", "Cancel")}</button><button className="rounded-2xl bg-rose-600 px-4 py-2 text-sm font-medium text-white hover:bg-rose-700" onClick={() => { const pending = confirmDelete; setConfirmDelete(null); void invoke('delete_vpn_item', { id: pending.id }).then(async () => { if (pending.kind === "server" && selectedId === pending.id) onSelect(null); await loadState(); if (pending.kind === "subscription") setIndex(0); }).catch(error => window.dispatchEvent(new CustomEvent('altfy-toast', { detail: String(error) }))); }}>{t("Удалить", "Delete")}</button></div></div></div>, document.body)}
</section>;
}
