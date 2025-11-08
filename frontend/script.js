function sendPostRequest(url, data) {
  return fetch(url, {
    method: "POST",
    body: JSON.stringify(data),
    headers: {
      "Content-Type": "application/json",
    },
  });
}


const Toast = Swal.mixin({
  toast: true,
  position: "bottom-end",
  showConfirmButton: false,
  timer: 3000,
  timerProgressBar: true,
  didOpen: (toast) => {
    toast.onmouseenter = Swal.stopTimer;
    toast.onmouseleave = Swal.resumeTimer;
  }
});

const THEME_KEY = "pop-booking-theme";
const THEMES = {
  DEFAULT: { label: "Default" },
  CHRISTMAS: { label: "Christmas" },
};
const DEFAULT_THEME = "DEFAULT";
const THEME_CLASS_PREFIX = "theme-";
let activeTheme = DEFAULT_THEME;

const SnowEffect = (() => {
  const TARGET_FPS = 60;
  const FIXED_TIMESTEP = 1000 / TARGET_FPS;
  const MAX_STEPS_PER_FRAME = 4;
  const MAX_RENDER_FPS = 60;
  const MIN_RENDER_INTERVAL = 1000 / MAX_RENDER_FPS;
  const MAX_ACTIVE_PARTICLES = 320;
  const MAX_SETTLED_PARTICLES = Infinity;
  const GRID_COLS = 32;
  const GRID_ROWS = 22;
  const RELAXATION_STEPS = 1;
  const GRAVITY = 0.0035;
  const TERMINAL_VELOCITY = 1.9;
  const FREEZE_THRESHOLD_FRAMES = 600;
  const FREEZE_THRESHOLD_MS = FREEZE_THRESHOLD_FRAMES * FIXED_TIMESTEP;
  const STILLNESS_EPSILON = 0.01;
  const pointerWind = { current: 0, target: 0 };
  let canvas = null;
  let ctx = null;
  let width = window.innerWidth;
  let height = window.innerHeight;
  let particles = [];
  let settledParticles = [];
  let frozenParticles = [];
  let particleOrder = [];
  let particleOrderReverse = [];
  let animationFrame = null;
  let isActive = false;
  let listenersBound = false;
  let snowColor = "rgba(255,255,255,0.95)";
  let snowShadowColor = "rgba(15,23,42,0.2)";
  let snowShadowBlur = 5;
  let pileColor = "rgba(235,245,255,0.92)";
  let gridCellWidth = width / GRID_COLS;
  let gridCellHeight = height / GRID_ROWS;
  let spatialGrid = [];
  let accumulator = 0;
  let lastFrameTime = 0;
  let fpsOverlayEl = null;
  let fpsVisible = false;
  let fpsDisplayValue = TARGET_FPS;
  let lastRenderTimestamp = 0;

  function ensureCanvas() {
    if (canvas) return true;
    canvas = document.getElementById("snow-canvas");
    if (!canvas) return false;
    ctx = canvas.getContext("2d");
    bindListeners();
    resizeCanvas();
    return true;
  }

  function bindListeners() {
    if (listenersBound) return;
    window.addEventListener("resize", resizeCanvas);
    window.addEventListener("pointermove", handlePointerMove);
    listenersBound = true;
  }

  function initSpatialGrid() {
    spatialGrid = new Array(GRID_ROWS).fill(null).map(() =>
      new Array(GRID_COLS).fill(null).map(() => [])
    );
  }

  function resetSpatialGrid() {
    for (let row = 0; row < GRID_ROWS; row += 1) {
      for (let col = 0; col < GRID_COLS; col += 1) {
        spatialGrid[row][col].length = 0;
      }
    }
  }

  function addParticleToGrid(index, type) {
    const source =
      type === "settled"
        ? settledParticles
        : type === "frozen"
          ? frozenParticles
          : particles;
    const particle = source[index];
    if (!particle) return;
    const col = Math.max(
      0,
      Math.min(GRID_COLS - 1, Math.floor(particle.x / gridCellWidth || 0))
    );
    const row = Math.max(
      0,
      Math.min(GRID_ROWS - 1, Math.floor(particle.y / gridCellHeight || 0))
    );
    spatialGrid[row][col].push({ type, index });
  }

  function resizeCanvas() {
    if (!canvas) return;
    width = window.innerWidth;
    height = window.innerHeight;
    gridCellWidth = Math.max(8, width / GRID_COLS);
    gridCellHeight = Math.max(8, height / GRID_ROWS);
    canvas.width = width;
    canvas.height = height;
    initSpatialGrid();
    settledParticles = [];
    frozenParticles = [];
    accumulator = 0;
    lastFrameTime = 0;
    lastRenderTimestamp = 0;
    if (isActive && particles.length) {
      particles.forEach((particle) => {
        particle.x = Math.random() * width;
        particle.y = Math.random() * height;
      });
    }
  }

  function handlePointerMove(event) {
    if (!width) return;
    const ratio = (event.clientX / width) * 2 - 1;
    pointerWind.target = ratio * 0.8;
  }

  function createParticle(initialY = null) {
    const radius = Math.random() * 2 + 1.2;
    return {
      x: Math.random() * width,
      y: initialY === null ? Math.random() * height : initialY,
      radius,
      vx: 0,
      vy: Math.random() * 0.7 + 0.25,
      sway: Math.random() * 0.65 + 0.3,
      angle: Math.random() * Math.PI * 2,
      angleSpeed: Math.random() * 0.03 + 0.007,
    };
  }

  function recycleParticle(target, initialY = null, keepMomentum = false) {
    const next = createParticle(initialY);
    if (keepMomentum) {
      next.angle = target.angle;
      next.angleSpeed = target.angleSpeed;
    }
    Object.assign(target, next);
  }

  function populateParticles() {
    particles = [];
    for (let i = 0; i < MAX_ACTIVE_PARTICLES; i += 1) {
      particles.push(createParticle());
    }
    particleOrder = particles.map((_, index) => index);
    particleOrderReverse = [...particleOrder].reverse();
  }

  function clamp(value, min, max) {
    return Math.min(Math.max(value, min), max);
  }

  function wrapParticle(particle) {
    if (particle.x > width + 5) {
      particle.x = -5;
    } else if (particle.x < -5) {
      particle.x = width + 5;
    }
  }

  function rebuildSpatialGrid(includeActive = true) {
    if (!spatialGrid.length) return;
    resetSpatialGrid();
    for (let i = 0; i < settledParticles.length; i += 1) {
      addParticleToGrid(i, "settled");
    }
    for (let i = 0; i < frozenParticles.length; i += 1) {
      addParticleToGrid(i, "frozen");
    }
    if (!includeActive) return;
    for (let i = 0; i < particles.length; i += 1) {
      addParticleToGrid(i, "active");
    }
  }

  function getNeighborEntries(particle) {
    if (!gridCellWidth || !gridCellHeight || !spatialGrid.length) return [];
    const reachX = particle.radius * 2.5;
    const reachY = particle.radius * 2.5;
    const minCol = Math.max(0, Math.floor((particle.x - reachX) / gridCellWidth));
    const maxCol = Math.min(GRID_COLS - 1, Math.floor((particle.x + reachX) / gridCellWidth));
    const minRow = Math.max(0, Math.floor((particle.y - reachY) / gridCellHeight));
    const maxRow = Math.min(GRID_ROWS - 1, Math.floor((particle.y + reachY) / gridCellHeight));
    const neighbors = [];
    for (let row = minRow; row <= maxRow; row += 1) {
      for (let col = minCol; col <= maxCol; col += 1) {
        const cell = spatialGrid[row][col];
        for (let i = 0; i < cell.length; i += 1) {
          neighbors.push(cell[i]);
        }
      }
    }
    return neighbors;
  }

  function settleParticle(particle) {
    const clampedX = clamp(
      particle.x,
      particle.radius,
      Math.max(particle.radius, width - particle.radius)
    );
    const clampedY = Math.min(height - particle.radius, particle.y);
    settledParticles.push({
      x: clampedX,
      y: clampedY,
      radius: particle.radius,
      stillTime: 0,
      lastX: clampedX,
      lastY: clampedY,
    });
    if (Number.isFinite(MAX_SETTLED_PARTICLES) && settledParticles.length > MAX_SETTLED_PARTICLES) {
      settledParticles.splice(0, settledParticles.length - MAX_SETTLED_PARTICLES);
    }
    if (spatialGrid.length) {
      addParticleToGrid(settledParticles.length - 1, "settled");
    }
    recycleParticle(particle, -40, true);
  }

  function resolveParticleCollisions(index) {
    const particle = particles[index];
    if (!particle) return false;
    let supportContacts = 0;
    const neighbors = getNeighborEntries(particle);

    for (let i = 0; i < neighbors.length; i += 1) {
      const entry = neighbors[i];
      if (entry.type === "active" && entry.index === index) continue;
      const otherSource = entry.type === "active" ? particles : settledParticles;
      const neighbor = otherSource[entry.index];
      if (!neighbor) continue;

      const dx = particle.x - neighbor.x;
      const dy = particle.y - neighbor.y;
      const distance = Math.sqrt(dx * dx + dy * dy) || 0.0001;
      const minDist = particle.radius + neighbor.radius + 0.25;
      if (distance >= minDist) continue;

      const nx = dx / distance;
      const ny = dy / distance;
      const overlap = minDist - distance;
      const compression = overlap * 0.65 + 0.02;

      particle.x += nx * compression;
      particle.y += ny * compression;
      particle.vx += nx * compression * 0.04;
      particle.vy += ny * compression * 0.04;

      if (entry.type === "active") {
        neighbor.x -= nx * compression;
        neighbor.y -= ny * compression;
        neighbor.vx -= nx * compression * 0.04;
        neighbor.vy -= ny * compression * 0.04;
      }

      if (ny > 0.35) {
        supportContacts += 1;
      }
    }

    if (particle.y + particle.radius >= height - 1) {
      particle.y = height - particle.radius;
      supportContacts += 2;
    }

    wrapParticle(particle);

    if (supportContacts >= 2) {
      settleParticle(particle);
      return true;
    }

    return false;
  }

  function updateActiveParticles(wind, deltaRatio) {
    for (let i = 0; i < particles.length; i += 1) {
      const particle = particles[i];
      particle.angle += particle.angleSpeed * deltaRatio;
      const sway = Math.sin(particle.angle) * particle.sway;
      particle.vx += (sway + wind - particle.vx) * 0.08 * deltaRatio;
      particle.vy = Math.min(
        particle.vy + GRAVITY * (1 + particle.radius * 0.35) * deltaRatio,
        TERMINAL_VELOCITY * 1.35
      );
      particle.x += particle.vx * deltaRatio;
      particle.y += particle.vy * 1.8 * deltaRatio;
      wrapParticle(particle);
    }
  }

  function runCollisionRelaxation() {
    if (!particles.length || !particleOrder.length) return;
    rebuildSpatialGrid(true);
    for (let step = 0; step < RELAXATION_STEPS; step += 1) {
      const order = step % 2 === 0 ? particleOrder : particleOrderReverse;
      for (let i = 0; i < order.length; i += 1) {
        resolveParticleCollisions(order[i]);
      }
      if (step < RELAXATION_STEPS - 1) {
        rebuildSpatialGrid(true);
      }
    }
  }

  function freezeCalmSettledParticles(deltaMs) {
    if (!settledParticles.length) return;
    for (let i = settledParticles.length - 1; i >= 0; i -= 1) {
      const particle = settledParticles[i];
      const dx = particle.x - particle.lastX;
      const dy = particle.y - particle.lastY;
      const moved = Math.abs(dx) + Math.abs(dy) > STILLNESS_EPSILON;
      if (moved) {
        particle.lastX = particle.x;
        particle.lastY = particle.y;
        particle.stillTime = 0;
        continue;
      }
      particle.stillTime += deltaMs;
      if (particle.stillTime >= FREEZE_THRESHOLD_MS) {
        frozenParticles.push({
          x: particle.x,
          y: particle.y,
          radius: particle.radius,
        });
        settledParticles.splice(i, 1);
      }
    }
  }

  function drawSettledParticles() {
    if (!ctx) return;
    ctx.save();
    ctx.fillStyle = pileColor;
    ctx.shadowColor = snowShadowColor;
    ctx.shadowBlur = Math.max(2, snowShadowBlur * 0.5);
    const renderParticle = (particle) => {
      ctx.beginPath();
      ctx.arc(particle.x, particle.y, particle.radius, 0, Math.PI * 2);
      ctx.fill();
    };
    for (let i = 0; i < frozenParticles.length; i += 1) {
      renderParticle(frozenParticles[i]);
    }
    for (let i = 0; i < settledParticles.length; i += 1) {
      renderParticle(settledParticles[i]);
    }
    ctx.restore();
  }

  function drawActiveParticles() {
    if (!ctx) return;
    for (let i = 0; i < particles.length; i += 1) {
      const particle = particles[i];
      ctx.beginPath();
      ctx.arc(particle.x, particle.y, particle.radius, 0, Math.PI * 2);
      ctx.fill();
    }
  }

  function stepSimulation(deltaMs) {
    const deltaRatio = deltaMs / FIXED_TIMESTEP;
    pointerWind.current += (pointerWind.target - pointerWind.current) * 0.02 * deltaRatio;
    const wind = pointerWind.current * 2.2;
    updateActiveParticles(wind, deltaRatio);
    runCollisionRelaxation();
    freezeCalmSettledParticles(deltaMs);
  }

  function renderScene() {
    if (!ctx) return;
    ctx.clearRect(0, 0, width, height);
    drawSettledParticles();
    ctx.fillStyle = snowColor;
    ctx.shadowColor = snowShadowColor;
    ctx.shadowBlur = snowShadowBlur;
    drawActiveParticles();
  }

  function getActiveParticleCount() {
    return particles.length + settledParticles.length;
  }

  function getInactiveParticleCount() {
    return frozenParticles.length;
  }

  function ensureFpsOverlay() {
    if (fpsOverlayEl) return;
    fpsOverlayEl = document.createElement("div");
    fpsOverlayEl.id = "fps-overlay";
    fpsOverlayEl.style.display = "none";
    fpsOverlayEl.textContent = `${TARGET_FPS.toFixed(1)} fps | active ${getActiveParticleCount()} | inactive ${getInactiveParticleCount()}`;
    document.body.appendChild(fpsOverlayEl);
  }

  function toggleFpsOverlay() {
    ensureFpsOverlay();
    fpsVisible = !fpsVisible;
    fpsOverlayEl.style.display = fpsVisible ? "block" : "none";
  }

  function updateFpsOverlay(deltaMs) {
    if (!fpsVisible || !fpsOverlayEl) return;
    const instantaneous = deltaMs > 0 ? 1000 / deltaMs : TARGET_FPS;
    fpsDisplayValue = fpsDisplayValue * 0.9 + instantaneous * 0.1;
    fpsOverlayEl.textContent = `${fpsDisplayValue.toFixed(1)} fps | active ${getActiveParticleCount()} | inactive ${getInactiveParticleCount()}`;
  }

  function draw(timestamp = performance.now()) {
    if (!ctx || !isActive) return;
    if (!lastFrameTime) lastFrameTime = timestamp;
    const delta = Math.min(1000, timestamp - lastFrameTime);
    lastFrameTime = timestamp;
    accumulator += delta;
    let steps = 0;
    while (accumulator >= FIXED_TIMESTEP && steps < MAX_STEPS_PER_FRAME) {
      stepSimulation(FIXED_TIMESTEP);
      accumulator -= FIXED_TIMESTEP;
      steps += 1;
    }
    if (steps === MAX_STEPS_PER_FRAME && accumulator > FIXED_TIMESTEP) {
      accumulator = FIXED_TIMESTEP;
    }
    const shouldRender =
      !lastRenderTimestamp || (timestamp - lastRenderTimestamp) >= MIN_RENDER_INTERVAL;
    if (shouldRender) {
      renderScene();
      updateFpsOverlay(delta);
      lastRenderTimestamp = timestamp;
    }
    animationFrame = window.requestAnimationFrame(draw);
  }

  function start() {
    if (isActive) return;
    if (!ensureCanvas()) return;
    isActive = true;
    document.body.classList.add("snow-active");
    populateParticles();
    settledParticles = [];
    frozenParticles = [];
    pointerWind.current = 0;
    pointerWind.target = 0;
    accumulator = 0;
    lastFrameTime = 0;
    lastRenderTimestamp = 0;
    fpsDisplayValue = TARGET_FPS;
    ctx.clearRect(0, 0, width, height);
    animationFrame = window.requestAnimationFrame(draw);
  }

  function stop() {
    if (!isActive) return;
    isActive = false;
    document.body.classList.remove("snow-active");
    pointerWind.target = 0;
    if (animationFrame) {
      window.cancelAnimationFrame(animationFrame);
      animationFrame = null;
    }
    if (ctx) {
      ctx.clearRect(0, 0, width, height);
      ctx.shadowBlur = 0;
      ctx.shadowColor = "transparent";
    }
    settledParticles = [];
    frozenParticles = [];
    particles = [];
    particleOrder = [];
    particleOrderReverse = [];
    accumulator = 0;
    lastFrameTime = 0;
    lastRenderTimestamp = 0;
  }

  function configure(options = {}) {
    snowColor = options.color || snowColor;
    snowShadowColor = options.shadowColor || snowShadowColor;
    snowShadowBlur = typeof options.shadowBlur === "number" ? options.shadowBlur : snowShadowBlur;
    pileColor = options.pileColor || pileColor;
  }

  return {
    start,
    stop,
    configure,
    toggleFpsOverlay,
  };
})();

