// VoxLocal landing: the page's own motion layer (reveals, hero story, counters, bars, mode typing, card light).
// Everything rests on its finished state without script and under prefers-reduced-motion.

const reduced = matchMedia('(prefers-reduced-motion: reduce)').matches;
const root = document.documentElement;

// ---------------------------------------------------------------- reveal on scroll
function reveals() {
  // what arrives: each block of a section, cards one after the other (never the hero or the scene)
  const sel = 'main > section:not(.hero):not(.sc) :is(.section-head, .grid > *, .ui.app, .hx-apps, .faq__item, .vs__row)';
  document.querySelectorAll(sel).forEach((el) => { if (!el.closest('[data-rv]')) el.setAttribute('data-rv', ''); });
  document.querySelectorAll('main > section:not(.hero) .grid, main .faq').forEach((g) => g.setAttribute('data-rv-group', ''));
  const els = [...document.querySelectorAll('[data-rv]')];
  if (reduced || !('IntersectionObserver' in window)) return;
  root.classList.add('rv-on');
  // children of a [data-rv-group] arrive one after the other
  document.querySelectorAll('[data-rv-group]').forEach((g) => {
    [...g.querySelectorAll('[data-rv]')].forEach((el, i) => el.style.setProperty('--rv-d', `${i * 90}ms`));
  });
  const io = new IntersectionObserver((es) => {
    for (const e of es) {
      if (!e.isIntersecting) continue;
      e.target.classList.add('is-in');
      io.unobserve(e.target);
    }
  }, { rootMargin: '0px 0px -12% 0px' });
  els.forEach((el) => io.observe(el));
}

// ---------------------------------------------------------------- counters and bars
function counters() {
  const els = [...document.querySelectorAll('[data-count]')];
  const bars = [...document.querySelectorAll('[data-bar]')];
  const fmt = (v, dec) => v.toLocaleString('it-IT', { minimumFractionDigits: dec, maximumFractionDigits: dec });
  const run = (el) => {
    const to = parseFloat(el.dataset.count);
    const dec = (el.dataset.count.split('.')[1] || '').length;
    if (reduced) { el.textContent = fmt(to, dec); return; }
    const t0 = performance.now(), dur = 1400;
    const step = (t) => {
      const k = Math.min(1, (t - t0) / dur), e = 1 - Math.pow(1 - k, 3);
      el.textContent = fmt(to * e, dec);
      if (k < 1) requestAnimationFrame(step);
    };
    requestAnimationFrame(step);
  };
  if (!('IntersectionObserver' in window)) { els.forEach(run); bars.forEach((b) => b.classList.add('is-in')); return; }
  const io = new IntersectionObserver((es) => {
    for (const e of es) {
      if (!e.isIntersecting) continue;
      if (e.target.dataset.count !== undefined) run(e.target); else e.target.classList.add('is-in');
      io.unobserve(e.target);
    }
  }, { threshold: 0.5 });
  [...els, ...bars].forEach((el) => io.observe(el));
}

// ---------------------------------------------------------------- the hero story: say, strike, clean, paste
function heroStory() {
  const said = document.querySelector('.hx__said p');
  const chips = [...document.querySelectorAll('.hx__chip')];
  const out = document.querySelector('.hx__out');
  if (!said || !out || reduced) return;
  const html = said.innerHTML;
  const strikes = () => [...said.querySelectorAll('s')];
  const wait = (ms) => new Promise((r) => setTimeout(r, ms));
  const text = said.textContent;
  let alive = true;
  document.addEventListener('visibilitychange', () => { alive = !document.hidden; });

  async function cycle() {
    // 1. the words arrive as they are spoken
    said.innerHTML = '';
    chips.forEach((c) => c.classList.remove('is-on'));
    out.classList.remove('is-on');
    for (let i = 1; i <= text.length; i += 2) {
      said.textContent = text.slice(0, i);
      await wait(28);
    }
    said.innerHTML = html;
    strikes().forEach((s) => s.classList.remove('is-struck'));
    await wait(500);
    // 2. each fix is struck through and named
    const ss = strikes();
    for (let i = 0; i < ss.length; i++) {
      ss[i].classList.add('is-struck');
      if (chips[i]) chips[i].classList.add('is-on');
      await wait(750);
    }
    // 3. the clean text is pasted
    out.classList.add('is-on');
    await wait(4200);
  }
  (async () => {
    root.classList.add('hx-live');
    for (;;) {
      if (alive) await cycle(); else await wait(500);
    }
  })();
}

