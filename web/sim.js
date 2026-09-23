(() => {
  const GRID = 96;
  const MAX_TIPS = 90;
  const MAX_SEGS = 14000;
  const MAX_SPORES = 28;
  const MAX_MUSH = 8;

  const canvas = document.getElementById("soil");
  const hint = document.getElementById("hint");
  const statTips = document.getElementById("stat-tips");
  const statMush = document.getElementById("stat-mush");
  const statMood = document.getElementById("stat-mood");
  const btnFeed = document.getElementById("btn-feed");
  const btnPause = document.getElementById("btn-pause");
  const btnReset = document.getElementById("btn-reset");
  const ctx = canvas.getContext("2d", { alpha: false });

  const nutrient = new Float32Array(GRID * GRID);
  const nutrientBack = new Float32Array(GRID * GRID);
  const tips = [];
  const segs = [];
  const spores = [];
  const mushrooms = [];
  const crumbs = [];

  let width = 0;
  let height = 0;
  let dpr = 1;
  let cell = 8;
  let paused = false;
  let frame = 0;
  let germinated = false;
  let lastMood = "";

  function idx(x, y) {
    return y * GRID + x;
  }

  function clamp(v, a, b) {
    return Math.max(a, Math.min(b, v));
  }

  function rand(a, b) {
    return a + Math.random() * (b - a);
  }

  function resize() {
    dpr = Math.min(window.devicePixelRatio || 1, 2);
    width = canvas.clientWidth;
    height = canvas.clientHeight;
    canvas.width = Math.max(1, Math.floor(width * dpr));
    canvas.height = Math.max(1, Math.floor(height * dpr));
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    cell = Math.min(width, height) / GRID;
  }

  function worldToGrid(px, py) {
    const ox = (width - GRID * cell) / 2;
    const oy = (height - GRID * cell) / 2;
    return {
      x: clamp((px - ox) / cell, 0.5, GRID - 1.5),
      y: clamp((py - oy) / cell, 0.5, GRID - 1.5),
    };
  }

  function gridToWorld(x, y) {
    const ox = (width - GRID * cell) / 2;
    const oy = (height - GRID * cell) / 2;
    return { x: ox + x * cell, y: oy + y * cell };
  }

  function addPatch(cx, cy, radius, amount) {
    const r = Math.ceil(radius);
    for (let dy = -r; dy <= r; dy++) {
      for (let dx = -r; dx <= r; dx++) {
        const x = Math.floor(cx + dx);
        const y = Math.floor(cy + dy);
        if (x < 0 || y < 0 || x >= GRID || y >= GRID) continue;
        const d = Math.hypot(dx, dy);
        if (d > radius) continue;
        const i = idx(x, y);
        nutrient[i] = Math.min(1, nutrient[i] + amount * (1 - d / radius));
      }
    }
  }

  function spawnTip(x, y, angle, energy) {
    if (tips.length >= MAX_TIPS) return;
    tips.push({
      x,
      y,
      px: x,
      py: y,
      angle,
      energy,
      age: 0,
      alive: true,
    });
  }

  function dropCrumb(px, py, fromUi) {
    const g = worldToGrid(px, py);
    addPatch(g.x, g.y, 5.5, 0.85);
    crumbs.push({ x: g.x, y: g.y, life: 1 });
    if (spores.length < MAX_SPORES) {
      spores.push({
        x: g.x + rand(-1.2, 1.2),
        y: g.y + rand(-1.2, 1.2),
        vx: rand(-0.04, 0.04),
        vy: rand(-0.08, -0.02),
        age: 0,
        life: rand(0.7, 1.4),
      });
    }
    if (fromUi) {
      hint.textContent = "yum. it smelled that.";
    }
  }

  function seedWorld() {
    nutrient.fill(0);
    tips.length = 0;
    segs.length = 0;
    spores.length = 0;
    mushrooms.length = 0;
    crumbs.length = 0;
    germinated = false;
    frame = 0;
    paused = false;
    btnPause.textContent = "pause";
    hint.textContent = "A spore is looking for a cozy spot…";

    for (let i = 0; i < 7; i++) {
      addPatch(rand(16, GRID - 16), rand(16, GRID - 16), rand(9, 16), rand(0.55, 0.95));
    }
    addPatch(GRID / 2, GRID / 2, 16, 0.9);

    spores.push({
      x: GRID / 2,
      y: GRID * 0.18,
      vx: 0.01,
      vy: 0.12,
      age: 0,
      life: 2.2,
      hero: true,
    });
  }

  function gradient(x, y) {
    const xi = Math.floor(x);
    const yi = Math.floor(y);
    if (xi < 1 || yi < 1 || xi >= GRID - 1 || yi >= GRID - 1) return { x: 0, y: 0 };
    const n = (ox, oy) => nutrient[idx(xi + ox, yi + oy)];
    return {
      x: n(1, 0) - n(-1, 0) + 0.5 * (n(1, 1) - n(-1, 1) + n(1, -1) - n(-1, -1)),
      y: n(0, 1) - n(0, -1) + 0.5 * (n(1, 1) - n(1, -1) + n(-1, 1) - n(-1, -1)),
    };
  }

  function diffuse() {
    nutrientBack.set(nutrient);
    for (let y = 1; y < GRID - 1; y++) {
      for (let x = 1; x < GRID - 1; x++) {
        const i = idx(x, y);
        const avg =
          (nutrientBack[idx(x - 1, y)] +
            nutrientBack[idx(x + 1, y)] +
            nutrientBack[idx(x, y - 1)] +
            nutrientBack[idx(x, y + 1)]) *
          0.25;
        nutrient[i] = nutrientBack[i] * 0.97 + avg * 0.03;
        if (nutrient[i] < 0.008) nutrient[i] *= 0.997;
      }
    }
  }

  function stepTips() {
    const born = [];
    for (const h of tips) {
      if (!h.alive) continue;
      h.px = h.x;
      h.py = h.y;
      h.age += 0.016;

      const g = gradient(h.x, h.y);
      const glen = Math.hypot(g.x, g.y);
      if (glen > 0.002) {
        const desired = Math.atan2(g.y, g.x);
        let da = desired - h.angle;
        while (da > Math.PI) da -= Math.PI * 2;
        while (da < -Math.PI) da += Math.PI * 2;
        h.angle += da * 0.18;
      }
      h.angle += rand(-0.08, 0.08);

      const xi = clamp(Math.floor(h.x), 0, GRID - 1);
      const yi = clamp(Math.floor(h.y), 0, GRID - 1);
      const food = nutrient[idx(xi, yi)];
      const speed = 0.16 + food * 0.18 + h.energy * 0.05;
      h.x += Math.cos(h.angle) * speed;
      h.y += Math.sin(h.angle) * speed;

      if (h.x < 1.2 || h.x > GRID - 1.2) {
        h.angle = Math.PI - h.angle + rand(-0.2, 0.2);
        h.x = clamp(h.x, 1.2, GRID - 1.2);
      }
      if (h.y < 1.2 || h.y > GRID - 1.2) {
        h.angle = -h.angle + rand(-0.2, 0.2);
        h.y = clamp(h.y, 1.2, GRID - 1.2);
      }

      const eat = Math.min(food, 0.028);
      nutrient[idx(xi, yi)] = Math.max(0, food - eat);
      h.energy = clamp(h.energy * 0.9988 + eat * 1.8, 0, 1);
      if (h.energy < 0.03) h.alive = false;

      if (segs.length >= MAX_SEGS) segs.shift();
      segs.push({ x0: h.px, y0: h.py, x1: h.x, y1: h.y, age: 0, energy: h.energy });

      if (
        tips.length + born.length < MAX_TIPS &&
        h.age > 0.8 &&
        h.energy > 0.32 &&
        Math.random() < 0.006 + food * 0.01
      ) {
        born.push({
          x: h.x,
          y: h.y,
          angle: h.angle + rand(-1.15, 1.15),
          energy: h.energy * 0.62,
        });
        h.energy *= 0.62;
      }

      if (
        mushrooms.length < MAX_MUSH &&
        tips.length > 24 &&
        h.energy > 0.8 &&
        food > 0.35 &&
        Math.random() < 0.00045
      ) {
        mushrooms.push({ x: h.x, y: h.y, age: 0, wobble: rand(0, Math.PI * 2) });
      }
    }

    for (const b of born) spawnTip(b.x, b.y, b.angle, b.energy);
    for (let i = tips.length - 1; i >= 0; i--) {
      if (!tips[i].alive) tips.splice(i, 1);
    }
    for (const s of segs) s.age += 0.01;
    while (segs.length && segs[0].age > 2.4) segs.shift();
  }

  function stepSpores() {
    for (const s of spores) {
      s.age += 0.016;
      s.x += s.vx;
      s.y += s.vy;
      s.vy += 0.0022;
      s.vx *= 0.99;
      if (s.hero && !germinated && s.y >= GRID / 2) {
        germinated = true;
        spawnTip(s.x, s.y, rand(0, Math.PI * 2), 0.85);
        spawnTip(s.x, s.y, rand(0, Math.PI * 2), 0.85);
        spawnTip(s.x, s.y, rand(0, Math.PI * 2), 0.8);
        spawnTip(s.x, s.y, rand(0, Math.PI * 2), 0.8);
        addPatch(s.x, s.y, 14, 0.95);
        hint.textContent = "It found a home. Tap to feed it crumbs.";
        s.age = 99;
      } else if (!s.hero && s.age > s.life * 0.6 && nutrient[idx(clamp(Math.floor(s.x), 0, GRID - 1), clamp(Math.floor(s.y), 0, GRID - 1))] > 0.35 && tips.length < MAX_TIPS) {
        spawnTip(s.x, s.y, rand(0, Math.PI * 2), 0.45);
        s.age = 99;
      }
    }
    for (let i = spores.length - 1; i >= 0; i--) {
      if (spores[i].age > spores[i].life) spores.splice(i, 1);
    }
    for (const m of mushrooms) m.age += 0.01;
    for (const c of crumbs) c.life -= 0.02;
    for (let i = crumbs.length - 1; i >= 0; i--) {
      if (crumbs[i].life <= 0) crumbs.splice(i, 1);
    }
  }

  function drawSoil() {
    ctx.fillStyle = "#15221c";
    ctx.fillRect(0, 0, width, height);

    const ox = (width - GRID * cell) / 2;
    const oy = (height - GRID * cell) / 2;

    const img = ctx.createImageData(GRID, GRID);
    const data = img.data;
    for (let i = 0; i < GRID * GRID; i++) {
      const n = nutrient[i];
      const p = i * 4;
      data[p] = 36 + n * 90;
      data[p + 1] = 52 + n * 150;
      data[p + 2] = 38 + n * 55;
      data[p + 3] = 255;
    }
    const tmp = drawSoil._tmp || (drawSoil._tmp = document.createElement("canvas"));
    tmp.width = GRID;
    tmp.height = GRID;
    tmp.getContext("2d").putImageData(img, 0, 0);
    ctx.imageSmoothingEnabled = true;
    ctx.drawImage(tmp, ox, oy, GRID * cell, GRID * cell);

    for (const c of crumbs) {
      const w = gridToWorld(c.x, c.y);
      const g = ctx.createRadialGradient(w.x, w.y, 2, w.x, w.y, 28 * c.life);
      g.addColorStop(0, `rgba(232, 196, 112, ${0.55 * c.life})`);
      g.addColorStop(1, "rgba(232, 196, 112, 0)");
      ctx.fillStyle = g;
      ctx.beginPath();
      ctx.arc(w.x, w.y, 28 * c.life, 0, Math.PI * 2);
      ctx.fill();
    }
  }

  function drawMycelium() {
    ctx.lineCap = "round";
    ctx.lineJoin = "round";
    for (let i = 0; i < segs.length; i++) {
      const s = segs[i];
      const fade = clamp(1 - s.age * 0.32, 0, 1);
      const w = gridToWorld(s.x0, s.y0);
      const w2 = gridToWorld(s.x1, s.y1);
      ctx.strokeStyle = `rgba(255, 244, 214, ${0.2 + fade * 0.58})`;
      ctx.lineWidth = 1.4 + fade * 1.6 + s.energy * 0.8;
      ctx.beginPath();
      ctx.moveTo(w.x, w.y);
      ctx.lineTo(w2.x, w2.y);
      ctx.stroke();
    }

    for (const h of tips) {
      const w = gridToWorld(h.x, h.y);
      ctx.fillStyle = `rgba(255, 252, 240, ${0.65 + h.energy * 0.35})`;
      ctx.beginPath();
      ctx.arc(w.x, w.y, 2.4 + h.energy * 1.8, 0, Math.PI * 2);
      ctx.fill();
    }

    for (const s of spores) {
      const w = gridToWorld(s.x, s.y);
      ctx.fillStyle = s.hero ? "#f4e4c8" : "rgba(243, 234, 215, 0.7)";
      ctx.beginPath();
      ctx.arc(w.x, w.y, s.hero ? 4.5 : 1.8, 0, Math.PI * 2);
      ctx.fill();
    }

    for (const m of mushrooms) {
      const w = gridToWorld(m.x, m.y);
      const grow = clamp(m.age * 2.2, 0, 1);
      const cap = 7 * grow;
      ctx.fillStyle = "rgba(13, 22, 18, 0.28)";
      ctx.beginPath();
      ctx.ellipse(w.x, w.y + 7 * grow, 6 * grow, 1.6 * grow, 0, 0, Math.PI * 2);
      ctx.fill();
      ctx.fillStyle = "#f4e4c8";
      ctx.fillRect(w.x - 1.4 * grow, w.y - 2 * grow, 2.8 * grow, 8 * grow);
      ctx.fillStyle = "#e07a5f";
      ctx.beginPath();
      ctx.ellipse(w.x, w.y - 3 * grow, cap, cap * 0.62, 0, Math.PI, Math.PI * 2);
      ctx.fill();
      ctx.fillStyle = "rgba(244, 228, 200, 0.45)";
      ctx.beginPath();
      ctx.arc(w.x - cap * 0.35, w.y - 4.2 * grow, 1.4 * grow, 0, Math.PI * 2);
      ctx.fill();
    }
  }

  function moodText() {
    if (!germinated) return "waking up";
    if (paused) return "napping";
    if (tips.length > 70) return "busy little city";
    if (tips.length > 22) return "stretching";
    return "sniffing around";
  }

  function updateHud() {
    const alive = tips.length;
    statTips.textContent = `tips ${alive}`;
    statMush.textContent = `sprouts ${mushrooms.length}`;
    const mood = moodText();
    if (mood !== lastMood) {
      statMood.textContent = mood;
      lastMood = mood;
    }
  }

  function tick() {
    frame += 1;
    if (!paused) {
      if (frame % 2 === 0) diffuse();
      stepSpores();
      if (germinated) stepTips();
    }
    drawSoil();
    drawMycelium();
    if (frame % 8 === 0) updateHud();
    requestAnimationFrame(tick);
  }

  function canvasPoint(event) {
    const rect = canvas.getBoundingClientRect();
    const t = event.touches ? event.touches[0] : event;
    return { x: t.clientX - rect.left, y: t.clientY - rect.top };
  }

  canvas.addEventListener("pointerdown", (event) => {
    event.preventDefault();
    const p = canvasPoint(event);
    dropCrumb(p.x, p.y, true);
  });

  btnFeed.addEventListener("click", () => {
    dropCrumb(width * 0.5 + rand(-80, 80), height * 0.5 + rand(-80, 80), true);
  });

  btnPause.addEventListener("click", () => {
    paused = !paused;
    btnPause.textContent = paused ? "grow" : "pause";
    hint.textContent = paused ? "Shhh. It is napping." : "Tap the soil to leave a crumb.";
  });

  btnReset.addEventListener("click", () => {
    seedWorld();
  });

  window.addEventListener("resize", resize);
  resize();
  seedWorld();
  updateHud();
  requestAnimationFrame(tick);
})();