function getThemeClassName(theme) {
  return `${THEME_CLASS_PREFIX}${theme.toLowerCase()}`;
}

function applyTheme(themeName) {
  const root = document.documentElement;
  Object.keys(THEMES).forEach((theme) => {
    root.classList.remove(getThemeClassName(theme));
  });
  root.classList.add(getThemeClassName(themeName));
  activeTheme = themeName;
}

function persistTheme(themeName) {
  try {
    localStorage.setItem(THEME_KEY, themeName);
  } catch (error) {
    console.warn("Unable to persist theme preference", error);
  }
}

function updateThemeSelector(themeName) {
  const selector = document.getElementById("theme-selector");
  if (selector && selector.value !== themeName) {
    selector.value = themeName;
  }
}

function setTheme(themeName) {
  const nextTheme = THEMES[themeName] ? themeName : DEFAULT_THEME;
  applyTheme(nextTheme);
  persistTheme(nextTheme);
  updateThemeSelector(nextTheme);
  syncThemeEffects(nextTheme);
}

function syncThemeEffects(themeName) {
  if (themeName === "CHRISTMAS") {
    SnowEffect.configure({
      color: "rgba(233, 249, 255, 0.97)",
      shadowColor: "rgba(56, 189, 248, 0.55)",
      shadowBlur: 8,
      pileColor: "rgba(216, 241, 255, 0.95)",
    });
    SnowEffect.start();
  } else {
    SnowEffect.configure({
      color: "rgba(255, 255, 255, 0.9)",
      shadowColor: "rgba(15, 23, 42, 0.15)",
      shadowBlur: 4,
      pileColor: "rgba(235,245,255,0.92)",
    });
    SnowEffect.stop();
  }
}

