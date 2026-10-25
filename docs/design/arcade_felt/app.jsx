// app.jsx — Arcade Felt direction, committed. Organised as a screen flow
// plus a dedicated section for the Ante Ladder hook + difficulty design.

function App() {
  const AB = ({ id, label, comp, w = 390, h = 844 }) => (
    <DCArtboard id={id} label={label} width={w} height={h}>{comp}</DCArtboard>
  );
  return (
    <DesignCanvas>
      <DCSection id="intro" title="Shed · Arcade Felt" subtitle="Committed direction — polished, mobile-first">
        <DCArtboard id="brief" label="Read me first" width={620} height={844} style={{ background: '#0a3122' }}>
          <Brief />
        </DCArtboard>
      </DCSection>

      <DCSection id="core" title="Core Screens" subtitle="The Balatro-flavoured game, screen by screen">
        {AB({ id: 'title', label: 'Title / Main Menu', comp: <TitleA /> })}
        {AB({ id: 'table', label: 'Play Table', comp: <TableA /> })}
        {AB({ id: 'draft', label: 'Buff Draft', comp: <DraftA /> })}
        {AB({ id: 'roundend', label: 'Round-End / Scoring', comp: <RoundEndA /> })}
        {AB({ id: 'cards', label: 'Card Kit', comp: <CardsA /> })}
      </DCSection>

      <DCSection id="hook" title="The Hook · Ante Ladder" subtitle="Climb antes; each one tightens the screws. Beat the top to unlock the next stake.">
        {AB({ id: 'ladder', label: 'Ante Ladder', comp: <HookA /> })}
        {AB({ id: 'difficulty', label: 'Ante Difficulty · design', comp: <AnteSpecA />, h: 1080 })}
        {AB({ id: 'boss', label: 'Boss Ante (in play)', comp: <BossTableA /> })}
      </DCSection>
    </DesignCanvas>
  );
}

function Brief() {
  const gold = '#ffd23e', cyan = '#27c4d8', magenta = '#ff3e9a', lime = '#7CFF6B';
  const pixel = '"Silkscreen", monospace';
  const li = (c, t) => (
    <div style={{ display: 'flex', gap: 10, marginBottom: 9, alignItems: 'flex-start' }}>
      <div style={{ width: 7, height: 7, borderRadius: 4, background: c, marginTop: 6, flexShrink: 0, boxShadow: `0 0 7px ${c}` }} />
      <div style={{ fontSize: 13, opacity: .82, lineHeight: 1.45 }}>{t}</div>
    </div>
  );
  return (
    <div style={{ width: 620, height: 844, padding: '46px 48px', color: '#fff', fontFamily: '"Rubik", system-ui, sans-serif', position: 'relative',
      background: 'radial-gradient(130% 100% at 50% 0%,#1c6b4f,#0a3122 70%)' }}>
      <div style={{ position: 'absolute', inset: 0, boxShadow: 'inset 0 0 0 3px rgba(255,210,62,.18), inset 0 0 120px rgba(0,0,0,.5)', pointerEvents: 'none' }} />
      <div style={{ fontFamily: pixel, fontSize: 11, letterSpacing: 3, color: cyan }}>DIRECTION LOCKED</div>
      <div style={{ fontSize: 40, fontWeight: 900, marginTop: 12, letterSpacing: -0.5 }}>Arcade <span style={{ color: magenta, textShadow: `0 0 18px ${magenta}` }}>Felt</span></div>
      <div style={{ fontSize: 14, opacity: .78, marginTop: 10, lineHeight: 1.55, maxWidth: 500 }}>
        Balatro-flavoured Shed: neon-on-felt, chunky type, juicy score pops, and special cards that wear a glowing ring + word badge so their rule reads instantly. This board is the polished spec — mobile-portrait first.
      </div>

      <div style={{ height: 1, background: 'rgba(255,255,255,.12)', margin: '26px 0 22px' }} />

      <div style={{ fontFamily: pixel, fontSize: 11, color: gold, marginBottom: 14 }}>THE HOOK — ANTE LADDER</div>
      {li(gold, <span>An <b>ante</b> is one full match (first to its point target). Win to climb; lose and the run ends — but unlocks stay.</span>)}
      {li(magenta, <span>Each ante turns up to <b>five dials</b>: target, AI skill, rivals-also-draft, runway, and a <b>boss hazard</b> that twists one rule.</span>)}
      {li(cyan, <span>Reward scales with risk — higher antes hand out rarer perks, so your build powers up as the table gets meaner.</span>)}
      {li(lime, <span>Beat the top ante to unlock the next <b>stake colour</b> (White → Gold), a permanent rule that raises the floor for every future run.</span>)}

      <div style={{ height: 1, background: 'rgba(255,255,255,.12)', margin: '22px 0' }} />

      <div style={{ display: 'flex', gap: 30 }}>
        <div style={{ flex: 1 }}>
          <div style={{ fontFamily: pixel, fontSize: 10, color: cyan, marginBottom: 8 }}>ON THIS BOARD</div>
          <div style={{ fontSize: 12.5, opacity: .74, lineHeight: 1.5 }}>
            <b>Core Screens</b> — the full game flow.<br />
            <b>The Hook</b> — ante ladder, the difficulty design, and a boss ante mid-play.
          </div>
        </div>
        <div style={{ flex: 1 }}>
          <div style={{ fontFamily: pixel, fontSize: 10, color: cyan, marginBottom: 8 }}>NEXT</div>
          <div style={{ fontSize: 12.5, opacity: .74, lineHeight: 1.5 }}>
            Packaged as a Claude Code handoff so you can build it straight into the Bevy app.
          </div>
        </div>
      </div>

      <div style={{ position: 'absolute', bottom: 30, left: 48, fontSize: 11, opacity: .45, fontFamily: pixel }}>click any frame to focus · ← / → to flip</div>
    </div>
  );
}

ReactDOM.createRoot(document.getElementById('root')).render(<App />);
