// shared.jsx — themeable playing-card primitive + helpers shared by all
// three visual directions. Exported to window for the screen files.

const SUIT = { S: '♠', H: '♥', D: '♦', C: '♣' };
const isRed = (s) => s === 'H' || s === 'D';

// Special-card metadata (single-card rules from the rules panel).
const SPECIAL = {
  '2':  { tag: 'RESET',  blurb: 'play anything next' },
  '3':  { tag: 'GHOST',  blurb: 'pile unchanged' },
  '7':  { tag: 'UNDER',  blurb: 'next must be ≤ 7' },
  '10': { tag: 'BURN',   blurb: 'burns the pile' },
};

// Per-skin token tables. Card visuals (and the card-design sheet) read from
// here so in-game cards always match the spec sheet.
const SKINS = {
  A: {
    paper: '#fbf8ef', ink: '#171720', red: '#e23b56', radius: 9,
    rankFont: '"Rubik", system-ui, sans-serif', rankWeight: 800,
    special: { '2': '#27c4d8', '3': '#8b93a7', '7': '#f4b740', '10': '#ff5a3c' },
  },
  B: {
    paper: '#e8e1d0', ink: '#222734', red: '#9b2f37', radius: 5,
    rankFont: '"Cinzel", serif', rankWeight: 600,
    special: { '2': '#6fb3c9', '3': '#9aa3b2', '7': '#c9a24b', '10': '#b4413e' },
  },
  C: {
    paper: '#e4e7ea', ink: '#1b1e22', red: '#d23a2e', radius: 2,
    rankFont: '"Share Tech Mono", monospace', rankWeight: 400,
    special: { '2': '#3fb6a8', '3': '#8a949c', '7': '#f2a23b', '10': '#d23a2e' },
  },
};

// ── Card backs, one per skin ───────────────────────────────────
function BackA({ r }) {
  return (
    <div style={{ position: 'absolute', inset: 0, borderRadius: r,
      background: 'linear-gradient(150deg,#241d4a,#15123a 60%,#0e0b27)',
      boxShadow: 'inset 0 0 0 2px rgba(255,255,255,.06), inset 0 0 0 4px #1a1640' }}>
      <div style={{ position: 'absolute', inset: 5, borderRadius: r - 2,
        border: '1.5px solid rgba(124,108,247,.45)',
        backgroundImage: 'repeating-linear-gradient(45deg,transparent 0 7px,rgba(124,108,247,.13) 7px 8px),repeating-linear-gradient(-45deg,transparent 0 7px,rgba(38,210,217,.10) 7px 8px)' }} />
      <div style={{ position: 'absolute', inset: 0, display: 'grid', placeItems: 'center' }}>
        <div style={{ width: '46%', aspectRatio: '1', borderRadius: '50%', display: 'grid', placeItems: 'center',
          background: 'radial-gradient(circle at 50% 35%,#3a2f7a,#1b1546)',
          boxShadow: '0 0 18px rgba(124,108,247,.5), inset 0 0 0 1.5px rgba(124,108,247,.6)',
          color: '#cfc6ff', fontFamily: '"Silkscreen", monospace', fontSize: '0.9em' }}>S</div>
      </div>
    </div>
  );
}
function BackB({ r }) {
  return (
    <div style={{ position: 'absolute', inset: 0, borderRadius: r,
      background: 'radial-gradient(120% 90% at 50% 0%,#16243a,#0b1018 70%)',
      boxShadow: 'inset 0 0 0 1px #36506b' }}>
      <div style={{ position: 'absolute', inset: 4, borderRadius: r - 1, border: '1px solid rgba(150,196,214,.35)' }} />
      <div style={{ position: 'absolute', inset: 0, display: 'grid', placeItems: 'center' }}>
        <div style={{ width: '50%', aspectRatio: '1', borderRadius: '50% 50% 48% 48%', position: 'relative',
          background: 'radial-gradient(circle at 50% 40%,#cdbf9e,#8d7f63)',
          boxShadow: '0 0 22px rgba(140,200,220,.45)' }}>
          <div style={{ position: 'absolute', top: '34%', left: '24%', width: '18%', height: '12%', borderRadius: '50%', background: '#0b1018' }} />
          <div style={{ position: 'absolute', top: '34%', right: '24%', width: '18%', height: '12%', borderRadius: '50%', background: '#0b1018' }} />
        </div>
      </div>
    </div>
  );
}
function BackC({ r }) {
  return (
    <div style={{ position: 'absolute', inset: 0, borderRadius: r,
      background: '#101316',
      boxShadow: 'inset 0 0 0 1px #2c3137' }}>
      <div style={{ position: 'absolute', inset: 0, borderRadius: r,
        backgroundImage: 'linear-gradient(rgba(242,162,59,.10) 1px,transparent 1px),linear-gradient(90deg,rgba(242,162,59,.10) 1px,transparent 1px)',
        backgroundSize: '11px 11px' }} />
      <div style={{ position: 'absolute', top: 4, left: 5, right: 5, height: 4,
        background: 'repeating-linear-gradient(45deg,#f2a23b 0 6px,#101316 6px 12px)' }} />
      <div style={{ position: 'absolute', inset: 0, display: 'grid', placeItems: 'center' }}>
        <div style={{ width: '44%', aspectRatio: '1', borderRadius: '50%', display: 'grid', placeItems: 'center',
          border: '2px solid #f2a23b', color: '#f2a23b', fontFamily: '"Share Tech Mono",monospace', fontSize: '1.1em',
          boxShadow: '0 0 12px rgba(242,162,59,.4)' }}>λ</div>
      </div>
    </div>
  );
}
const BACKS = { A: BackA, B: BackB, C: BackC };

