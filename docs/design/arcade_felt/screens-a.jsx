// screens-a.jsx — Direction A · "Arcade Felt" (Balatro-flavored)
// Neon-on-felt, chunky type, big juicy numbers.

const themeA = {
  felt: 'radial-gradient(130% 100% at 50% 0%,#1c6b4f 0%,#125239 45%,#0a3122 100%)',
  ink: '#0a2a1d',
  magenta: '#ff3e9a', cyan: '#27c4d8', gold: '#ffd23e', lime: '#7CFF6B',
  ui: '"Rubik", system-ui, sans-serif',
  pixel: '"Silkscreen", monospace',
};
const W = 390, H = 844;

function FrameA({ children, pad = true, dim }) {
  return (
    <div style={{ width: W, height: H, position: 'relative', overflow: 'hidden',
      background: themeA.felt, fontFamily: themeA.ui, color: '#fff' }}>
      {/* felt noise + rim */}
      <div style={{ position: 'absolute', inset: 0, opacity: .5, mixBlendMode: 'overlay',
        backgroundImage: "url(\"data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='80' height='80'%3E%3Cfilter id='n'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='.9' numOctaves='2'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23n)' opacity='.5'/%3E%3C/svg%3E\")" }} />
      <div style={{ position: 'absolute', inset: 0, boxShadow: 'inset 0 0 0 3px rgba(255,210,62,.18), inset 0 0 90px rgba(0,0,0,.55)', pointerEvents: 'none' }} />
      <div style={{ position: 'absolute', inset: 0, padding: pad ? 0 : 0 }}>{children}</div>
      {dim && <div style={{ position: 'absolute', inset: 0, background: 'rgba(5,18,12,.62)' }} />}
    </div>
  );
}

function ChunkyA({ children, color = themeA.magenta, sub, big, style }) {
  return (
    <div style={{ background: color, color: '#1a0f1a', fontFamily: themeA.ui, fontWeight: 800,
      borderRadius: 12, padding: big ? '15px 0' : '11px 0', textAlign: 'center',
      fontSize: big ? 22 : 16, letterSpacing: .3, lineHeight: 1,
      boxShadow: `0 5px 0 ${shade(color)}, 0 9px 16px rgba(0,0,0,.4)`,
      border: '2px solid rgba(255,255,255,.35)', ...style }}>
      {children}{sub && <div style={{ fontSize: 10, opacity: .65, marginTop: 3, fontWeight: 700 }}>{sub}</div>}
    </div>
  );
}
function shade(c) {
  const m = { '#ff3e9a': '#b51c64', '#27c4d8': '#127e8c', '#ffd23e': '#c39400', '#7CFF6B': '#3da033' };
  return m[c] || 'rgba(0,0,0,.4)';
}

function SeatA({ name, init, ring, fans = 3, active, mood }) {
  return (
    <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 4, position: 'relative' }}>
      {active && <div style={{ position: 'absolute', inset: -8, borderRadius: 16, boxShadow: `0 0 0 2px ${themeA.gold}, 0 0 18px ${themeA.gold}aa`, pointerEvents: 'none' }} />}
      <div style={{ display: 'flex', alignItems: 'center', gap: 5 }}>
        <div style={{ width: 30, height: 30, borderRadius: '50%', background: `radial-gradient(circle at 40% 35%,${ring},#1a1326)`,
          border: `2px solid ${ring}`, display: 'grid', placeItems: 'center', fontWeight: 800, fontSize: 13, color: '#fff', boxShadow: `0 0 10px ${ring}66` }}>{init}</div>
        <div style={{ textAlign: 'left' }}>
          <div style={{ fontSize: 11, fontWeight: 700 }}>{name}</div>
          {mood && <div style={{ fontSize: 9, color: themeA.gold, fontWeight: 700, fontFamily: themeA.pixel }}>{mood}</div>}
        </div>
      </div>
      <MiniFan skin="A" n={fans} w={22} h={31} spread={6} />
      <div style={{ display: 'flex', gap: 3 }}>
        <Card rank="8" suit="C" skin="A" w={20} h={28} flat />
        <Card rank="K" suit="H" skin="A" w={20} h={28} flat />
        <Card faceDown skin="A" w={20} h={28} flat />
      </div>
    </div>
  );
}