function hydrateThemeSelectorOptions(selector) {
  selector.innerHTML = "";
  Object.entries(THEMES).forEach(([value, config]) => {
    const option = document.createElement("option");
    option.value = value;
    option.textContent = config.label;
    selector.appendChild(option);
  });
}

function initThemeSelector() {
  const selector = document.getElementById("theme-selector");
  if (!selector) return;
  hydrateThemeSelectorOptions(selector);
  selector.addEventListener("change", (event) => setTheme(event.target.value));
  updateThemeSelector(activeTheme);
}

(function bootstrapTheme() {
  let storedTheme = null;
  try {
    storedTheme = localStorage.getItem(THEME_KEY);
  } catch (_) {
    // Access to localStorage can fail in private browsing
  }
  const initialTheme = THEMES[storedTheme] ? storedTheme : DEFAULT_THEME;
  applyTheme(initialTheme);
})();

document.addEventListener("DOMContentLoaded", function () {
  initThemeSelector();
  syncThemeEffects(activeTheme);
});

document.addEventListener('keydown', function (event) {
  if (event.key === 'h' || event.key === 'H') {
    SnowEffect.toggleFpsOverlay();
    return;
  }
  if (event.key === 'Escape') {
    Swal.clickCancel();
  }
  // Prevent Enter from confirming if Select2 dropdown is open
  if (event.key === "Enter") {
    if (document.querySelector('.select2-container--open')) {
      return;
    }
    Swal.clickConfirm();
  }

}, true); //use capture so it triggers before bootstrap