// ── The playing card ───────────────────────────────────────────
// props: rank, suit, skin('A'|'B'|'C'), w, h, faceDown, sel(raised+ring),
//        dim, glow(highlight specials), flat(no shadow), style
function Card({ rank, suit, skin = 'A', w = 60, h = 84, faceDown, sel, dim,
                glow, flat, onClick, style = {} }) {
  const sk = SKINS[skin];
  const r = sk.radius;
  const sp = SPECIAL[rank];
  const spColor = sp && sk.special[rank];
  const col = isRed(suit) ? sk.red : sk.ink;
  const Back = BACKS[skin];

  const baseShadow = flat ? 'none'
    : sel ? '0 14px 26px rgba(0,0,0,.45)'
    : '0 3px 7px rgba(0,0,0,.28)';

  const wrap = {
    position: 'relative', width: w, height: h, borderRadius: r,
    flexShrink: 0, cursor: onClick ? 'pointer' : 'default',
    transform: sel ? 'translateY(-12px)' : 'none',
    transition: 'transform .15s ease', boxShadow: baseShadow,
    opacity: dim ? 0.5 : 1, ...style,
  };

  if (faceDown) {
    return <div style={wrap} onClick={onClick}><Back r={r} /></div>;
  }

  const big = Math.round(h * 0.30);
  const suitSz = Math.round(h * 0.20);
  const cornerR = Math.round(h * 0.155);
  const cornerS = Math.round(h * 0.135);

  // skin-specific frame chrome
  let frame = null, accentRing = null;
  if (skin === 'B') {
    frame = <div style={{ position: 'absolute', inset: 3, borderRadius: r - 1,
      border: `1px solid ${col}`, boxShadow: `inset 0 0 0 2px ${sk.paper}, inset 0 0 0 3px ${col}33` }} />;
  }
  if (skin === 'C') {
    frame = <React.Fragment>
      <div style={{ position: 'absolute', top: 0, left: 0, right: 0, height: 5, background: spColor || sk.ink, borderRadius: `${r}px ${r}px 0 0` }} />
      <div style={{ position: 'absolute', inset: 3, border: '1px solid rgba(0,0,0,.18)' }} />
    </React.Fragment>;
  }
  if (glow && spColor) {
    accentRing = { boxShadow: `${baseShadow}, 0 0 0 2px ${spColor}, 0 0 16px ${spColor}88` };
  }

  return (
    <div style={{ ...wrap, ...accentRing }} onClick={onClick}>
      <div style={{ position: 'absolute', inset: 0, borderRadius: r, background: sk.paper,
        boxShadow: skin === 'A' ? 'inset 0 0 0 1px rgba(0,0,0,.10)' : 'inset 0 0 0 1px rgba(0,0,0,.14)' }} />
      {frame}
      {/* top-left corner */}
      <div style={{ position: 'absolute', top: skin==='C'?9:6, left: 7, lineHeight: 0.92, color: col,
        fontFamily: sk.rankFont, fontWeight: sk.rankWeight, textAlign: 'center' }}>
        <div style={{ fontSize: cornerR }}>{rank}</div>
        <div style={{ fontSize: cornerS }}>{SUIT[suit]}</div>
      </div>
      {/* center suit / rank */}
      <div style={{ position: 'absolute', inset: 0, display: 'grid', placeItems: 'center' }}>
        <div style={{ fontSize: big, color: col, opacity: skin==='B'?0.9:1,
          fontFamily: sk.rankFont, fontWeight: sk.rankWeight,
          textShadow: skin==='A' && spColor ? `0 0 10px ${spColor}55` : 'none' }}>{SUIT[suit]}</div>
      </div>
      {/* special badge */}
      {sp && (
        <div style={{ position: 'absolute', bottom: skin==='C'?5:6, left: 0, right: 0, display: 'flex', justifyContent: 'center' }}>
          <span style={badgeStyle(skin, spColor)}>{sp.tag}</span>
        </div>
      )}
      {/* bottom-right corner (rotated) */}
      <div style={{ position: 'absolute', bottom: skin==='C'?7:6, right: 7, lineHeight: 0.92, color: col, transform: 'rotate(180deg)',
        fontFamily: sk.rankFont, fontWeight: sk.rankWeight, textAlign: 'center', opacity: sp ? 0 : 1 }}>
        <div style={{ fontSize: cornerR }}>{rank}</div>
        <div style={{ fontSize: cornerS }}>{SUIT[suit]}</div>
      </div>
    </div>
  );
}