// 1 ── TITLE ───────────────────────────────────────────────────
function TitleA() {
  const neon = (c) => `0 0 1px ${c},0 0 6px ${c},0 0 16px ${c}`;
  return (
    <FrameA>
      <div style={{ position: 'absolute', inset: 0, display: 'flex', flexDirection: 'column', alignItems: 'center', padding: '88px 30px 30px' }}>
        <div style={{ fontFamily: themeA.pixel, fontWeight: 700, fontSize: 64, color: '#fff', letterSpacing: 2,
          textShadow: neon(themeA.magenta), transform: 'rotate(-2deg)' }}>SHED</div>
        <div style={{ marginTop: 10, fontFamily: themeA.ui, fontWeight: 700, fontSize: 13, letterSpacing: 6, color: themeA.cyan, textShadow: neon(themeA.cyan) }}>ROGUELIKE CARD GAME</div>

        {/* hero card fan */}
        <div style={{ position: 'relative', height: 150, marginTop: 34, width: 200 }}>
          {[['A','S',-22,18,-1],['2','H',-2,6,0.5],['10','C',18,0,2],['7','D',38,10,1.5]].map(([rk,su,x,y,rot],i)=>(
            <div key={i} style={{ position: 'absolute', left: 70 + x*1.6, top: 6 + y, transform: `rotate(${rot*7}deg)` }}>
              <Card rank={rk} suit={su} skin="A" w={64} h={90} glow />
            </div>
          ))}
        </div>

        <div style={{ flex: 1 }} />
        <div style={{ width: '100%', display: 'flex', flexDirection: 'column', gap: 11 }}>
          <ChunkyA color={themeA.gold} big sub="ante 1 · stake white">CONTINUE RUN</ChunkyA>
          <ChunkyA color={themeA.magenta}>NEW RUN</ChunkyA>
          <div style={{ display: 'flex', gap: 11 }}>
            <ChunkyA color={themeA.cyan} style={{ flex: 1, fontSize: 13 }}>DAILY</ChunkyA>
            <ChunkyA color={themeA.cyan} style={{ flex: 1, fontSize: 13 }}>COLLECTION</ChunkyA>
          </div>
        </div>
        <div style={{ marginTop: 16, fontSize: 10, opacity: .5, fontFamily: themeA.pixel }}>v0.4 · ♪ on</div>
      </div>
    </FrameA>
  );
}