document.addEventListener('swiped-left', function (e) {
  calendar.next();
});

document.addEventListener('swiped-right', function (e) {
  calendar.prev();
});

window.mobilecheck = function () {
  var check = false;
  (function (a) { if (/(android|bb\d+|meego).+mobile|avantgo|bada\/|blackberry|blazer|compal|elaine|fennec|hiptop|iemobile|ip(hone|od)|iris|kindle|lge |maemo|midp|mmp|mobile.+firefox|netfront|opera m(ob|in)i|palm( os)?|phone|p(ixi|re)\/|plucker|pocket|psp|series(4|6)0|symbian|treo|up\.(browser|link)|vodafone|wap|windows ce|xda|xiino/i.test(a) || /1207|6310|6590|3gso|4thp|50[1-6]i|770s|802s|a wa|abac|ac(er|oo|s\-)|ai(ko|rn)|al(av|ca|co)|amoi|an(ex|ny|yw)|aptu|ar(ch|go)|as(te|us)|attw|au(di|\-m|r |s )|avan|be(ck|ll|nq)|bi(lb|rd)|bl(ac|az)|br(e|v)w|bumb|bw\-(n|u)|c55\/|capi|ccwa|cdm\-|cell|chtm|cldc|cmd\-|co(mp|nd)|craw|da(it|ll|ng)|dbte|dc\-s|devi|dica|dmob|do(c|p)o|ds(12|\-d)|el(49|ai)|em(l2|ul)|er(ic|k0)|esl8|ez([4-7]0|os|wa|ze)|fetc|fly(\-|_)|g1 u|g560|gene|gf\-5|g\-mo|go(\.w|od)|gr(ad|un)|haie|hcit|hd\-(m|p|t)|hei\-|hi(pt|ta)|hp( i|ip)|hs\-c|ht(c(\-| |_|a|g|p|s|t)|tp)|hu(aw|tc)|i\-(20|go|ma)|i230|iac( |\-|\/)|ibro|idea|ig01|ikom|im1k|inno|ipaq|iris|ja(t|v)a|jbro|jemu|jigs|kddi|keji|kgt( |\/)|klon|kpt |kwc\-|kyo(c|k)|le(no|xi)|lg( g|\/(k|l|u)|50|54|\-[a-w])|libw|lynx|m1\-w|m3ga|m50\/|ma(te|ui|xo)|mc(01|21|ca)|m\-cr|me(rc|ri)|mi(o8|oa|ts)|mmef|mo(01|02|bi|de|do|t(\-| |o|v)|zz)|mt(50|p1|v )|mwbp|mywa|n10[0-2]|n20[2-3]|n30(0|2)|n50(0|2|5)|n7(0(0|1)|10)|ne((c|m)\-|on|tf|wf|wg|wt)|nok(6|i)|nzph|o2im|op(ti|wv)|oran|owg1|p800|pan(a|d|t)|pdxg|pg(13|\-([1-8]|c))|phil|pire|pl(ay|uc)|pn\-2|po(ck|rt|se)|prox|psio|pt\-g|qa\-a|qc(07|12|21|32|60|\-[2-7]|i\-)|qtek|r380|r600|raks|rim9|ro(ve|zo)|s55\/|sa(ge|ma|mm|ms|ny|va)|sc(01|h\-|oo|p\-)|sdk\/|se(c(\-|0|1)|47|mc|nd|ri)|sgh\-|shar|sie(\-|m)|sk\-0|sl(45|id)|sm(al|ar|b3|it|t5)|so(ft|ny)|sp(01|h\-|v\-|v )|sy(01|mb)|t2(18|50)|t6(00|10|18)|ta(gt|lk)|tcl\-|tdg\-|tel(i|m)|tim\-|t\-mo|to(pl|sh)|ts(70|m\-|m3|m5)|tx\-9|up(\.b|g1|si)|utst|v400|v750|veri|vi(rg|te)|vk(40|5[0-3]|\-v)|vm40|voda|vulc|vx(52|53|60|61|70|80|81|83|85|98)|w3c(\-| )|webc|whit|wi(g |nc|nw)|wmlb|wonu|x700|yas\-|your|zeto|zte\-/i.test(a.substr(0, 4))) check = true; })(navigator.userAgent || navigator.vendor || window.opera);
  return check;
};


