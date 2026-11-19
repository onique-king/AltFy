# Project Rules & Architecture
- App Name: AltFy
- Stack: Tauri v2, React 18, TypeScript, Tailwind CSS, Lucide Icons, Rust backend.
- License: MIT.
- Theme: Light & Dark mode. Primary accent: Violet (#8B5CF6 / #7C3AED).
- Layout: Compact responsive window (420x740 on desktop), mobile-first for future Android/iOS/Linux ports.
- Header: Left = Settings gear icon; Center = Logo placeholder (AltFy); Right = Theme toggle button (Sun/Moon icons).
- Navigation: Floating bottom rounded capsule with 5 tabs: Home, VPN, DPI, TG-WS, Info.
- Settings Design: Grouped vertical list style (inspired by Happ client) with toggles, dropdowns, TUN/Proxy switch, fragmentation settings, routing rules, and HWID transmission toggle for VPN subscriptions.
- Core Engines (src-tauri/binaries/): sing-box, winws (zapret), tg-ws-proxy.
- Token Economy Rules:
  1. Output ONLY code and brief file paths. No conversational filler or verbose code comments.
  2. Edit existing files surgically; avoid full-file rewrites.
  3. Keep UI components in `src/components/` and pages in `src/pages/`.