// 2 ── PLAY TABLE ──────────────────────────────────────────────
function TableA({ hazard, hazardDetail }) {
  return (
    <FrameA>
      {hazard && <div style={{ position: 'absolute', inset: 0, background: 'radial-gradient(120% 90% at 50% 32%, rgba(255,50,70,.18), transparent 62%)', pointerEvents: 'none' }} />}
      {/* opponents */}
      <div style={{ position: 'absolute', top: 16, left: 0, right: 0, display: 'flex', justifyContent: 'space-around', padding: '0 8px' }}>
        <SeatA name="Rob" init="R" ring={themeA.magenta} mood="‹ on a tear" active />
        <SeatA name="Mike" init="M" ring="#9b6bff" fans={2} mood="…hoarding" />
        <SeatA name="Dave" init="D" ring={themeA.lime} fans={4} mood="chaos!" />
      </div>

      {/* scoreboard chip */}
      <div style={{ position: 'absolute', top: 14, right: 12, background: 'rgba(8,30,20,.7)', border: `1px solid ${themeA.gold}55`, borderRadius: 9, padding: '6px 9px', fontSize: 10, fontFamily: themeA.pixel, lineHeight: 1.5 }}>
        <div style={{ color: themeA.gold }}>ANTE {hazard ? '3' : '1'}</div>
        <div>YOU 0 · 1ST→{hazard ? '12' : '10'}</div>
      </div>

      {hazard && (
        <div style={{ position: 'absolute', top: 150, left: 22, right: 22 }}>
          <div style={{ background: 'repeating-linear-gradient(45deg,#1a0c10 0 11px,#3a141c 11px 22px)', border: `1.5px solid ${themeA.magenta}`, borderRadius: 11, padding: '9px 13px', boxShadow: `0 0 22px rgba(255,62,154,.45)` }}>
            <div style={{ fontFamily: themeA.pixel, fontSize: 11, color: themeA.magenta, letterSpacing: 1, textShadow: `0 0 8px ${themeA.magenta}` }}>▲ HAZARD · {hazard}</div>
            <div style={{ fontSize: 11, opacity: .9, marginTop: 4, lineHeight: 1.35 }}>{hazardDetail}</div>
          </div>
        </div>
      )}

      {/* center pile + prompt */}
      <div style={{ position: 'absolute', top: 322, left: 0, right: 0, display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 14 }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 16 }}>
          <div style={{ textAlign: 'center' }}>
            <Card faceDown skin="A" w={56} h={80} />
            <div style={{ fontSize: 9, marginTop: 5, fontFamily: themeA.pixel, opacity: .7 }}>DECK 19</div>
          </div>
          <div style={{ position: 'relative' }}>
            {/* fanned pile underlay */}
            <div style={{ position: 'absolute', left: -8, top: 6, transform: 'rotate(-9deg)' }}><Card rank="6" suit="S" skin="A" w={60} h={84} flat /></div>
            <div style={{ position: 'absolute', left: 5, top: -3, transform: 'rotate(6deg)' }}><Card rank="9" suit="D" skin="A" w={60} h={84} flat /></div>
            <Card rank="9" suit="C" skin="A" w={64} h={90} glow />
            <div style={{ position: 'absolute', top: -10, right: -14, background: themeA.magenta, color: '#fff', fontFamily: themeA.pixel, fontSize: 10, borderRadius: 10, padding: '2px 6px', boxShadow: '0 2px 6px rgba(0,0,0,.4)' }}>×7</div>
          </div>
        </div>
        <div style={{ fontFamily: themeA.pixel, fontSize: 14, color: themeA.gold, textShadow: `0 0 10px ${themeA.gold}` }}>▲ YOUR TURN · PLAY ≥ 9</div>
        {/* juice pop */}
        <div style={{ position: 'absolute', top: -6, left: 250, fontFamily: themeA.pixel, color: themeA.lime, fontSize: 22, textShadow: `0 0 12px ${themeA.lime}`, transform: 'rotate(-8deg)' }}>+3!</div>
      </div>

      {/* active buffs HUD */}
      <div style={{ position: 'absolute', bottom: 232, left: 10, display: 'flex', gap: 6, flexWrap: 'wrap', maxWidth: 240 }}>
        {[['Wild 2s', themeA.cyan], ['Hot Hand', themeA.magenta], ['Counter-7', themeA.gold]].map(([t, c]) => (
          <span key={t} style={{ background: 'rgba(8,30,20,.65)', border: `1.5px solid ${c}`, color: c, fontSize: 10, fontWeight: 800, padding: '4px 8px', borderRadius: 20 }}>{t}</span>
        ))}
      </div>

      {/* your stacks */}
      <div style={{ position: 'absolute', bottom: 120, left: 0, right: 0, display: 'flex', justifyContent: 'center', gap: 7 }}>
        {[['Q','S'],['Q','C'],['A','H']].map(([rk,su],i)=>(
          <div key={i} style={{ position: 'relative' }}>
            <div style={{ position: 'absolute', inset: 0, top: 4, left: 3 }}><Card faceDown skin="A" w={50} h={70} flat /></div>
            <Card rank={rk} suit={su} skin="A" w={50} h={70} flat />
          </div>
        ))}
      </div>

      {/* hand */}
      <div style={{ position: 'absolute', bottom: 26, left: 0, right: 92, display: 'flex', justifyContent: 'center', gap: 8, alignItems: 'flex-end' }}>
        <Card rank="K" suit="C" skin="A" w={62} h={88} />
        <Card rank="J" suit="D" skin="A" w={62} h={88} sel />
        <Card rank="10" suit="S" skin="A" w={62} h={88} glow />
        <Card rank="4" suit="H" skin="A" w={62} h={88} />
      </div>
      <div style={{ position: 'absolute', bottom: 30, right: 12 }}>
        <ChunkyA color={themeA.lime} style={{ padding: '12px 16px', fontSize: 14 }}>PLAY ▸</ChunkyA>
      </div>
    </FrameA>
  );
}