// ---------------------------------------------------------------- the mode tabs write their text in
function modeTyping() {
  if (reduced) return;
  document.querySelectorAll('#modalita [role="tab"]').forEach((tab) => {
    tab.addEventListener('click', () => {
      const id = (tab.getAttribute('aria-controls') || '').split(' ').find((x) => x.startsWith('s-'));
      const screen = id && document.getElementById(id);
      const target = screen && screen.querySelector('.mx__out');
      if (!target) return;
      const blocks = [...target.querySelectorAll('p, li')];
      const texts = blocks.map((b) => b.textContent);
      blocks.forEach((b) => { b.textContent = ''; });
      let bi = 0, ci = 0;
      const tick = () => {
        if (bi >= blocks.length) return;
        ci += 2;
        blocks[bi].textContent = texts[bi].slice(0, ci);
        if (ci >= texts[bi].length) { bi += 1; ci = 0; }
        setTimeout(tick, 18);
      };
      setTimeout(tick, 120);
    });
  });
}

// ---------------------------------------------------------------- a light that follows the pointer on panels
function cardLight() {
  if (reduced || !matchMedia('(hover: hover) and (pointer: fine)').matches) return;
  document.querySelectorAll('.panel, .frame, .app.ui').forEach((el) => {
    el.classList.add('has-light');
    el.addEventListener('pointermove', (e) => {
      const r = el.getBoundingClientRect();
      el.style.setProperty('--lx', `${e.clientX - r.left}px`);
      el.style.setProperty('--ly', `${e.clientY - r.top}px`);
    });
  });
}

// ---------------------------------------------------------------- dictations floating beside the headline
function floatingDictations() {
  const box = document.querySelector('.hx-float');
  if (!box || reduced || !matchMedia('(min-width: 1100px)').matches) return;
  const items = [
    ['Slack', 'Il deploy slitta alle 21, vi aggiorno.'],
    ['Mail', 'Ciao Sara, il preventivo va bene.'],
    ['Note', 'Spesa: latte, pane, arance.'],
    ['Word', 'Il verbale è pronto per la firma.'],
    ['WhatsApp', 'Arrivo tra dieci minuti.'],
    ['Teams', 'Riunione spostata a martedì alle 10.'],
    ['Calendario', 'Dentista giovedì alle 16.'],
    ['Notion', 'Idee per il lancio di dicembre.'],
  ];
  // slots in the empty space at the sides of the headline (left and right columns)
  const slots = [[0, 4], [0, 38], [0, 70], [1, 12], [1, 46], [1, 76]];
  const busy = new Set();
  let n = 0;
  const wait = (ms) => new Promise((r) => setTimeout(r, ms));
  async function spawn() {
    const free = slots.map((_, i) => i).filter((i) => !busy.has(i));
    if (!free.length || document.hidden) return;
    const si = free[Math.floor(Math.random() * free.length)];
    busy.add(si);
    const [side, top] = slots[si];
    const [app, text] = items[n++ % items.length];
    const el = document.createElement('div');
    el.className = 'hxf';
    el.style.top = `${top}%`;
    el.style[side ? 'right' : 'left'] = `${8 + Math.round(Math.random() * 24)}px`;
    el.innerHTML = `<div class="hxf__top"><span class="hxf__dot"></span><span class="hxf__st">Registro…</span><span class="hxf__t">0:00</span></div><div class="hxf__wave">${'<i></i>'.repeat(16)}</div><div class="hxf__text"></div><span class="hxf__to">Incollato in ${app}</span>`;
    box.appendChild(el);
    requestAnimationFrame(() => el.classList.add('is-on'));
    const t = el.querySelector('.hxf__t'), st = el.querySelector('.hxf__st'), out = el.querySelector('.hxf__text');
    for (let k = 1; k <= 3; k++) { await wait(600); t.textContent = `0:0${k}`; }
    st.textContent = 'Trascrivo…';
    el.querySelector('.hxf__wave').style.opacity = '.35';
    await wait(500);
    el.classList.add('is-done');
    st.textContent = 'Trascritto';
    for (let i = 1; i <= text.length; i++) { out.textContent = text.slice(0, i); await wait(24); }
    await wait(2200);
    el.classList.add('is-out');
    await wait(700);
    el.remove();
    busy.delete(si);
  }
  spawn();
  setTimeout(spawn, 900);
  setInterval(spawn, 1700);
}

