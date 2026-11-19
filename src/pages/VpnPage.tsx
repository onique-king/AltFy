import { useEffect, useRef, useState } from "react";
import "../components/VpnPage.css";
import { Activity, Camera, Check, ChevronLeft, ChevronRight, Clipboard, Folder, ImagePlus, Link, LoaderCircle, Plus, RefreshCw, Server, Trash2, X } from "lucide-react";

export type VpnServer = { id: string; flag: string; name: string; protocol: string; ping: number; group?: string; address?: string; port?: string; key?: string; source?: string };
type Props = { selectedId: string | null; onSelect: (server: VpnServer) => void };
type Subscription = { id: string; name: string; english: string; traffic: string; percent: number; description?: string };
const subscriptions: Subscription[] = [
  { id: "main", name: "Моя подписка", english: "My subscription", traffic: "42.8 / 100 GB", percent: 43, description: "Подписка включает стабильные серверы в Европе с низкой задержкой, автоматическим обновлением конфигураций и удобным доступом к защищённому соединению в поездках и дома." },
  { id: "travel", name: "Путешествия", english: "Travel", traffic: "18 / 50 GB", percent: 36 },
  { id: "work", name: "Работа", english: "Work", traffic: "12 / 100 GB", percent: 12 },
  { id: "manual", name: "Без подписки / Ручные", english: "No subscription / Manual", traffic: "", percent: 0 },
];
const demos: VpnServer[] = [
  { id: "nl", flag: "nl", name: "Нидерланды · Амстердам", protocol: "VLESS • Reality", ping: 34, group: "main" },
  { id: "de", flag: "de", name: "Германия · Франкфурт", protocol: "WireGuard", ping: 48, group: "main" },
  { id: "fi", flag: "fi", name: "Финляндия · Хельсинки", protocol: "VLESS • Reality", ping: 61, group: "main" },
  { id: "travel-us", flag: "us", name: "США · Нью-Йорк", protocol: "Trojan", ping: 92, group: "travel" },
  { id: "travel-jp", flag: "jp", name: "Япония · Токио", protocol: "Hysteria2", ping: 145, group: "travel" },
  { id: "work-gb", flag: "gb", name: "Великобритания · Лондон", protocol: "VLESS", ping: 55, group: "work" },
  { id: "work-ch", flag: "ch", name: "Швейцария · Цюрих", protocol: "WireGuard", ping: 62, group: "work" },
];
function loadServers(): VpnServer[] {
  try {
    const value = JSON.parse(localStorage.getItem("altfy-vpn-servers") || "null");
    if (Array.isArray(value)) {
      const migrated = value.map((server: VpnServer) => ({ ...server, group: server.group || (demos.some(d => d.id === server.id) ? "main" : "manual") }));
      if (!localStorage.getItem("altfy-vpn-groups-v1")) return [...migrated, ...demos.filter(d => d.group !== "main" && !migrated.some((s: VpnServer) => s.id === d.id))];
      return migrated;
    }
  } catch { /* storage unavailable */ }
  return demos;
}
function Flag({ flag }: { flag: string }) {
  const [failed, setFailed] = useState(false);
  const emojiCodes: Record<string, string> = { "🇳🇱": "nl", "🇩🇪": "de", "🇫🇮": "fi" };
  const code = emojiCodes[flag] || flag.toLowerCase();
  return <span className="server-flag">{!failed && /^[a-z]{2}$/.test(code) ? <img src={`https://flagcdn.com/w80/${code}.png`} alt={code.toUpperCase()} onError={() => setFailed(true)} /> : <span>🌐</span>}</span>;
}
type Detector = { detect: (source: ImageBitmapSource) => Promise<{ rawValue: string }[]> };
type DetectorConstructor = new (options: { formats: string[] }) => Detector;
export default function VpnPage({ selectedId, onSelect }: Props) {
  const [servers, setServers] = useState<VpnServer[]>(loadServers);
  const [index, setIndex] = useState(0);
  const [descriptionExpanded, setDescriptionExpanded] = useState(false);
  const [busy, setBusy] = useState(false);
  const [updated, setUpdated] = useState("Сегодня, 12:40");
  const [en, setEn] = useState(false);
  const [modal, setModal] = useState<"menu" | "link" | "manual" | "qr" | null>(null);
  const [error, setError] = useState("");
  const [link, setLink] = useState("");
  const [protocol, setProtocol] = useState("VLESS");
  const [address, setAddress] = useState("");
  const [port, setPort] = useState("");
  const [key, setKey] = useState("");
  const [scanning, setScanning] = useState(false);
  const video = useRef<HTMLVideoElement>(null);
  const startX = useRef<number | null>(null);
  const t = (ru: string, english: string) => en ? english : ru;
  const current = subscriptions[index];
  const visibleServers = servers.filter(server => server.group === current.id);
  useEffect(() => { try { const saved = JSON.parse(localStorage.getItem("altfy-settings") || "{}"); setEn(saved.language === "en"); } catch { /* storage unavailable */ } }, []);
  useEffect(() => { try { localStorage.setItem("altfy-vpn-servers", JSON.stringify(servers)); localStorage.setItem("altfy-vpn-groups-v1", "1"); } catch { /* storage unavailable */ } }, [servers]);
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
  const move = (step: number) => { setIndex(i => (i + step + subscriptions.length) % subscriptions.length); setDescriptionExpanded(false); };
  const open = (view: typeof modal) => { setError(""); setScanning(false); setModal(view); };
  const addLink = (input: string) => {
    const value = input.trim();
    if (!/^(vless|vmess|trojan|ss|hy2|hysteria2|wg|wireguard|https?):\/\//i.test(value)) { setError(t("Введите корректную ссылку сервера или URL подписки.", "Enter a valid server link or subscription URL.")); return; }
    const scheme = value.split(":")[0].toUpperCase();
    const server: VpnServer = { id: `custom-${Date.now()}-${Math.random()}`, flag: "🌐", name: /^https?:/i.test(value) ? t("URL подписки", "Subscription URL") : t("Новый сервер", "New server"), protocol: scheme, ping:  0, group: "manual", source: value };
    setServers(items => [server, ...items]); setIndex(subscriptions.length - 1); setLink(""); open(null);
  };
  const addFromClipboard = async () => {
    try { const text = await navigator.clipboard.readText(); setLink(text); open("link"); } catch { setLink(""); open("link"); }
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
    const server: VpnServer = { id: `custom-${Date.now()}-${Math.random()}`, flag: "🌐", name: address.trim(), protocol, ping: 0, group: "manual", address: address.trim(), port, key: key.trim() };
    setServers(items => [server, ...items]); setIndex(subscriptions.length - 1); setAddress(""); setPort(""); setKey(""); open(null);
  };
  const pingAll = () => { setBusy(true); window.setTimeout(() => { setServers(items => items.map((item, i) => item.group === current.id ? { ...item, ping: [34, 48, 61][i % 3] + Math.floor(Math.random() * 14) - 7 } : item)); setBusy(false); }, 1250); };
  const refresh = () => { setBusy(true); window.setTimeout(() => { setUpdated(new Date().toLocaleString(en ? "en-US" : "ru-RU", { day: "numeric", month: "short", hour: "2-digit", minute: "2-digit" })); setBusy(false); }, 900); };
  const card = (item: Subscription, active = false) => <article className={`subscription-card ${active && descriptionExpanded ? "h-auto" : "h-[185px]"} flex flex-col justify-between`} style={{ background: "linear-gradient(135deg, rgba(139,92,246,.16), rgba(99,102,241,.08)), var(--card, var(--bg, #fff))", border: "1px solid rgba(139,92,246,.35)", boxShadow: "0 0 18px rgba(139,92,246,.12)" }}>
    <div className="subscription-heading"><span className="subscription-folder"><Folder size={17} /></span><span className="subscription-name"><strong>{en ? item.english : item.name}</strong><small>{item.id === "manual" ? t("Личные серверы", "Personal servers") : `${t("Обновлено", "Updated")} · ${updated}`}</small></span>{active && item.id !== "manual" && <button className="refresh-button" onClick={refresh} disabled={busy} aria-label={t("Обновить", "Refresh")}><RefreshCw size={15} className={busy ? "vpn-spin" : ""} /></button>}</div>
    <span className="inline-flex rounded-full bg-violet-500/10 px-2 py-1 text-xs font-medium text-violet-500">{servers.filter(server => server.group === item.id).length} {t("серверов", "servers")}</span>
    {item.description && <div className="subscription-description flex items-end gap-1"><p className="text-xs" style={{ margin: 0, flex: 1, overflow: "hidden", display: "-webkit-box", WebkitBoxOrient: "vertical", WebkitLineClamp: active && descriptionExpanded ? "unset" : 2, maskImage: active && descriptionExpanded ? undefined : "linear-gradient(to bottom, #000 55%, transparent 100%)" }}>{en ? "Includes reliable European servers with low latency, automatic configuration updates, and convenient access to a secure connection at home and while travelling." : item.description}</p>{active && <button onClick={() => setDescriptionExpanded(value => !value)} aria-expanded={descriptionExpanded} aria-label={descriptionExpanded ? t("Свернуть описание", "Collapse description") : t("Развернуть описание", "Expand description")}><ChevronRight size={16} style={{ transform: descriptionExpanded ? "rotate(-90deg)" : "rotate(90deg)", transition: "transform .2s ease" }} /></button>}</div>}
    {item.id !== "manual" ? <><div className="traffic-row"><span>{t("Трафик", "Traffic")}</span><strong>{item.traffic}</strong></div><div className="traffic-track" style={{ height: 4, borderRadius: 999, overflow: "hidden" }}><i style={{ width: `${item.percent}%`, background: "linear-gradient(90deg, #8b5cf6, #6366f1)" }} /></div><div className="subscription-foot"><span>{t("Истекает: не указано", "Expires: not specified")}</span><span>{item.percent}%</span></div></> : <div className="subscription-foot manual-foot">{t("Добавленные вручную серверы и ссылки", "Manually added servers and links")}</div>}
  </article>;
  return <section className="vpn-page">
    <div className="vpn-actions" style={{ position: "relative" }}><button onClick={() => open(modal === "menu" ? null : "menu")}><Plus size={16} />{t("Добавить", "Add")}</button><button onClick={pingAll} disabled={busy}>{busy ? <LoaderCircle className="vpn-spin" size={16} /> : <Activity size={16} />}{t("Пинг всех", "Ping all")}</button>{modal === "menu" && <div className="add-popover w-max min-w-[230px]" role="menu" style={{ position: "absolute", zIndex: 20, top: "calc(100% + 8px)", left: 0, width: "max-content", minWidth: 230, borderRadius: 16, padding: 8, background: "var(--card, var(--bg, #fff))", boxShadow: "0 12px 36px #0003", animation: "vpn-pop-in .18s ease-out" }}><div className="add-options">
      <button className="w-full flex items-center justify-start gap-3 px-4 py-2.5 text-left" onClick={addFromClipboard}><Clipboard className="w-5 h-5 shrink-0" /><span className="truncate text-left">{t("Из буфера обмена", "From clipboard")}</span></button>
      <button className="w-full flex items-center justify-start gap-3 px-4 py-2.5 text-left" onClick={() => open("qr")}><Camera className="w-5 h-5 shrink-0" /><span className="truncate text-left">{t("Сканировать QR / Фото", "Scan QR / Photo")}</span></button>
      <button className="w-full flex items-center justify-start gap-3 px-4 py-2.5 text-left" onClick={() => open("link")}><Link className="w-5 h-5 shrink-0" /><span className="truncate text-left">{t("Ввести ссылку / URL", "Enter link / URL")}</span></button>
      <button className="w-full flex items-center justify-start gap-3 px-4 py-2.5 text-left" onClick={() => open("manual")}><Server className="w-5 h-5 shrink-0" /><span className="truncate text-left">{t("Настроить вручную", "Set up manually")}</span></button>
      <button className="w-full flex items-center justify-start gap-3 px-4 py-2.5 text-left" onClick={() => open(null)}><svg className="w-5 h-5 shrink-0" viewBox="0 0 24 26" fill="none" aria-hidden="true"><path d="M12 1.5 21 5v7.2c0 5.3-3.8 9.6-9 12.3-5.2-2.7-9-7-9-12.3V5l9-3.5Z" stroke="currentColor" strokeWidth="1.5"/><ellipse cx="12" cy="12" rx="8" ry="3" transform="rotate(-30 12 12)" stroke="currentColor" strokeWidth="1.3" /></svg><span className="truncate text-left">AltVPN</span></button>
    </div></div>}</div>
    <div className="vpn-section-title">{t("ПОДПИСКИ", "SUBSCRIPTIONS")}</div>
    <div className="subscription-carousel" style={{ display: "block" }}>
      <div className="carousel-viewport" style={{ position: "relative", overflow: "hidden" }} onPointerDown={e => { if (!(e.target as HTMLElement).closest("button")) startX.current = e.clientX; }} onPointerUp={e => { if (startX.current !== null && Math.abs(e.clientX - startX.current) > 45) move(e.clientX < startX.current ? 1 : -1); startX.current = null; }} onPointerCancel={() => { startX.current = null; }}>
        <div style={{ width: "82%", margin: "0 auto", visibility: "hidden", pointerEvents: "none" }} aria-hidden="true">{card(current, true)}</div>
        {subscriptions.map((item, i) => {
          const offset = (i - index + subscriptions.length) % subscriptions.length;
          const position = offset === subscriptions.length - 1 ? -1 : offset;
          const active = position === 0;
          return <div key={item.id} className="transition-all duration-300 ease-out" aria-hidden={!active} style={{ position: "absolute", top: 0, left: "9%", width: "82%", transform: `translateX(${position * 106}%) scale(${active ? 1 : .95})`, opacity: active ? 1 : position === -1 || position === 1 ? .5 : 0, zIndex: active ? 2 : 1, pointerEvents: active ? "auto" : "none", transition: "all 300ms ease-out" }}>{card(item, active)}</div>;
        })}
      </div>
      <div className="flex items-center justify-center gap-2 mt-3">
        <button className="flex h-7 w-7 items-center justify-center rounded-full" onClick={() => move(-1)} aria-label={t("Предыдущая подписка", "Previous subscription")}><ChevronLeft size={16} /></button>
        {subscriptions.map((item, i) => <button key={item.id} className="rounded-full transition-all duration-300 ease-out" style={{ width: i === index ? 20 : 6, height: 6, background: i === index ? "#8b5cf6" : "rgba(139,92,246,.3)" }} onClick={() => { setIndex(i); setDescriptionExpanded(false); }} aria-label={`${t("Подписка", "Subscription")}: ${en ? item.english : item.name}`} aria-current={i === index ? "true" : undefined} />)}
        <button className="flex h-7 w-7 items-center justify-center rounded-full" onClick={() => move(1)} aria-label={t("Следующая подписка", "Next subscription")}><ChevronRight size={16} /></button>
      </div>
    </div>
    <div className="vpn-section-title server-title">{t("СЕРВЕРЫ", "SERVERS")} <span>{visibleServers.length}</span></div>
    <div className="vpn-server-list">{visibleServers.map(server => <article key={server.id} className={`vpn-server ${selectedId === server.id ? "chosen" : ""}`}><button className="server-select" onClick={() => onSelect(server)}><Flag flag={server.flag} /><span className="server-info"><strong>{server.name}</strong><small>{server.protocol}</small></span><span className="server-ping">{server.ping ? <span className={server.ping < 70 ? "text-emerald-500" : server.ping <= 150 ? "text-amber-500" : "text-rose-500"}>{server.ping} ms</span> : "—"}</span>{selectedId === server.id && <span className="server-check"><Check size={14} /></span>}</button>{selectedId !== server.id && <button className="server-delete" aria-label={t("Удалить сервер", "Delete server")} onClick={() => setServers(items => items.filter(item => item.id !== server.id))}><Trash2 size={15} /></button>}</article>)}{!visibleServers.length && <div className="vpn-empty">{t("Здесь пока нет серверов", "No servers here yet")}</div>}</div>
    {modal && modal !== "menu" && <div className="vpn-overlay" onClick={() => open(null)}><div className="vpn-sheet" role="dialog" aria-modal="true" aria-label={t("Добавить VPN", "Add VPN")} onClick={e => e.stopPropagation()}><div className="sheet-heading"><strong>{modal === "link" ? t("Ссылка или URL подписки", "Link or subscription URL") : modal === "manual" ? t("Настроить сервер", "Set up server") : t("Сканировать QR-код", "Scan QR code")}</strong><button onClick={() => open(null)} aria-label={t("Закрыть", "Close")}><X size={18} /></button></div>
      {modal === "link" && <form className="vpn-form" onSubmit={e => { e.preventDefault(); addLink(link); }}><label>{t("Ссылка", "Link")}<textarea value={link} onChange={e => setLink(e.target.value)} placeholder="vless://… / https://…" /></label><small>{t("URL сохраняется как ссылка; загрузка подписки пока не подключена.", "URLs are saved as links; subscription fetching is not connected yet.")}</small><button className="vpn-submit">{t("Сохранить", "Save")}</button></form>}
      {modal === "manual" && <form className="vpn-form" onSubmit={e => { e.preventDefault(); addManual(); }}><label>{t("Протокол", "Protocol")}<select value={protocol} onChange={e => setProtocol(e.target.value)}>{["VLESS", "VMess", "Trojan", "WireGuard", "Shadowsocks", "Hysteria2"].map(p => <option key={p}>{p}</option>)}</select></label><label>{t("Адрес", "Address")}<input value={address} onChange={e => setAddress(e.target.value)} placeholder="example.com" /></label><label>{t("Порт", "Port")}<input value={port} onChange={e => setPort(e.target.value)} inputMode="numeric" placeholder="443" /></label><label>{t("Ключ", "Key")}<input value={key} onChange={e => setKey(e.target.value)} type="password" autoComplete="off" /></label><button className="vpn-submit">{t("Сохранить", "Save")}</button></form>}
      {modal === "qr" && <div className="vpn-form">{scanning && <video ref={video} className="qr-video" muted playsInline />}<button className="qr-action" onClick={() => setScanning(!scanning)}><Camera size={17} />{scanning ? t("Остановить камеру", "Stop camera") : t("Открыть камеру", "Open camera")}</button><label className="qr-action"><ImagePlus size={17} />{t("Загрузить фото", "Upload photo")}<input type="file" accept="image/*" onChange={e => { void scanFile(e.target.files?.[0]); e.target.value = ""; }} /></label></div>}
      {error && <div className="vpn-error" role="alert">{error}</div>}
    </div></div>}
  </section>;
}