// 3 ── BUFF DRAFT ──────────────────────────────────────────────
function PerkA({ name, blurb, rarity, icon, sub }) {
  const rc = { common: '#9aa7b4', rare: themeA.cyan, epic: '#b06bff', gold: themeA.gold }[rarity];
  return (
    <div style={{ position: 'relative', borderRadius: 14, padding: 12, display: 'flex', gap: 12, alignItems: 'center',
      background: 'linear-gradient(150deg,#13241c,#0c1812)', border: `2px solid ${rc}`, boxShadow: `0 0 18px ${rc}44, 0 6px 14px rgba(0,0,0,.4)` }}>
      <div style={{ width: 50, height: 50, borderRadius: 11, flexShrink: 0, background: `radial-gradient(circle at 40% 35%,${rc},#0c140f)`,
        border: `1.5px solid ${rc}`, display: 'grid', placeItems: 'center', fontSize: 22, boxShadow: `0 0 12px ${rc}66` }}>{icon}</div>
      <div style={{ flex: 1 }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 7 }}>
          <span style={{ fontWeight: 800, fontSize: 15 }}>{name}</span>
          <span style={{ fontFamily: themeA.pixel, fontSize: 8, color: rc, border: `1px solid ${rc}`, borderRadius: 4, padding: '1px 4px' }}>{rarity.toUpperCase()}</span>
        </div>
        <div style={{ fontSize: 11.5, opacity: .82, marginTop: 3, lineHeight: 1.35 }}>{blurb}</div>
        {sub && <div style={{ fontSize: 9.5, color: themeA.gold, marginTop: 3, fontWeight: 700 }}>{sub}</div>}
      </div>
    </div>
  );
}
function DraftA() {
  return (
    <FrameA dim>
      <div style={{ position: 'absolute', inset: 0, padding: '70px 18px 26px', display: 'flex', flexDirection: 'column' }}>
        <div style={{ textAlign: 'center' }}>
          <div style={{ fontFamily: themeA.pixel, fontSize: 13, color: themeA.cyan, letterSpacing: 2 }}>ROUND 1 · ANTE 1</div>
          <div style={{ fontFamily: themeA.ui, fontWeight: 900, fontSize: 30, marginTop: 6, color: '#fff', textShadow: `0 0 16px ${themeA.magenta}` }}>PICK A PERK</div>
          <div style={{ fontSize: 11.5, opacity: .65, marginTop: 4 }}>Buffs stack for the whole run. Choose 1 of 3.</div>
        </div>
        <div style={{ display: 'flex', flexDirection: 'column', gap: 12, marginTop: 24 }}>
          <PerkA name="Wild Twos" rarity="rare" icon="2" blurb="Every 2 you play also copies the rank beneath it." />
          <PerkA name="Hot Hand" rarity="epic" icon="↑" blurb="First card each turn counts as one rank higher." sub="synergises with Counter-7" />
          <PerkA name="Peek" rarity="common" icon="◉" blurb="Once per round (P): reveal your face-down cards for 3s." sub="consumable · refreshes each round" />
        </div>
        <div style={{ flex: 1 }} />
        <div style={{ display: 'flex', gap: 6, justifyContent: 'center', marginBottom: 10 }}>
          {[0,1,2].map(i=>(<div key={i} style={{ width: 7, height: 7, borderRadius: 4, background: i===0?themeA.gold:'rgba(255,255,255,.25)' }} />))}
        </div>
        <div style={{ fontSize: 10, textAlign: 'center', opacity: .5, fontFamily: themeA.pixel }}>REROLL POOL · $3</div>
      </div>
    </FrameA>
  );
}

