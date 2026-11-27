// FastEmu landing page. Starts each card's animation when it scrolls into
// view and pauses it when it leaves. The CSS loops do the moving; this file
// only toggles a class and types the terminal line.
(() => {
  const reduce = matchMedia('(prefers-reduced-motion: reduce)').matches;
  const cards = document.querySelectorAll('.card');

  // Terminal card: types the command, then shows the output lines.
  const term = (() => {
    const cmd = document.getElementById('cmd');
    const body = cmd && cmd.closest('.tbody');
    if (!cmd || !body) return null;
    const full = cmd.textContent;
    const lines = [...body.querySelectorAll('.ol')];
    let timers = [];
    const later = (fn, ms) => timers.push(setTimeout(fn, ms));
    const clear = () => { timers.forEach(clearTimeout); timers = []; };
    const finish = () => {
      cmd.textContent = full;
      lines.forEach((l) => l.classList.add('show'));
    };
    const cycle = () => {
      clear();
      body.classList.add('arm');
      cmd.textContent = '';
      lines.forEach((l) => l.classList.remove('show'));
      let t = 500;
      for (let i = 1; i <= full.length; i++) {
        later(() => { cmd.textContent = full.slice(0, i); }, t);
        t += 34;
      }
      t += 400;
      lines.forEach((l) => { later(() => l.classList.add('show'), t); t += 380; });
      later(cycle, t + 3200);
    };
    return {
      start: cycle,
      stop() { clear(); finish(); },
    };
  })();

  // The clip of the desktop window plays while it is on screen, unless the
  // reader paused it or asks for less motion; its poster is a still of the
  // same window.
  const clip = document.querySelector('.showcase video');
  if (clip) {
    const button = document.querySelector('.clip-toggle');
    let wanted = !reduce;
    let visible = !('IntersectionObserver' in window);
    const sync = () => {
      if (wanted && visible) clip.play().catch(() => {}); else clip.pause();
      if (button) {
        button.textContent = wanted ? 'Pause' : 'Play';
        button.setAttribute('aria-pressed', String(!wanted));
      }
    };
    if (button) {
      button.hidden = false;
      button.addEventListener('click', () => { wanted = !wanted; sync(); });
    }
    if (!visible) {
      new IntersectionObserver(([e]) => { visible = e.isIntersecting; sync(); }, { threshold: 0.25 }).observe(clip);
    }
    sync();
  }

  if (reduce || !('IntersectionObserver' in window)) {
    cards.forEach((c) => c.classList.add('on'));
    return;
  }

  // Game tiles: the recording plays while its card is on screen, and the
  // still frame comes back when it leaves, so only visible tiles animate.
  const play = (card, on) => {
    card.querySelectorAll('img[data-play]').forEach((img) => {
      if (!img.dataset.still) img.dataset.still = img.getAttribute('src');
      img.src = on ? img.dataset.play : img.dataset.still;
    });
  };

  const io = new IntersectionObserver((entries) => {
    entries.forEach((e) => {
      e.target.classList.toggle('on', e.isIntersecting);
      play(e.target, e.isIntersecting);
      if (term && e.target.contains(document.getElementById('cmd'))) {
        if (e.isIntersecting) term.start(); else term.stop();
      }
    });
  }, { threshold: 0.4 });
  cards.forEach((c) => io.observe(c));
})();