function switchPages(toPage) {
  let messageDiv = document.getElementById("notices-div");
  let calendarDiv = document.getElementById("calendar-div");
  let messageButton = document.getElementById("notices");
  let calendarButton = document.getElementById("calendar");

  switch (toPage) {
    case "notices":
      messageDiv.removeAttribute("hidden");
      messageButton.setAttribute("class", "active");
      calendarDiv.setAttribute("hidden", "");
      calendarButton.setAttribute("class", "");
      break;
    case "calendar":
      messageDiv.setAttribute("hidden", "");
      messageButton.setAttribute("class", "");
      calendarDiv.removeAttribute("hidden");
      calendarButton.setAttribute("class", "active");
      break;
  }
}

async function logout() {
  let response = await fetch("/api/logout");

  if (response.status !== 200) {
    Toast.fire({
      icon: "error",
      title: "Logout failed",
    });
    return;
  }

  onSignOut();

  Toast.fire({
    icon: "success",
    title: "Logout successful",
  });
}

async function showLoginForm() {
  return new Promise((resolve) => {
    Swal.fire({
      title: 'Login',
      html: `
        <label for="username">Username:</label>
        <input type="text" id="username" class="swal2-input" placeholder="Username" style="margin: 5pt 5pt" required>
        <label for="password">Password:</label>
        <input type="password" id="password" class="swal2-input" placeholder="Password" style="margin: 5pt 5pt" required>
        <p id="nextlogintime"></p>
        `,
      showConfirmButton: true,
      showCancelButton: true,
      padding: '1em',
      confirmButtonText: 'Login',
      confirmButtonColor: '#4BB543',
      allowEnterKey: true,
      cancelButtonText: 'Cancel',
      focusConfirm: false,
      preConfirm: async () => {
        try {
          const response = await sendPostRequest("/api/login", {
            username: document.getElementById("username").value,
            password: document.getElementById("password").value,
          });

          if (response.status === 200) {
            // Handle the successful response here
            Toast.fire({
              icon: "success",
              title: "Login successful",
            });

            const data = await response.json();
            onSignIn(data.user);
            return true;
          } else {
            const errorText = await response.text();
            document.getElementById("nextlogintime").innerHTML = errorText;
            return false;
          }
        } catch (error) {
          Toast.fire({
            icon: "error",
            title: "Login failed",
            text: "Something went wrong",
          });
          return false;
        }
      },
      allowOutsideClick: () => !Swal.isLoading()
    }).then((result) => {
      if (result.dismiss === Swal.DismissReason.cancel) {
        Toast.fire({
          icon: "error",
          title: "Login cancelled",
        });
        resolve(false);
      }
    });
  });
}