// 4 ── ROUND END ───────────────────────────────────────────────
function RoundEndA() {
  const rows = [
    ['1st', 'You', '+3', themeA.gold, 'Y'],
    ['2nd', 'Rob', '+2', themeA.magenta, 'R'],
    ['3rd', 'Mike', '+1', '#9b6bff', 'M'],
    ['4th', 'Dave', '+0', themeA.lime, 'D'],
  ];
  const totals = [['You', 6], ['Rob', 5], ['Mike', 3], ['Dave', 1]];
  return (
    <FrameA dim>
      <div style={{ position: 'absolute', inset: 0, padding: '64px 22px 26px', display: 'flex', flexDirection: 'column' }}>
        <div style={{ textAlign: 'center' }}>
          <div style={{ fontFamily: themeA.pixel, fontSize: 12, color: themeA.cyan, letterSpacing: 2 }}>ROUND 2 COMPLETE</div>
          <div style={{ fontFamily: themeA.ui, fontWeight: 900, fontSize: 30, marginTop: 4, color: themeA.gold, textShadow: `0 0 18px ${themeA.gold}` }}>FINISH ORDER</div>
        </div>
        <div style={{ display: 'flex', flexDirection: 'column', gap: 9, marginTop: 22 }}>
          {rows.map(([pos, name, pts, c, init], i) => (
            <div key={name} style={{ display: 'flex', alignItems: 'center', gap: 12, background: i===0?'rgba(255,210,62,.12)':'rgba(8,30,20,.6)',
              border: `1.5px solid ${i===0?themeA.gold:'rgba(255,255,255,.1)'}`, borderRadius: 12, padding: '10px 14px' }}>
              <div style={{ fontFamily: themeA.pixel, fontSize: 13, color: c, width: 28 }}>{pos}</div>
              <div style={{ width: 30, height: 30, borderRadius: '50%', background: `radial-gradient(circle at 40% 35%,${c},#1a1326)`, border: `2px solid ${c}`, display: 'grid', placeItems: 'center', fontWeight: 800 }}>{init}</div>
              <div style={{ flex: 1, fontWeight: 700, fontSize: 15, display: 'flex', alignItems: 'center', gap: 7 }}>{name}{name==='Dave' && <span style={{ fontFamily: themeA.pixel, fontSize: 7.5, color: themeA.magenta, border: `1px solid ${themeA.magenta}77`, borderRadius: 4, padding: '2px 4px', letterSpacing: .5 }}>THE SHED</span>}</div>
              <div style={{ fontFamily: themeA.pixel, fontSize: 18, color: pts==='+0'?'#6a8378':themeA.lime, textShadow: pts==='+0'?'none':`0 0 10px ${themeA.lime}` }}>{pts}</div>
            </div>
          ))}
        </div>
        <div style={{ marginTop: 18, background: 'rgba(8,30,20,.55)', borderRadius: 12, padding: '12px 14px' }}>
          <div style={{ fontSize: 10, fontFamily: themeA.pixel, color: themeA.cyan, marginBottom: 8 }}>RUN TOTAL · FIRST TO 10</div>
          {totals.map(([n, v]) => (
            <div key={n} style={{ display: 'flex', alignItems: 'center', gap: 8, margin: '6px 0' }}>
              <div style={{ width: 38, fontSize: 12, fontWeight: 700 }}>{n}</div>
              <div style={{ flex: 1, height: 8, background: 'rgba(255,255,255,.08)', borderRadius: 5, overflow: 'hidden' }}>
                <div style={{ width: `${v*10}%`, height: '100%', background: `linear-gradient(90deg,${themeA.cyan},${themeA.gold})` }} />
              </div>
              <div style={{ fontFamily: themeA.pixel, fontSize: 12, width: 18, textAlign: 'right' }}>{v}</div>
            </div>
          ))}
        </div>
        <div style={{ flex: 1 }} />
        <div style={{ fontSize: 11, textAlign: 'center', opacity: .7, marginBottom: 12 }}>Dave was the Shed — they play first next round, with a <span style={{ color: themeA.gold }}>bigger draft pool</span>.</div>
        <ChunkyA color={themeA.lime} big>CONTINUE ▸</ChunkyA>
      </div>
    </FrameA>
  );
}