function badgeStyle(skin, color) {
  if (skin === 'A') return {
    fontFamily: '"Rubik",sans-serif', fontWeight: 800, fontSize: 8, letterSpacing: .5,
    color: '#fff', background: color, borderRadius: 4, padding: '2px 5px', lineHeight: 1 };
  if (skin === 'B') return {
    fontFamily: '"Cinzel",serif', fontWeight: 600, fontSize: 7.5, letterSpacing: 1.5,
    color: color, background: 'transparent', lineHeight: 1, textTransform: 'uppercase' };
  return { // C — hazard tab
    fontFamily: '"Share Tech Mono",monospace', fontSize: 7.5, letterSpacing: .5,
    color: '#0e0e0e', background: color, padding: '1.5px 4px', lineHeight: 1, fontWeight: 700 };
}

// Small face-down fan used for opponent hands (count badge optional)
function MiniFan({ skin, n = 3, w = 30, h = 42, spread = 7 }) {
  const Back = BACKS[skin];
  return (
    <div style={{ position: 'relative', width: w + (n - 1) * spread, height: h }}>
      {Array.from({ length: n }).map((_, i) => (
        <div key={i} style={{ position: 'absolute', left: i * spread, top: 0, width: w, height: h, borderRadius: SKINS[skin].radius,
          boxShadow: '0 2px 5px rgba(0,0,0,.35)' }}><Back r={SKINS[skin].radius} /></div>
      ))}
    </div>
  );
}

Object.assign(window, { Card, MiniFan, SKINS, SUIT, SPECIAL, isRed, BACKS });