// ---------------------------------------------------------------- meeting notes and ask: small stories that play while on screen
function whenVisible(el, cls, run) {
  if (!el || reduced || !('IntersectionObserver' in window)) return;
  let on = false, playing = false;
  new IntersectionObserver((es) => { on = es[0].isIntersecting; if (on && !playing) loop(); }, { threshold: 0.35 }).observe(el);
  root.classList.add(cls);
  async function loop() {
    playing = true;
    while (on && !document.hidden) await run();
    playing = false;
  }
}

function meetingStory() {
  const box = document.querySelector('.mt');
  if (!box) return;
  const wait = (ms) => new Promise((r) => setTimeout(r, ms));
  const prompt = box.querySelector('[data-mt="prompt"]'), yes = box.querySelector('[data-mt="yes"]');
  const lines = [...box.querySelectorAll('[data-mt="line"]')], sum = box.querySelector('[data-mt="sum"]');
  const time = box.querySelector('[data-mt="time"]');
  const rem = box.querySelector('[data-mt="rem"]');
  const hide = () => { [prompt, ...lines, sum].forEach((e) => e.classList.add('is-hid')); };
  hide();
  whenVisible(box, 'mt-live', async () => {
    hide();
    time.textContent = '00:00';
    await wait(500);
    prompt.classList.remove('is-hid');
    await wait(1400);
    yes.classList.add('is-press');
    await wait(180);
    yes.classList.remove('is-press');
    await wait(300);
    prompt.classList.add('is-hid');
    const t0 = 130;
    for (let i = 0; i < lines.length; i++) {
      await wait(900);
      const sec = t0 + i * 8;
      time.textContent = `0${Math.floor(sec / 60)}:${String(sec % 60).padStart(2, '0')}`;
      lines[i].classList.remove('is-hid');
    }
    await wait(1100);
    time.textContent = '42:10';
    rem.classList.remove('is-done');
    rem.textContent = 'Aggiungi a Promemoria';
    sum.classList.remove('is-hid');
    await wait(1600);
    rem.classList.add('is-press');
    await wait(180);
    rem.classList.remove('is-press');
    rem.classList.add('is-done');
    rem.textContent = 'Aggiunte a Promemoria: 2';
    await wait(3400);
  });
}

function askStory() {
  const box = document.querySelector('.ak');
  if (!box) return;
  const wait = (ms) => new Promise((r) => setTimeout(r, ms));
  const q = box.querySelector('[data-ak="q"]'), card = box.querySelector('[data-ak="card"]');
  const src = box.querySelector('[data-ak="src"]'), n1 = box.querySelector('[data-ak-n="1"]');
  const text = q.textContent;
  whenVisible(box, 'ak-live', async () => {
    card.classList.add('is-hid'); src.classList.add('is-hid'); n1.classList.remove('on');
    q.textContent = '';
    await wait(400);
    for (let i = 1; i <= text.length; i++) { q.textContent = text.slice(0, i); await wait(40); }
    await wait(700);
    card.classList.remove('is-hid');
    await wait(1600);
    n1.classList.add('on');
    src.classList.remove('is-hid');
    await wait(4500);
  });
}

reveals();
counters();
floatingDictations();
heroStory();
modeTyping();
cardLight();
meetingStory();
askStory();