// 5 ── META HOOK · Ante / Stake ladder ─────────────────────────
function aiStars(n) { return '★★★★'.slice(0, n) + '☆☆☆☆'.slice(0, 4 - n); }
function HookA() {
  const antes = [
    { n: '1', stake: 'White', st: 'cleared', c: themeA.lime, tgt: 10, ai: 1 },
    { n: '2', stake: 'White', st: 'cleared', c: themeA.lime, tgt: 10, ai: 2 },
    { n: '3', stake: 'Red',   st: 'NOW',     c: themeA.gold, tgt: 12, ai: 2, hz: 'Glass Sevens' },
    { n: '4', stake: 'Red',   st: 'locked',  c: '#3a5147',   tgt: 12, ai: 3 },
    { n: '5', stake: 'Green', st: 'locked',  c: '#3a5147',   tgt: 14, ai: 3, hz: 'Stacked Deck' },
  ];
  return (
    <FrameA>
      <div style={{ position: 'absolute', inset: 0, padding: '60px 18px 22px', display: 'flex', flexDirection: 'column' }}>
        <div style={{ textAlign: 'center' }}>
          <div style={{ fontFamily: themeA.pixel, fontSize: 12, color: themeA.cyan, letterSpacing: 2 }}>THE RUN</div>
          <div style={{ fontFamily: themeA.ui, fontWeight: 900, fontSize: 27, marginTop: 4 }}>ANTE LADDER</div>
          <div style={{ fontSize: 11, opacity: .6, marginTop: 3 }}>Win the match to climb. Each ante tightens the screws.</div>
        </div>
        <div style={{ flex: 1, display: 'flex', flexDirection: 'column', justifyContent: 'center', gap: 8, margin: '12px 2px' }}>
          {antes.map(({ n, stake, st, c, tgt, ai, hz }) => (
            <div key={n} style={{ display: 'flex', alignItems: 'center', gap: 11 }}>
              <div style={{ width: 44, height: 44, borderRadius: 12, flexShrink: 0, display: 'grid', placeItems: 'center',
                background: st==='NOW'?`radial-gradient(circle,${c},#7a5a00)`:'rgba(8,30,20,.6)', border: `2px solid ${c}`,
                fontFamily: themeA.pixel, fontSize: 18, color: st==='locked'?'#6a8378':'#fff', boxShadow: st==='NOW'?`0 0 18px ${c}`:'none' }}>{n}</div>
              <div style={{ flex: 1, background: 'rgba(8,30,20,.5)', borderRadius: 11, padding: '8px 12px', border: `1px solid ${st==='NOW'?c:'rgba(255,255,255,.08)'}` }}>
                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                  <span style={{ fontWeight: 700, fontSize: 13.5 }}>Ante {n} · <span style={{ color: c }}>{stake}</span></span>
                  <span style={{ fontFamily: themeA.pixel, fontSize: 8.5, color: c }}>{st}</span>
                </div>
                <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginTop: 3, fontSize: 10, opacity: .72 }}>
                  <span>1ST→{tgt}</span>
                  <span style={{ color: themeA.gold, letterSpacing: -1 }}>{aiStars(ai)}</span>
                  {hz && <span style={{ fontFamily: themeA.pixel, fontSize: 8, color: themeA.magenta, border: `1px solid ${themeA.magenta}66`, borderRadius: 4, padding: '1px 4px', opacity: 1 }}>▲ {hz}</span>}
                </div>
              </div>
            </div>
          ))}
        </div>
        <div style={{ background: 'rgba(8,30,20,.6)', borderRadius: 12, padding: '10px 13px', display: 'flex', alignItems: 'center', gap: 10, marginBottom: 11 }}>
          <div style={{ fontFamily: themeA.pixel, fontSize: 10, color: themeA.cyan }}>UNLOCKS</div>
          <div style={{ flex: 1, display: 'flex', gap: 5 }}>
            {['2','7','K','½','◉','↑'].map((e,i)=>(<div key={i} style={{ width: 26, height: 26, borderRadius: 7, background: 'rgba(255,255,255,.06)', border: '1px solid rgba(255,255,255,.12)', display: 'grid', placeItems: 'center', fontSize: 13, fontWeight: 800 }}>{e}</div>))}
            <div style={{ width: 26, height: 26, borderRadius: 7, border: '1px dashed rgba(255,255,255,.2)', display: 'grid', placeItems: 'center', fontSize: 11, opacity: .5 }}>+7</div>
          </div>
        </div>
        <ChunkyA color={themeA.gold} big sub="red stake · Glass Sevens · 1st→12">ENTER ANTE 3 ▸</ChunkyA>
      </div>
    </FrameA>
  );
}

