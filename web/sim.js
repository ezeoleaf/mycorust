(() => {
  const GRID = 96;
  const MAX_TIPS = 90;
  const MAX_TRAIL = 480;
  const MAX_GHOSTS = 48;
  const MAX_SPORES = 28;
  const MAX_MUSH = 8;
  const SEASON_FRAMES = 22 * 60;
  const WEATHER_FRAMES = 9 * 60;

  const SEASONS = ["spring", "summer", "autumn", "winter"];
  const WEATHER_BY_SEASON = {
    spring: ["clear", "rain", "sun", "warm"],
    summer: ["sun", "warm", "clear", "rain"],
    autumn: ["clear", "rain", "cold", "sun"],
    winter: ["cold", "snow", "clear", "cold"],
  };
  const WEATHER_HINTS = {
    clear: "A quiet sky. Tap the soil to leave a crumb.",
    rain: "Rain is soaking the crumbs. The hyphae love this.",
    sun: "Sunlight on the soil. Fruiting feels close.",
    warm: "Warm air. Everything is stretching faster.",
    cold: "A chill. The network is taking its time.",
    snow: "Snow dust. Growth is sleepy and pale.",
  };
  const SOIL = {
    spring: { deep: [18, 36, 28], rich: [92, 168, 88], light: [210, 232, 170] },
    summer: { deep: [28, 40, 18], rich: [140, 170, 62], light: [240, 214, 110] },
    autumn: { deep: [32, 22, 14], rich: [168, 98, 42], light: [232, 176, 92] },
    winter: { deep: [18, 28, 36], rich: [92, 128, 148], light: [210, 226, 236] },
  };

  const canvas = document.getElementById("soil");
  const hint = document.getElementById("hint");
  const statSeason = document.getElementById("stat-season");
  const statWeather = document.getElementById("stat-weather");
  const statTips = document.getElementById("stat-tips");
  const statMush = document.getElementById("stat-mush");
  const statMood = document.getElementById("stat-mood");
  const btnFeed = document.getElementById("btn-feed");
  const btnSky = document.getElementById("btn-sky");
  const btnPause = document.getElementById("btn-pause");
  const btnReset = document.getElementById("btn-reset");
  const ctx = canvas.getContext("2d", { alpha: false });

  const nutrient = new Float32Array(GRID * GRID);
  const nutrientBack = new Float32Array(GRID * GRID);
  const tips = [];
  const ghosts = [];
  const spores = [];
  const mushrooms = [];
  const crumbs = [];
  const flakes = [];
  const motes = [];

  let nextTipId = 1;
  let width = 0;
  let height = 0;
  let dpr = 1;
  let cell = 8;
  let paused = false;
  let frame = 0;
  let germinated = false;
  let lastMood = "";
  let seasonIndex = 0;
  let seasonTimer = 0;
  let weather = "clear";
  let weatherTimer = 0;
  let climateLocked = false;

  function idx(x, y) {
    return y * GRID + x;
  }

  function clamp(v, a, b) {
    return Math.max(a, Math.min(b, v));
  }

  function lerp(a, b, t) {
    return a + (b - a) * t;
  }

  function rand(a, b) {
    return a + Math.random() * (b - a);
  }

  function pick(list) {
    return list[Math.floor(Math.random() * list.length)];
  }

  function season() {
    return SEASONS[seasonIndex];
  }

  function climate() {
    const s = season();
    const w = weather;
    const c = {
      speed: 1,
      branch: 1,
      fruit: 1,
      diffuse: 1,
      eat: 1,
      energyDrain: 1,
      rainFeed: 0,
    };
    if (s === "spring") {
      c.speed = 1.05;
      c.branch = 1.15;
      c.fruit = 0.85;
    } else if (s === "summer") {
      c.speed = 1.12;
      c.branch = 1.05;
      c.fruit = 1.45;
    } else if (s === "autumn") {
      c.speed = 0.92;
      c.branch = 0.9;
      c.fruit = 1.2;
    } else {
      c.speed = 0.72;
      c.branch = 0.55;
      c.fruit = 0.35;
      c.energyDrain = 1.15;
    }
    if (w === "rain") {
      c.speed *= 1.08;
      c.branch *= 1.25;
      c.diffuse = 1.45;
      c.rainFeed = 0.0018;
    } else if (w === "sun") {
      c.speed *= 1.1;
      c.fruit *= 1.4;
      c.eat *= 1.1;
    } else if (w === "warm") {
      c.speed *= 1.18;
      c.branch *= 1.2;
      c.energyDrain *= 0.92;
    } else if (w === "cold") {
      c.speed *= 0.7;
      c.branch *= 0.65;
      c.fruit *= 0.55;
      c.eat *= 0.85;
    } else if (w === "snow") {
      c.speed *= 0.55;
      c.branch *= 0.4;
      c.fruit *= 0.25;
      c.eat *= 0.7;
      c.energyDrain *= 1.2;
    }
    return c;
  }

  function shadowOffset() {
    const s = season();
    if (s === "summer") return { x: 3.2, y: 4.4 };
    if (s === "winter") return { x: 1.4, y: 2.2 };
    if (s === "autumn") return { x: 2.8, y: 3.2 };
    return { x: 2.2, y: 3.4 };
  }

  function applyTheme() {
    document.body.dataset.season = season();
    document.body.dataset.weather = weather;
    statSeason.textContent = season();
    statWeather.textContent = weather;
  }

  function setWeather(next, fromUi) {
    weather = next;
    weatherTimer = 0;
    applyTheme();
    if (fromUi || !paused) {
      hint.textContent = WEATHER_HINTS[weather] || WEATHER_HINTS.clear;
    }
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
      id: nextTipId++,
      x,
      y,
      angle,
      energy,
      age: 0,
      alive: true,
      trail: [{ x, y, energy }],
    });
  }

  function buryTip(h) {
    if (h.trail.length > 4 && ghosts.length < MAX_GHOSTS) {
      ghosts.push({ trail: h.trail, age: 0 });
    }
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
    ghosts.length = 0;
    spores.length = 0;
    mushrooms.length = 0;
    crumbs.length = 0;
    flakes.length = 0;
    motes.length = 0;
    germinated = false;
    frame = 0;
    paused = false;
    nextTipId = 1;
    seasonIndex = 0;
    seasonTimer = 0;
    climateLocked = false;
    btnPause.textContent = "pause";
    hint.textContent = "A spore is looking for a cozy spot…";
    setWeather("clear", false);

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
    const wet = climate().diffuse;
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
        const mix = 0.03 * wet;
        nutrient[i] = nutrientBack[i] * (1 - mix) + avg * mix;
        if (nutrient[i] < 0.008) nutrient[i] *= 0.997;
      }
    }
  }

  function rainNourish() {
    const feed = climate().rainFeed;
    if (feed <= 0) return;
    for (let i = 0; i < 18; i++) {
      addPatch(rand(2, GRID - 2), rand(2, GRID - 2), 2.4, feed * 4);
    }
  }

  function stepClimate() {
    seasonTimer += 1;
    weatherTimer += 1;
    if (!climateLocked && seasonTimer >= SEASON_FRAMES) {
      seasonTimer = 0;
      seasonIndex = (seasonIndex + 1) % SEASONS.length;
      applyTheme();
      const allowed = WEATHER_BY_SEASON[season()];
      if (!allowed.includes(weather)) setWeather(pick(allowed), false);
    }
    if (!climateLocked && weatherTimer >= WEATHER_FRAMES) {
      setWeather(pick(WEATHER_BY_SEASON[season()]), false);
    }
    if (weather === "rain" || weather === "snow") spawnFlakes();
    if (season() === "autumn" && frame % 8 === 0) spawnMote();
    stepFlakes();
    stepMotes();
    if (weather === "rain" && frame % 6 === 0) rainNourish();
  }

  function spawnFlakes() {
    const want = weather === "snow" ? 70 : 90;
    while (flakes.length < want) {
      flakes.push({
        x: rand(-20, width + 20),
        y: rand(-height * 0.2, height),
        vx: weather === "snow" ? rand(-0.35, 0.35) : rand(-0.8, 0.2),
        vy: weather === "snow" ? rand(0.6, 1.5) : rand(6, 11),
        r: weather === "snow" ? rand(1.4, 3.2) : rand(0.6, 1.3),
        a: rand(0.25, 0.7),
        kind: weather,
      });
    }
  }

  function stepFlakes() {
    for (const f of flakes) {
      f.x += f.vx;
      f.y += f.vy;
      if (weather === "snow") f.x += Math.sin(frame * 0.04 + f.y * 0.02) * 0.4;
      if (f.y > height + 12) {
        f.y = -8;
        f.x = rand(-20, width + 20);
      }
    }
    if (weather !== "rain" && weather !== "snow") {
      for (let i = flakes.length - 1; i >= 0; i--) {
        flakes[i].y += flakes[i].vy;
        flakes[i].a -= 0.04;
        if (flakes[i].a <= 0 || flakes[i].y > height + 20) flakes.splice(i, 1);
      }
    }
  }

  function spawnMote() {
    if (motes.length > 40) return;
    motes.push({
      x: rand(0, width),
      y: rand(-20, height * 0.4),
      vx: rand(-0.4, 0.15),
      vy: rand(0.4, 1.1),
      rot: rand(0, Math.PI * 2),
      spin: rand(-0.04, 0.04),
      life: 1,
      hue: rand(18, 40),
    });
  }

  function stepMotes() {
    for (const m of motes) {
      m.x += m.vx;
      m.y += m.vy;
      m.rot += m.spin;
      m.life -= 0.003;
    }
    for (let i = motes.length - 1; i >= 0; i--) {
      if (motes[i].life <= 0 || motes[i].y > height + 20) motes.splice(i, 1);
    }
  }

  function stepTips() {
    const c = climate();
    const born = [];
    for (const h of tips) {
      if (!h.alive) continue;
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
      const speed = (0.16 + food * 0.18 + h.energy * 0.05) * c.speed;
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

      const eat = Math.min(food, 0.028 * c.eat);
      nutrient[idx(xi, yi)] = Math.max(0, food - eat);
      h.energy = clamp(h.energy * (1 - 0.0012 * c.energyDrain) + eat * 1.8, 0, 1);
      if (h.energy < 0.03) h.alive = false;

      h.trail.push({ x: h.x, y: h.y, energy: h.energy });
      if (h.trail.length > MAX_TRAIL) h.trail.shift();

      if (
        tips.length + born.length < MAX_TIPS &&
        h.age > 0.8 &&
        h.energy > 0.32 &&
        Math.random() < (0.006 + food * 0.01) * c.branch
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
        Math.random() < 0.00045 * c.fruit
      ) {
        mushrooms.push({ x: h.x, y: h.y, age: 0, wobble: rand(0, Math.PI * 2) });
      }
    }

    for (const b of born) spawnTip(b.x, b.y, b.angle, b.energy);
    for (let i = tips.length - 1; i >= 0; i--) {
      if (!tips[i].alive) {
        buryTip(tips[i]);
        tips.splice(i, 1);
      }
    }
    for (const g of ghosts) g.age += 0.01;
    while (ghosts.length && ghosts[0].age > 2.4) ghosts.shift();
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
      } else if (
        !s.hero &&
        s.age > s.life * 0.6 &&
        nutrient[idx(clamp(Math.floor(s.x), 0, GRID - 1), clamp(Math.floor(s.y), 0, GRID - 1))] > 0.35 &&
        tips.length < MAX_TIPS
      ) {
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
    const pal = SOIL[season()];
    ctx.fillStyle = `rgb(${pal.deep[0]}, ${pal.deep[1]}, ${pal.deep[2]})`;
    ctx.fillRect(0, 0, width, height);

    const ox = (width - GRID * cell) / 2;
    const oy = (height - GRID * cell) / 2;
    const img = ctx.createImageData(GRID, GRID);
    const data = img.data;
    const sunLift = weather === "sun" || weather === "warm" ? 18 : weather === "cold" || weather === "snow" ? -12 : 0;

    for (let y = 0; y < GRID; y++) {
      const shade = 0.72 + (y / GRID) * 0.38;
      for (let x = 0; x < GRID; x++) {
        const i = y * GRID + x;
        const n = nutrient[i];
        const p = i * 4;
        const r = lerp(pal.deep[0], pal.rich[0], n);
        const g = lerp(pal.deep[1], pal.rich[1], n);
        const b = lerp(pal.deep[2], pal.rich[2], n);
        const spark = n > 0.55 ? (n - 0.55) * 0.8 : 0;
        data[p] = clamp(r * shade + spark * pal.light[0] + sunLift, 0, 255);
        data[p + 1] = clamp(g * shade + spark * pal.light[1] + sunLift * 0.7, 0, 255);
        data[p + 2] = clamp(b * shade + spark * pal.light[2] + sunLift * 0.4, 0, 255);
        data[p + 3] = 255;
      }
    }

    const tmp = drawSoil._tmp || (drawSoil._tmp = document.createElement("canvas"));
    tmp.width = GRID;
    tmp.height = GRID;
    tmp.getContext("2d").putImageData(img, 0, 0);
    ctx.imageSmoothingEnabled = true;
    ctx.drawImage(tmp, ox, oy, GRID * cell, GRID * cell);

    const light = ctx.createRadialGradient(
      width * 0.72,
      height * 0.08,
      20,
      width * 0.5,
      height * 0.55,
      Math.max(width, height) * 0.85
    );
    if (weather === "sun" || weather === "warm") {
      light.addColorStop(0, "rgba(255, 230, 150, 0.22)");
      light.addColorStop(0.45, "rgba(255, 210, 120, 0.05)");
    } else if (weather === "cold" || weather === "snow") {
      light.addColorStop(0, "rgba(190, 220, 240, 0.16)");
      light.addColorStop(0.45, "rgba(140, 170, 200, 0.04)");
    } else {
      light.addColorStop(0, "rgba(255, 255, 230, 0.08)");
      light.addColorStop(0.45, "rgba(255, 255, 230, 0.02)");
    }
    light.addColorStop(1, "rgba(0, 0, 0, 0.28)");
    ctx.fillStyle = light;
    ctx.fillRect(ox, oy, GRID * cell, GRID * cell);

    for (const c of crumbs) {
      const w = gridToWorld(c.x, c.y);
      ctx.fillStyle = `rgba(20, 12, 6, ${0.22 * c.life})`;
      ctx.beginPath();
      ctx.ellipse(w.x + 2, w.y + 4, 16 * c.life, 6 * c.life, 0, 0, Math.PI * 2);
      ctx.fill();
      const g = ctx.createRadialGradient(w.x, w.y, 2, w.x, w.y, 28 * c.life);
      g.addColorStop(0, `rgba(232, 196, 112, ${0.55 * c.life})`);
      g.addColorStop(1, "rgba(232, 196, 112, 0)");
      ctx.fillStyle = g;
      ctx.beginPath();
      ctx.arc(w.x, w.y, 28 * c.life, 0, Math.PI * 2);
      ctx.fill();
    }
  }

  function strokeTrail(trail, offsetX, offsetY, fade) {
    if (trail.length < 2) return;
    ctx.beginPath();
    const first = gridToWorld(trail[0].x, trail[0].y);
    ctx.moveTo(first.x + offsetX, first.y + offsetY);
    for (let i = 1; i < trail.length; i++) {
      const w = gridToWorld(trail[i].x, trail[i].y);
      ctx.lineTo(w.x + offsetX, w.y + offsetY);
    }
    ctx.globalAlpha = fade;
    ctx.stroke();
    ctx.globalAlpha = 1;
  }

  function hyphaColor() {
    const s = season();
    if (s === "autumn") return { body: "rgba(255, 228, 186, 0.95)", glow: "rgba(255, 244, 214, 0.45)" };
    if (s === "winter") return { body: "rgba(236, 246, 255, 0.92)", glow: "rgba(210, 230, 255, 0.4)" };
    if (s === "summer") return { body: "rgba(255, 250, 220, 0.95)", glow: "rgba(255, 236, 170, 0.4)" };
    return { body: "rgba(255, 244, 214, 0.95)", glow: "rgba(255, 252, 236, 0.4)" };
  }

  function drawMycelium() {
    const ox = (width - GRID * cell) / 2;
    const oy = (height - GRID * cell) / 2;
    const sh = shadowOffset();
    const col = hyphaColor();
    ctx.save();
    ctx.beginPath();
    ctx.rect(ox, oy, GRID * cell, GRID * cell);
    ctx.clip();
    ctx.lineCap = "round";
    ctx.lineJoin = "round";

    const trails = [];
    for (const g of ghosts) trails.push({ trail: g.trail, fade: clamp(1 - g.age * 0.42, 0.12, 0.7), energy: 0.35 });
    for (const h of tips) {
      const last = h.trail[h.trail.length - 1];
      trails.push({ trail: h.trail, fade: 0.95, energy: last ? last.energy : h.energy });
    }

    ctx.strokeStyle = "rgba(8, 12, 10, 0.42)";
    for (const t of trails) {
      ctx.lineWidth = 3.6 + t.energy * 1.4;
      strokeTrail(t.trail, sh.x, sh.y, t.fade * 0.85);
    }

    ctx.strokeStyle = col.body;
    for (const t of trails) {
      ctx.lineWidth = 1.7 + t.energy * 1.5;
      strokeTrail(t.trail, 0, 0, t.fade);
    }

    ctx.strokeStyle = col.glow;
    for (const t of trails) {
      if (t.fade < 0.5) continue;
      ctx.lineWidth = 0.7 + t.energy * 0.5;
      strokeTrail(t.trail, -0.6, -0.8, t.fade * 0.55);
    }

    for (const h of tips) {
      const w = gridToWorld(h.x, h.y);
      ctx.fillStyle = "rgba(8, 12, 10, 0.35)";
      ctx.beginPath();
      ctx.ellipse(w.x + sh.x * 0.6, w.y + sh.y * 0.6, 3.2, 1.8, 0, 0, Math.PI * 2);
      ctx.fill();
      const glow = ctx.createRadialGradient(w.x - 1, w.y - 1, 0.4, w.x, w.y, 5 + h.energy * 3);
      glow.addColorStop(0, "rgba(255, 252, 240, 1)");
      glow.addColorStop(0.45, `rgba(255, 236, 190, ${0.7 + h.energy * 0.3})`);
      glow.addColorStop(1, "rgba(255, 236, 190, 0)");
      ctx.fillStyle = glow;
      ctx.beginPath();
      ctx.arc(w.x, w.y, 2.6 + h.energy * 2.1, 0, Math.PI * 2);
      ctx.fill();
    }

    for (const s of spores) {
      const w = gridToWorld(s.x, s.y);
      ctx.fillStyle = "rgba(10, 14, 12, 0.3)";
      ctx.beginPath();
      ctx.ellipse(w.x + 1.5, w.y + 3, s.hero ? 5 : 2.2, s.hero ? 2 : 1, 0, 0, Math.PI * 2);
      ctx.fill();
      ctx.fillStyle = s.hero ? "#f4e4c8" : "rgba(243, 234, 215, 0.7)";
      ctx.beginPath();
      ctx.arc(w.x, w.y, s.hero ? 4.5 : 1.8, 0, Math.PI * 2);
      ctx.fill();
      if (s.hero) {
        ctx.fillStyle = "rgba(255, 255, 255, 0.45)";
        ctx.beginPath();
        ctx.arc(w.x - 1.4, w.y - 1.4, 1.3, 0, Math.PI * 2);
        ctx.fill();
      }
    }

    for (const m of mushrooms) {
      const w = gridToWorld(m.x, m.y);
      const grow = clamp(m.age * 2.2, 0, 1);
      const bob = Math.sin(frame * 0.04 + m.wobble) * 0.8 * grow;
      const cap = 8.5 * grow;
      ctx.fillStyle = "rgba(8, 12, 10, 0.38)";
      ctx.beginPath();
      ctx.ellipse(w.x + 1.5, w.y + 8 * grow + bob, 7.5 * grow, 2.2 * grow, 0, 0, Math.PI * 2);
      ctx.fill();
      ctx.fillStyle = "#efe0c4";
      ctx.beginPath();
      ctx.roundRect
        ? ctx.roundRect(w.x - 1.6 * grow, w.y - 2 * grow + bob, 3.2 * grow, 9 * grow, 2 * grow)
        : ctx.rect(w.x - 1.6 * grow, w.y - 2 * grow + bob, 3.2 * grow, 9 * grow);
      ctx.fill();
      const capGrad = ctx.createLinearGradient(w.x, w.y - 8 * grow + bob, w.x, w.y + bob);
      capGrad.addColorStop(0, "#f08a6c");
      capGrad.addColorStop(0.55, "#e07a5f");
      capGrad.addColorStop(1, "#c45d45");
      ctx.fillStyle = capGrad;
      ctx.beginPath();
      ctx.ellipse(w.x, w.y - 3.2 * grow + bob, cap, cap * 0.62, 0, Math.PI, Math.PI * 2);
      ctx.fill();
      ctx.fillStyle = "rgba(255, 236, 214, 0.5)";
      ctx.beginPath();
      ctx.ellipse(w.x - cap * 0.32, w.y - 4.6 * grow + bob, 2.1 * grow, 1.3 * grow, -0.4, 0, Math.PI * 2);
      ctx.fill();
    }
    ctx.restore();
  }

  function drawWeather() {
    if (weather === "sun" || weather === "warm") {
      ctx.save();
      ctx.translate(width * 0.78, height * 0.08);
      ctx.rotate(frame * 0.002);
      ctx.strokeStyle = weather === "warm" ? "rgba(255, 196, 90, 0.12)" : "rgba(255, 224, 140, 0.14)";
      ctx.lineWidth = 10;
      for (let i = 0; i < 10; i++) {
        ctx.beginPath();
        ctx.moveTo(28, 0);
        ctx.lineTo(90 + Math.sin(frame * 0.03 + i) * 8, 0);
        ctx.stroke();
        ctx.rotate(Math.PI / 5);
      }
      ctx.restore();
      const sun = ctx.createRadialGradient(width * 0.78, height * 0.08, 8, width * 0.78, height * 0.08, 90);
      sun.addColorStop(0, "rgba(255, 244, 200, 0.85)");
      sun.addColorStop(0.35, "rgba(255, 210, 110, 0.35)");
      sun.addColorStop(1, "rgba(255, 210, 110, 0)");
      ctx.fillStyle = sun;
      ctx.beginPath();
      ctx.arc(width * 0.78, height * 0.08, 90, 0, Math.PI * 2);
      ctx.fill();
    }

    if (season() === "autumn") {
      for (const m of motes) {
        ctx.save();
        ctx.translate(m.x, m.y);
        ctx.rotate(m.rot);
        ctx.fillStyle = `hsla(${m.hue}, 70%, 48%, ${0.7 * m.life})`;
        ctx.beginPath();
        ctx.ellipse(0, 0, 5, 2.4, 0, 0, Math.PI * 2);
        ctx.fill();
        ctx.restore();
      }
    }

    if (flakes.length) {
      for (const f of flakes) {
        if (f.kind === "snow") {
          ctx.fillStyle = `rgba(255, 255, 255, ${f.a})`;
          ctx.beginPath();
          ctx.arc(f.x, f.y, f.r, 0, Math.PI * 2);
          ctx.fill();
        } else {
          ctx.strokeStyle = `rgba(186, 214, 230, ${f.a})`;
          ctx.lineWidth = f.r;
          ctx.beginPath();
          ctx.moveTo(f.x, f.y);
          ctx.lineTo(f.x + f.vx * 1.4, f.y + 10);
          ctx.stroke();
        }
      }
    }

    if (weather === "cold" || weather === "snow") {
      ctx.fillStyle = "rgba(180, 210, 230, 0.08)";
      ctx.fillRect(0, 0, width, height);
    }
    if (weather === "rain") {
      ctx.fillStyle = "rgba(40, 60, 70, 0.08)";
      ctx.fillRect(0, 0, width, height);
    }
  }

  function moodText() {
    if (!germinated) return "waking up";
    if (paused) return "napping";
    if (weather === "snow") return "bundled up";
    if (weather === "rain") return "drinking rain";
    if (weather === "sun") return "basking";
    if (weather === "warm") return "toasty";
    if (weather === "cold") return "shivering";
    if (tips.length > 70) return "busy little city";
    if (tips.length > 22) return "stretching";
    return "sniffing around";
  }

  function updateHud() {
    applyTheme();
    statTips.textContent = `tips ${tips.length}`;
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
      stepClimate();
      if (frame % 2 === 0) diffuse();
      stepSpores();
      if (germinated) stepTips();
    } else {
      if (weather === "rain" || weather === "snow") spawnFlakes();
      if (season() === "autumn" && frame % 8 === 0) spawnMote();
      stepFlakes();
      stepMotes();
    }
    drawSoil();
    drawMycelium();
    drawWeather();
    if (frame % 8 === 0) updateHud();
    requestAnimationFrame(tick);
  }

  function canvasPoint(event) {
    const rect = canvas.getBoundingClientRect();
    const t = event.touches ? event.touches[0] : event;
    return { x: t.clientX - rect.left, y: t.clientY - rect.top };
  }

  function cycleSky() {
    const options = WEATHER_BY_SEASON[season()];
    const i = options.indexOf(weather);
    climateLocked = true;
    setWeather(options[(i + 1) % options.length], true);
    seasonTimer = 0;
    hint.textContent = `${WEATHER_HINTS[weather]} (sky paused — new spore restores the cycle)`;
  }

  canvas.addEventListener("pointerdown", (event) => {
    event.preventDefault();
    const p = canvasPoint(event);
    dropCrumb(p.x, p.y, true);
  });

  btnFeed.addEventListener("click", () => {
    dropCrumb(width * 0.5 + rand(-80, 80), height * 0.5 + rand(-80, 80), true);
  });

  btnSky.addEventListener("click", cycleSky);

  statSeason.addEventListener("click", () => {
    seasonIndex = (seasonIndex + 1) % SEASONS.length;
    seasonTimer = 0;
    climateLocked = true;
    const allowed = WEATHER_BY_SEASON[season()];
    if (!allowed.includes(weather)) setWeather(pick(allowed), true);
    else applyTheme();
    hint.textContent = `${season()} has arrived. (cycle paused — new spore restores it)`;
  });

  btnPause.addEventListener("click", () => {
    paused = !paused;
    btnPause.textContent = paused ? "grow" : "pause";
    hint.textContent = paused ? "Shhh. It is napping." : WEATHER_HINTS[weather];
  });

  btnReset.addEventListener("click", () => {
    seedWorld();
    updateHud();
  });

  window.addEventListener("resize", resize);
  resize();
  seedWorld();
  updateHud();
  requestAnimationFrame(tick);
})();
