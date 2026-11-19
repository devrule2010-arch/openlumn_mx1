const q = s => document.querySelector(s), qa = s => [...document.querySelectorAll(s)];
let mons = [];
function render() {
  const s = OL.settings;
  qa('[data-k]').forEach(el => {
    if (el === document.activeElement && el.type === 'range') return;
    const v = s[el.dataset.k]; el.type === 'checkbox' ? el.checked = v : el.value = v;
  });
  qa('#modes button').forEach(b => b.classList.toggle('on', b.dataset.m === s.mode));
  const [a, b] = OL.colors();
  q('#grad').style.background = `linear-gradient(90deg, ${a}, ${a} 40%, ${b} 80%)`;
  q('#displays').innerHTML = '';
  mons.forEach(m => {
    const l = document.createElement('label'); l.className = 'row';
    l.innerHTML = `<span>${m.name} <span class="dim">${m.width}×${m.height}</span></span><input class="sw" type="checkbox">`;
    const c = l.querySelector('input'); c.checked = !s.disabledDisplays.includes(m.index);
    c.onchange = () => {
      s.disabledDisplays = s.disabledDisplays.filter(i => i !== m.index);
      if (!c.checked) s.disabledDisplays.push(m.index);
      OL.save();
    };
    q('#displays').append(l);
  });
  const t = OL.track; q('#np').textContent = t?.title ? `♪ ${t.title}`.slice(0, 32) : '';
}
OL.init().then(async () => {
  mons = await OL.invoke('get_monitors');
  qa('[data-k]').forEach(el => el.addEventListener('input', () => {
    OL.settings[el.dataset.k] = el.type === 'checkbox' ? el.checked : el.type === 'range' ? +el.value : el.value;
    if (el.type === 'color') OL.settings.overrideAlbum = true;   // picking a color locks it in
    OL.save(); render();
  }));
  qa('#modes button').forEach(b => b.onclick = () => { OL.settings.mode = b.dataset.m; OL.save(); render(); });
  q('#quit').onclick = () => OL.invoke('quit');
  OL.onSettings = OL.onTrack = render;
  render();
});