// 6 ── CARD DESIGN SHEET ───────────────────────────────────────
function CardsA() {
  return (
    <FrameA>
      <div style={{ position: 'absolute', inset: 0, padding: '40px 22px', display: 'flex', flexDirection: 'column', gap: 18 }}>
        <div style={{ fontFamily: themeA.pixel, fontSize: 13, color: themeA.gold, letterSpacing: 1 }}>CARD KIT · ARCADE FELT</div>
        <Row label="FACES">
          <Card rank="A" suit="S" skin="A" w={58} h={82} />
          <Card rank="9" suit="H" skin="A" w={58} h={82} />
          <Card rank="K" suit="D" skin="A" w={58} h={82} />
        </Row>
        <Row label="SPECIALS · glow + badge">
          <Card rank="2" suit="H" skin="A" w={52} h={74} glow />
          <Card rank="3" suit="S" skin="A" w={52} h={74} glow />
          <Card rank="7" suit="D" skin="A" w={52} h={74} glow />
          <Card rank="10" suit="C" skin="A" w={52} h={74} glow />
        </Row>
        <Row label="STATES">
          <div style={{ textAlign: 'center' }}><Card rank="J" suit="C" skin="A" w={52} h={74} sel /><div style={cap}>staged</div></div>
          <div style={{ textAlign: 'center' }}><Card rank="5" suit="S" skin="A" w={52} h={74} dim /><div style={cap}>invalid</div></div>
          <div style={{ textAlign: 'center' }}><Card faceDown skin="A" w={52} h={74} /><div style={cap}>back</div></div>
        </Row>
        <div style={{ flex: 1 }} />
        <div style={{ fontSize: 11, opacity: .7, lineHeight: 1.5 }}>
          Warm paper, hot-pink/cyan/gold accents. Specials carry a coloured neon ring + word badge so rules read at a glance. Big Rubik rank, Silkscreen for HUD numerals.
        </div>
      </div>
    </FrameA>
  );
}
function Row({ label, children }) {
  return (
    <div>
      <div style={{ fontSize: 9.5, fontFamily: themeA.pixel, opacity: .55, marginBottom: 8 }}>{label}</div>
      <div style={{ display: 'flex', gap: 10, alignItems: 'flex-end' }}>{children}</div>
    </div>
  );
}
const cap = { fontSize: 9, opacity: .6, marginTop: 5, fontFamily: themeA.pixel };

// 7 ── BOSS ANTE (table variant with hazard active) ───────────
function BossTableA() {
  return <TableA hazard="GLASS SEVENS" hazardDetail="A 7 now forces the next TWO plays to be ≤ 7. Each boss ante twists one rule." />;
}