async function loadEvents(_, successCallback, failureCallback) {
  try {
    let response = await fetch("/api/book/events");
    let events = await response.json();

    events = events.map((event) => {
      event.start = new Date(event.start);
      event.end = new Date(event.end);
      if (event.owner == room) {
        event.title = "You, " + event.title.split(" ").slice(2).join(" ");
        event.editable = true;
      }
      return event;
    });
    successCallback(events);
  } catch (error) {
    failureCallback(error);
  }
}

function onResize(info) {
  let start = calendar.formatIso(info.event.start).slice(0, -6);
  let end = calendar.formatIso(info.event.end).slice(0, -6);
  let id = parseInt(info.event.id, 10);
  console.log(start, end, id);

  Swal.fire({
    title: 'Reschedule Booking',
    html: `
      <label for="start">New Start Time:</label>
      <input type="datetime-local" id="start" name="start" value="${start}" required>
      <br>
      <label for="end">New End Time:</label>
      <input type="datetime-local" id="end" name="end" value="${end}" required>
    `,
    showCancelButton: true,
    confirmButtonText: 'Reschedule',
    preConfirm: async () => {
      const start = rfc3339(document.getElementById('start').value);
      const end = rfc3339(document.getElementById('end').value);
      reschedule(start, end, id);
    }
  });

}

function reschedule(start_str, end_str, id) {
  const start = rfc3339(start_str);
  const end = rfc3339(end_str);
  sendPostRequest("/api/book/secure/change", {
    start_time: start,
    end_time: end,
    id: parseInt(id, 10),
  }).then((response) => {
    if (response.ok) {
      Toast.fire({
        icon: "success",
        title: "Booking rescheduled"
      });
    } else {
      response.text().then((errorText) => {
        Toast.fire({
          icon: "error",
          title: "Booking reschedule failed",
          text: errorText,
        });
      });
    }
    calendar.refetchEvents();
  });
}


