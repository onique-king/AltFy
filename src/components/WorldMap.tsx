export default function WorldMap() {
  return (
    <svg className="world-map" viewBox="0 0 420 250" fill="none" aria-hidden="true">
      <defs>
        <pattern id="dots" width="7" height="7" patternUnits="userSpaceOnUse"><circle cx="2" cy="2" r="1.2" fill="currentColor" /></pattern>
        <linearGradient id="route" x1="0" x2="1" y1="0" y2="1"><stop stopColor="#8B5CF6" stopOpacity="0" /><stop offset=".5" stopColor="#A78BFA" /><stop offset="1" stopColor="#8B5CF6" stopOpacity="0" /></linearGradient>
      </defs>
      <g fill="url(#dots)">
        <path d="M20 55 42 36 77 29 96 35 108 29 131 39 126 52 112 56 107 70 94 73 87 88 73 92 67 108 55 101 49 87 32 81 24 68Z" />
        <path d="M100 104 119 109 133 121 137 138 130 152 126 171 115 192 107 205 101 189 102 171 94 156 90 135 83 120Z" />
        <path d="M177 48 187 38 202 41 210 33 226 38 237 34 249 42 269 35 291 39 301 31 327 36 345 44 366 43 390 55 392 67 378 73 362 70 351 82 336 80 324 91 310 87 300 99 286 92 277 100 262 92 250 99 239 86 224 83 214 70 201 75 188 69 177 70 169 60Z" />
        <path d="M184 83 203 79 218 86 229 100 230 117 222 132 215 151 201 169 190 160 183 142 173 128 170 105Z" />
        <path d="M317 151 337 142 359 147 373 161 367 175 348 181 330 175 317 165Z M154 30 161 18 176 16 182 24 174 35 161 38Z M281 105 289 108 296 128 290 136 283 123Z M377 182 382 186 379 196 374 192Z" />
      </g>
      <g className="map-routes" stroke="url(#route)" strokeWidth="1.2"><path d="M74 78 Q139 3 213 72" /><path d="M213 72 Q265 15 339 69" /><path d="M213 72 Q263 91 344 157" /><path d="M112 137 Q143 83 213 72" /></g>
      <g fill="#A78BFA"><circle cx="74" cy="78" r="2.5" /><circle cx="213" cy="72" r="3" /><circle cx="339" cy="69" r="2.5" /><circle cx="344" cy="157" r="2.5" /><circle cx="112" cy="137" r="2.5" /></g>
    </svg>
  );
}