// 8 ── ANTE DIFFICULTY · design spec ───────────────────────────
function AnteSpecA() {
  const dials = [
    ['◎', 'Point target', 'Match length grows — first-to-10 → 12 → 14 → 16.', 4],
    ['☻', 'AI skill', 'Rivals bundle, hoard and burn smarter the higher you climb.', 4],
    ['♣', 'Rivals draft too', 'From Ante 2 opponents also gain buffs (+1 → +3 each).', 3],
    ['▾', 'Runway', 'High antes shrink your hand refill and cap pile pickups.', 3],
    ['▲', 'Boss hazard', 'Every 2nd ante twists one rule for that match only.', 5],
  ];
  const hazards = [
    ['Glass Sevens', 'a 7 forces the next TWO plays ≤ 7'],
    ['Stacked Deck', 'each round the pile starts on a high card'],
    ['No Mercy', 'pickup-softening buffs are disabled'],
    ['Cold Tens', 'your 10s no longer burn — only theirs do'],
    ['Cull', 'last place each round loses a random buff'],
    ['Twin Shed', 'finish bottom-two and the run ends'],
  ];
  const stakes = [
    ['White', themeA.lime, 'Base rules. Where every run starts.', true],
    ['Red', themeA.gold, 'Rivals open each round having ditched a low card.', true],
    ['Green', themeA.cyan, 'Half-Pickup & friends are weakened.', false],
    ['Black', themeA.magenta, 'A random boss hazard runs on every ante.', false],
    ['Gold', '#ff8a3c', 'All hazards harder, AI maxed. The summit.', false],
  ];
  const seg = (n) => (
    <div style={{ display: 'flex', gap: 2 }}>
      {Array.from({ length: 5 }).map((_, i) => (
        <div key={i} style={{ width: 9, height: 5, borderRadius: 1, background: i < n ? themeA.gold : 'rgba(255,255,255,.14)' }} />
      ))}
    </div>
  );
  return (
    <div style={{ width: 390, minHeight: 1080, position: 'relative', overflow: 'hidden', background: themeA.felt, fontFamily: themeA.ui, color: '#fff' }}>
      <div style={{ position: 'absolute', inset: 0, boxShadow: 'inset 0 0 0 3px rgba(255,210,62,.18), inset 0 0 90px rgba(0,0,0,.55)', pointerEvents: 'none' }} />
      <div style={{ padding: '44px 22px 40px', position: 'relative' }}>
        <div style={{ fontFamily: themeA.pixel, fontSize: 12, color: themeA.cyan, letterSpacing: 2 }}>THE RUN · BALANCE</div>
        <div style={{ fontFamily: themeA.ui, fontWeight: 900, fontSize: 28, marginTop: 5 }}>ANTE DIFFICULTY</div>
        <div style={{ fontSize: 12, opacity: .72, marginTop: 7, lineHeight: 1.5 }}>
          An <b>ante</b> is one full match (first to its point target). Win it to climb; lose and the run ends — but your unlocks stay. Power and pressure rise together, so each ante feels winnable but meaner.
        </div>

        <SpecHead>DIALS THAT TIGHTEN</SpecHead>
        <div style={{ display: 'flex', flexDirection: 'column', gap: 9 }}>
          {dials.map(([ic, name, desc, ramp]) => (
            <div key={name} style={{ display: 'flex', gap: 11, alignItems: 'center', background: 'rgba(8,30,20,.5)', borderRadius: 11, padding: '9px 12px' }}>
              <div style={{ width: 30, height: 30, borderRadius: 8, flexShrink: 0, display: 'grid', placeItems: 'center', background: 'rgba(255,210,62,.12)', border: `1px solid ${themeA.gold}55`, color: themeA.gold, fontSize: 15, fontWeight: 800 }}>{ic}</div>
              <div style={{ flex: 1 }}>
                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                  <span style={{ fontWeight: 700, fontSize: 13.5 }}>{name}</span>{seg(ramp)}
                </div>
                <div style={{ fontSize: 11, opacity: .72, marginTop: 2, lineHeight: 1.3 }}>{desc}</div>
              </div>
            </div>
          ))}
        </div>

        <SpecHead>BOSS HAZARDS <span style={{ color: themeA.magenta, fontFamily: themeA.ui, fontWeight: 700 }}>▲ one per boss ante</span></SpecHead>
        <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 8 }}>
          {hazards.map(([name, desc]) => (
            <div key={name} style={{ background: 'rgba(40,10,18,.5)', border: `1px solid ${themeA.magenta}44`, borderRadius: 10, padding: '8px 10px' }}>
              <div style={{ fontWeight: 800, fontSize: 12, color: themeA.magenta }}>{name}</div>
              <div style={{ fontSize: 10, opacity: .78, marginTop: 2, lineHeight: 1.3 }}>{desc}</div>
            </div>
          ))}
        </div>

        <SpecHead>STAKES <span style={{ color: themeA.cyan, fontFamily: themeA.ui, fontWeight: 700 }}>· meta ladder across runs</span></SpecHead>
        <div style={{ display: 'flex', flexDirection: 'column', gap: 7 }}>
          {stakes.map(([name, c, desc, unlocked]) => (
            <div key={name} style={{ display: 'flex', gap: 11, alignItems: 'center', background: 'rgba(8,30,20,.5)', borderRadius: 10, padding: '8px 12px', opacity: unlocked ? 1 : .62 }}>
              <div style={{ width: 12, height: 12, borderRadius: '50%', flexShrink: 0, background: c, boxShadow: `0 0 8px ${c}` }} />
              <div style={{ width: 50, fontWeight: 800, fontSize: 13, color: c }}>{name}</div>
              <div style={{ flex: 1, fontSize: 11, opacity: .82, lineHeight: 1.3 }}>{desc}</div>
              <div style={{ fontFamily: themeA.pixel, fontSize: 8, color: unlocked ? themeA.lime : '#6a8378' }}>{unlocked ? 'OPEN' : 'LOCK'}</div>
            </div>
          ))}
        </div>
        <div style={{ fontSize: 10.5, opacity: .55, marginTop: 16, lineHeight: 1.45, fontStyle: 'italic' }}>
          Beat the final ante to unlock the next stake colour — a permanent rule that raises the floor for every future run. That's the long-tail "one more run".
        </div>
      </div>
    </div>
  );
}
function SpecHead({ children }) {
  return <div style={{ fontFamily: themeA.pixel, fontSize: 10, color: themeA.gold, letterSpacing: 1, margin: '22px 0 11px', display: 'flex', gap: 8, alignItems: 'baseline', flexWrap: 'wrap' }}>{children}</div>;
}

Object.assign(window, { TitleA, TableA, DraftA, RoundEndA, HookA, CardsA, BossTableA, AnteSpecA });