document.addEventListener("DOMContentLoaded", function () {
  var calendarEl = document.getElementById("calendar-div");
  calendar = new FullCalendar.Calendar(calendarEl, {
    initialView: window.mobilecheck() ? "timeGridDay" : "month",
    events: loadEvents,
    height: "100%",
    selectable: true,
    selectMirror: true,
    unselectAuto: false,
    eventClick: handle_event_click,
    weekNumbers: true,
    selectMinDistance: 10,
    select: calendarSelect,
    eventResize: onResize,
    eventDrop: onResize,
    customButtons: {
      newBookingButton: {
        text: 'Create Booking',
        click: function () {
          let now = new Date();
          now.setMinutes(now.getMinutes() - now.getTimezoneOffset());
          now.setDate(now.getDate() + 1);
          let start = now.toISOString().slice(0, -8);
          let end = new Date(now.getTime() + 3600000).toISOString().slice(0, -8);
          bookingPopup(start, end);
        }
      }
    },
    headerToolbar: {
      left: 'today newBookingButton',
      center: 'title',
      right: 'month,timeGridWeek,timeGridDay,prev,next'
    },
    views: {
      month: {
        type: 'dayGridMonth',
        buttonText: 'Month',
        dateClick: (info) => { calendar.changeView('timeGridDay', info.date) },
        dayMaxEventRows: 3,
        selectable: false,
        dayMaxEvents: true,
        eventTimeFormat: {
          hour: 'numeric',
          minute: '2-digit',
          omitZeroMinute: true,
        }
      },
      timeGridWeek: {
        type: 'timeGrid',
        allDaySlot: false,
        slotDuration: '00:15:00',
        dateClick: (info) => {
          let start = new Date(info.date.getTime());
          start.setMinutes(start.getMinutes() - start.getTimezoneOffset());
          let end = new Date(info.date.getTime() + 3600000);
          end.setMinutes(end.getMinutes() - end.getTimezoneOffset());

          bookingPopup(start.toISOString().slice(0, -8), end.toISOString().slice(0, -8));
        },
        slotLabelInterval: '01:00',
        buttonText: 'Week',
        nowIndicator: true,
        scrollTime: '14:00:00',
        slotLabelFormat: {
          week: 'numeric',
          hour: 'numeric',
          minute: '2-digit',
          omitZeroMinute: true,
        }
      },
      timeGridDay: {
        type: 'timeGrid',
        allDaySlot: false,
        slotDuration: '00:15:00',
        dateClick: (info) => {
          let start = new Date(info.date.getTime());
          start.setMinutes(start.getMinutes() - start.getTimezoneOffset());
          let end = new Date(info.date.getTime() + 3600000);
          end.setMinutes(end.getMinutes() - end.getTimezoneOffset());

          bookingPopup(start.toISOString().slice(0, -8), end.toISOString().slice(0, -8));
        },
        slotLabelInterval: '01:00',
        buttonText: 'Day',
        nowIndicator: true,
        scrollTime: '14:00:00',
        slotLabelFormat: {
          hour: 'numeric',
          minute: '2-digit',
          omitZeroMinute: true,
        }
      },
    },
    firstDay: 1,
    locale: "dk",
  });
  calendar.render();
});


async function calendarSelect(info) {
  let start = calendar.formatIso(info.start).slice(0, -6);
  let end = calendar.formatIso(info.end).slice(0, -6);

  bookingPopup(start, end);
}

async function bookingPopup(start, end) {
  await Swal.fire({
    title: 'Select Time',
    html: `
    <label for="resources-dropdown">Select Resource:</label>
    <select id="resources-dropdown" class="resources-dropdown" multiple="multiple">
    </select>
    <br>
    <label for="start">Start Time:</label>
    <input type="datetime-local" id="start" name="start" value="${start}" required>
    <br>
    <label for="end">End Time:</label>
    <input type="datetime-local" id="end" name="end" value="${end}" required>
    `,
    showCancelButton: true,
    confirmButtonText: 'Confirm',
    confirmButtonColor: '#4BB543',
    cancelButtonText: 'Cancel',
    focusConfirm: false,
    didOpen: async function () {
      $('.resources-dropdown').select2({
        dropdownParent: $('#swal2-html-container'),
        placeholder: "Select resources",
        width: '200pt',
        data: await getResources(start, end),
      });
    }
  }).then(async (result) => {
    if (result.isConfirmed) {
      let start = rfc3339(document.getElementById("start").value);
      let end = rfc3339(document.getElementById("end").value);
      let resources = $('.resources-dropdown').select2('data').map((x) => x.id);
      await newBooking(start, end, resources);
    }
    calendar.unselect();
  });
}

async function newBooking(start, end, resources) {
  sendPostRequest("/api/book/secure/new", {
    start_time: start,
    end_time: end,
    resource_names: resources,
  }).then((response) => {
    if (response.ok) {
      Toast.fire({
        icon: "success",
        title: "Booking successful"
      });
    }
    else if (response.status === 401) {
      response.text().then((errorText) => {
        Toast.fire({
          icon: "error",
          title: "Login required",
        });
      });
    }

    else {
      response.text().then((errorText) => {
        Toast.fire({
          icon: "error",
          title: "Booking failed",
          text: errorText,
        });
      });
    }
    calendar.unselect();
    calendar.refetchEvents();
  });
}

function onSubmit(token) {
  document.getElementById("demo-form").submit();
}

function onSignIn(user) {
  if (logged_in == true) return;
  document.getElementById("name-plate").innerHTML = "Room " + user.room;
  document.getElementById("login").innerHTML = "Logout";
  document.getElementById("login").onclick = logout;
  username = user.username;
  room = user.room;
  logged_in = true;
  calendar.refetchEvents();
}

function onSignOut() {
  if (logged_in == false) return;
  document.getElementById("name-plate").innerHTML = "";
  document.getElementById("login").innerHTML = "Login";
  document.getElementById("login").onclick = showLoginForm;
  logged_in = false;
  room = -1;
  calendar.refetchEvents();
}

async function check_login() {
  let response = await fetch("/api/login");
  if (response.status === 202) {
    const data = await response.json();
    onSignIn(data.user);
  } else if (response.status === 200) { // 200 means not logged in
    onSignOut();
  }
}

async function handle_event_click(info) {
  //first check that the event is owned by the user
  let owned = (info.event.extendedProps.owner == room);
  let confirmed = false;
  if (owned && new Date(info.event.start) > new Date()) {
    Swal.fire({
      titleText: info.event.title.split(" ").slice(1).join(" "),
      html: `
        <label for="start">Start Time:</label>
        <input type="datetime-local" id="start" name="start" value="${info.event.startStr.slice(0, -6)}" required>
        <br>
        <label for="end">End Time:</label>
        <input type="datetime-local" id="end" name="end" value="${info.event.endStr.slice(0, -6)}" required>
      `,
      showCancelButton: true,
      confirmButtonText: 'Reschedule',
      confirmButtonColor: '#4BB543',
      showDenyButton: true,
      denyButtonText: confirmed ? 'Are you sure?' : 'Delete',
      denyButtonColor: 'red',
      preDeny: () => {
        // confirm deletion
        if (!confirmed) {
          confirmed = true;
          Swal.getDenyButton().textContent = "Are you sure?"
          return false;
        }
        return true;
      }
    }).then((result) => {
      if (result.isConfirmed) {

        //assert new start and end times are in the future

        if (new Date(document.getElementById('start').value) < new Date()) {
          Toast.fire('Error', 'Start time must be in the future', 'error');
          return;
        }

        if (new Date(document.getElementById('end').value) < new Date()) {
          Toast.fire('Error', 'End time must be in the future', 'error');
          return;
        }

        //Other checks will be done server-side

        const start = rfc3339(document.getElementById('start').value);
        const end = rfc3339(document.getElementById('end').value);
        reschedule(start, end, info.event.id);
      } else if (result.isDenied) {

        sendPostRequest("/api/book/secure/delete", {
          id: parseInt(info.event.id, 10),
        }).then((response) => {
          if (response.ok) {
            Toast.fire('Success', 'Booking deleted successfully', 'success');
            calendar.refetchEvents();
          } else {
            response.text().then((errorText) => {
              Toast.fire('Error', 'Booking delete failed: ' + errorText, 'error');
            });
          }
        })
      }
    });
  } else {
    Swal.fire({
      title: info.event.title,
      html: `
        <label for="start">Start Time:</label>
        <input type="datetime-local" id="start" name="start" style="cursor: default;" value="${info.event.startStr.slice(0, -6)}" required disabled>
        <br>
        <label for="end">End Time:</label>
        <input type="datetime-local" id="end" name="end" style="cursor: default;" value="${info.event.endStr.slice(0, -6)}" required disabled>
      `,
      showCancelButton: false,
      confirmButtonText: 'OK',
      confirmButtonColor: '#4BB543'
    });
  }

}

var logged_in = false;
var username = "";
var room = -1;
document.onload = check_login();
setInterval(async function () {
  await check_login();
}, 10000);

async function getResources(start, end) {
  const response = await fetch('api/book/resources');
  const resources = await response.json();
  // return a list of resource name strings

  let resourceNames = [];
  outer:
  for (const [key, value] of Object.entries(resources)) {
    //check disallowed periods

    if (value.disallowed_periods) {
      for (const [period_name, dates] of Object.entries(value.disallowed_periods)) {

        let is_in_range = (start, end, target) => {
          if (start > end) {
            if (target >= start || target <= end) {
              return true;
            }
          } else {
            if (target >= start && target <= end) {
              return true;
            }
          }
          return false;
        }

        let startmonth = parseInt(start.split("-")[1]);
        let startday = parseInt(start.split("-")[2].split("T")[0]);
        let endmonth = parseInt(end.split("-")[1]);
        let endday = parseInt(end.split("-")[2].split("T")[0]);
        if ((is_in_range(dates.start, dates.end, [startmonth, startday])) ||
          (is_in_range(dates.start, dates.end, [endmonth, endday]))) {
          continue outer;
        }

      }
    }


    resourceNames.push({ id: key, text: value.name });
  }
  return resourceNames;
}

function isoToDate(iso) {
  //2024-05-29T18:00:00
  let date = new Date();
  date.setFullYear(parseInt(iso.slice(0, 4)));
  date.setMonth(parseInt(iso.slice(5, 7)) - 1);
  date.setDate(parseInt(iso.slice(8, 10)));
  date.setHours(parseInt(iso.slice(11, 13)));
  date.setMinutes(parseInt(iso.slice(14, 16)));
  return date;
}

function rfc3339(d) {
  var d = new Date(d);
  function pad(n) {
    return n < 10 ? "0" + n : n;
  }

  function timezoneOffset(offset) {
    var sign;
    if (offset === 0) {
      return "Z";
    }
    sign = (offset > 0) ? "-" : "+";
    offset = Math.abs(offset);
    return sign + pad(Math.floor(offset / 60)) + ":" + pad(offset % 60);
  }

  return d.getFullYear() + "-" +
    pad(d.getMonth() + 1) + "-" +
    pad(d.getDate()) + "T" +
    pad(d.getHours()) + ":" +
    pad(d.getMinutes()) + ":" +
    pad(d.getSeconds()) +
    timezoneOffset(d.getTimezoneOffset());
}

$(document).ready(function () {
  $('.resources-dropdown').select2({
    dropdownParent: $('#create-booking-dialog'),
    placeholder: "Select resources",
    width: 'resolve'
  });
});
