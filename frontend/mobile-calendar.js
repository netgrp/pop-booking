/* ===== Mobile Calendar Component ===== */
/* Vanilla JS implementation of the reference React mobile calendar UI */
/* Integrates with existing backend API: /api/book/events, /api/book/resources, etc. */

(function () {
  "use strict";

  // ---- SVG Icons (inline, no dependencies) ----
  const ICONS = {
    chevronLeft: '<svg viewBox="0 0 24 24"><polyline points="15 18 9 12 15 6"/></svg>',
    chevronRight: '<svg viewBox="0 0 24 24"><polyline points="9 6 15 12 9 18"/></svg>',
    plus: '<svg viewBox="0 0 24 24"><line x1="12" y1="5" x2="12" y2="19"/><line x1="5" y1="12" x2="19" y2="12"/></svg>',
    arrowLeft: '<svg viewBox="0 0 24 24"><line x1="19" y1="12" x2="5" y2="12"/><polyline points="12 19 5 12 12 5"/></svg>',
    calendar: '<svg viewBox="0 0 24 24"><rect x="3" y="4" width="18" height="18" rx="2" ry="2"/><line x1="16" y1="2" x2="16" y2="6"/><line x1="8" y1="2" x2="8" y2="6"/><line x1="3" y1="10" x2="21" y2="10"/></svg>',
    clock: '<svg viewBox="0 0 24 24"><circle cx="12" cy="12" r="10"/><polyline points="12 6 12 12 16 14"/></svg>',
    resource: '<svg viewBox="0 0 24 24"><path d="M20 9V6a2 2 0 0 0-2-2H6a2 2 0 0 0-2 2v3"/><path d="M2 11v5a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2v-5a3 3 0 0 0-3-3H5a3 3 0 0 0-3 3Z"/><path d="M4 18v2"/><path d="M20 18v2"/><path d="M12 4v9"/></svg>',
    user: '<svg viewBox="0 0 24 24"><path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2"/><circle cx="12" cy="7" r="4"/></svg>',
    logout: '<svg viewBox="0 0 24 24"><path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4"/><polyline points="16 17 21 12 16 7"/><line x1="21" y1="12" x2="9" y2="12"/></svg>',
  };

  const MONTH_NAMES = [
    "January", "February", "March", "April", "May", "June",
    "July", "August", "September", "October", "November", "December",
  ];
  const WEEKDAYS = ["MON", "TUE", "WED", "THU", "FRI", "SAT", "SUN"];

  // ---- State ----
  let viewYear, viewMonth;
  let selectedDate = todayKey();
  let sheetOpen = false; // whether agenda overlay is expanded
  let events = []; // from backend
  let resources = {}; // from backend {key: {name, disallowed_periods, ...}}
  let resourceColors = {}; // built from events: {resourceKey: color}
  let newBookingOpen = false;
  let selectedResources = []; // for new booking form (multi-select)
  let mobileResourceSelect = null; // MultiSelect instance

  // ---- Helpers ----
  function pad(n) { return String(n).padStart(2, "0"); }

  function escapeHTML(s) {
    const div = document.createElement("div");
    div.textContent = s;
    return div.innerHTML;
  }
  function escapeAttr(s) {
    return s.replace(/&/g, "&amp;").replace(/"/g, "&quot;").replace(/'/g, "&#39;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
  }

  function todayKey() {
    const d = new Date();
    return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
  }

  function dateKey(y, m, d) {
    // Handle month overflow/underflow
    const dt = new Date(y, m, d);
    return `${dt.getFullYear()}-${pad(dt.getMonth() + 1)}-${pad(dt.getDate())}`;
  }

  function formatDateLabel(key) {
    const d = new Date(key + "T12:00:00");
    return new Intl.DateTimeFormat("en-US", {
      weekday: "long", month: "long", day: "numeric", year: "numeric"
    }).format(d);
  }

  function formatShortDate(key) {
    const d = new Date(key + "T12:00:00");
    return new Intl.DateTimeFormat("en-US", {
      weekday: "short", month: "short", day: "numeric"
    }).format(d);
  }

  function formatTimeFromDate(date) {
    return new Intl.DateTimeFormat("da-DK", {
      hour: "2-digit", minute: "2-digit", hour12: false
    }).format(date);
  }

  function buildMonth(year, month) {
    const firstDay = new Date(year, month, 1);
    const daysInMonth = new Date(year, month + 1, 0).getDate();
    const prevMonthDays = new Date(year, month, 0).getDate();
    const mondayIdx = (firstDay.getDay() + 6) % 7;
    const days = [];

    for (let i = mondayIdx - 1; i >= 0; i--) {
      days.push({ day: prevMonthDays - i, date: dateKey(year, month - 1, prevMonthDays - i), muted: true });
    }
    for (let d = 1; d <= daysInMonth; d++) {
      days.push({ day: d, date: dateKey(year, month, d), muted: false });
    }
    let next = 1;
    while (days.length < 42) {
      days.push({ day: next, date: dateKey(year, month + 1, next), muted: true });
      next++;
    }
    return days;
  }

  // Group events by date key (YYYY-MM-DD)
  // ev.start / ev.end are Date objects after loadEventsFromAPI
  function eventsByDate() {
    const map = {};
    for (const ev of events) {
      const s = ev.start instanceof Date ? ev.start : new Date(ev.start);
      const e = ev.end instanceof Date ? ev.end : new Date(ev.end);
      // Add event to every day it spans
      const d = new Date(s);
      d.setHours(0, 0, 0, 0);
      const endDay = new Date(e);
      endDay.setHours(0, 0, 0, 0);
      while (d <= endDay) {
        const key = `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
        if (!map[key]) map[key] = [];
        map[key].push(ev);
        d.setDate(d.getDate() + 1);
      }
    }
    return map;
  }

  function eventsForDate(dateStr) {
    const map = eventsByDate();
    return (map[dateStr] || []).sort((a, b) => {
      const sa = a.start instanceof Date ? a.start : new Date(a.start);
      const sb = b.start instanceof Date ? b.start : new Date(b.start);
      return sa - sb;
    });
  }

  function evStartISO(ev) {
    const d = ev.start instanceof Date ? ev.start : new Date(ev.start);
    return d.toISOString();
  }
  function evEndISO(ev) {
    const d = ev.end instanceof Date ? ev.end : new Date(ev.end);
    return d.toISOString();
  }

  // ---- DOM Construction ----
  let rootEl, sheetEl, gridEl, newBookingEl;

  function init() {
    const today = new Date();
    viewYear = today.getFullYear();
    viewMonth = today.getMonth();

    rootEl = document.createElement("div");
    rootEl.className = "mobile-cal";
    rootEl.innerHTML = buildCalendarHTML();
    document.body.appendChild(rootEl);

    // New booking screen
    newBookingEl = document.createElement("div");
    newBookingEl.className = "mc-new-booking";
    document.body.appendChild(newBookingEl);

    bindEvents();
    loadResourcesFromAPI(); // This will also trigger loadEventsFromAPI
    render();

    // Set initial grid overlay position (no animation)
    requestAnimationFrame(() => {
      const overlay = document.getElementById("mc-agenda-overlay");
      const grid = document.getElementById("mc-grid-container");
      const midTop = grid.getBoundingClientRect().bottom;
      overlay.style.transform = `translateY(${midTop}px)`;
      // Enable transitions after initial position is painted
      requestAnimationFrame(() => {
        overlay.style.transition = "transform 0.35s cubic-bezier(0.32, 0.72, 0, 1)";
      });
    });
  }

  function buildCalendarHTML() {
    return `
      <div class="mc-header">
        <span class="mc-name-badge" id="mc-name-badge"></span>
        <span class="mc-month-title" id="mc-month-title"></span>
        <div class="mc-header-right">
          <button class="mc-login-btn" id="mc-login-btn">
            ${ICONS.user} Login
          </button>
        </div>
      </div>
      <div class="mc-weekdays">
        ${WEEKDAYS.map(d => `<span>${d}</span>`).join("")}
      </div>
      <div class="mc-grid-container" id="mc-grid-container">
        <div class="mc-grid" id="mc-grid"></div>
      </div>
      <div class="mc-agenda-overlay" id="mc-agenda-overlay">
        <button class="mc-agenda-handle" id="mc-agenda-handle"></button>
        <div class="mc-agenda-header-row">
          <div>
            <div class="mc-agenda-title">Bookings</div>
            <div class="mc-agenda-date" id="mc-agenda-date"></div>
          </div>
          <button class="mc-btn-create" id="mc-btn-new">${ICONS.plus}</button>
        </div>
        <div class="mc-agenda-scroll" id="mc-agenda-scroll">
          <div class="mc-agenda" id="mc-agenda"></div>
        </div>
      </div>
    `;
  }

  function render() {
    renderHeader();
    renderGrid();
    renderAgenda();
    renderLoginState();
  }

  function renderHeader() {
    document.getElementById("mc-month-title").textContent = `${MONTH_NAMES[viewMonth]} ${viewYear}`;
  }

  function renderLoginState() {
    const badge = document.getElementById("mc-name-badge");
    const btn = document.getElementById("mc-login-btn");
    if (logged_in) {
      badge.textContent = `Room ${room}`;
      btn.innerHTML = `${ICONS.logout} Logout`;
    } else {
      badge.textContent = "";
      btn.innerHTML = `${ICONS.user} Login`;
    }
  }

  function buildGridHTML(year, month) {
    const days = buildMonth(year, month);
    const byDate = eventsByDate();
    const todayStr = todayKey();

    return days.map(day => {
      const isSelected = day.date === selectedDate;
      const isToday = day.date === todayStr;
      const dayEvents = byDate[day.date] || [];
      const dots = dayEvents.slice(0, 3);

      let cls = "mc-day";
      if (day.muted) cls += " muted";
      if (isSelected) cls += " selected";
      if (isToday) cls += " today";

      return `
        <button class="${cls}" data-date="${day.date}" data-muted="${day.muted}">
          <span class="mc-day-num">${day.day}</span>
          <span class="mc-dots">
            ${dots.map(ev => `<i class="mc-dot" style="background:${ev.color || '#2563eb'}"></i>`).join("")}
          </span>
        </button>
      `;
    }).join("");
  }

  function renderGrid() {
    // Clean up any stale swipe tracks
    finishPendingSwipe();
    const container = document.getElementById("mc-grid-container");
    container.querySelectorAll(".mc-grid-track").forEach(t => t.remove());
    container.style.height = "";
    container.style.transition = "";

    gridEl = document.getElementById("mc-grid");
    gridEl.style.display = "";
    gridEl.innerHTML = buildGridHTML(viewYear, viewMonth);
  }

  function buildAgendaHTML(forDate) {
    const dayEvents = eventsForDate(forDate);

    if (dayEvents.length === 0) {
      return `
        <div class="mc-empty">
          <div class="mc-empty-title">No bookings</div>
          <div class="mc-empty-sub">Tap + to create a booking for this day.</div>
        </div>
      `;
    }

    const viewDate = new Date(forDate + "T00:00:00");
    const viewDateEnd = new Date(forDate + "T23:59:59");

    const mapped = dayEvents.map(ev => {
      const s = ev.start instanceof Date ? ev.start : new Date(ev.start);
      const e = ev.end instanceof Date ? ev.end : new Date(ev.end);
      const effectiveStart = s < viewDate ? viewDate : s;
      const effectiveEnd = e > viewDateEnd ? viewDateEnd : e;
      const isMultiDay = s.toDateString() !== e.toDateString();
      const startedBefore = s < viewDate;
      const endsAfter = e > viewDateEnd;
      const daysBefore = startedBefore ? Math.round((viewDate - new Date(s.getFullYear(), s.getMonth(), s.getDate())) / (1000 * 60 * 60 * 24)) : 0;
      const daysAfter = endsAfter ? Math.round((new Date(e.getFullYear(), e.getMonth(), e.getDate()) - viewDate) / (1000 * 60 * 60 * 24)) : 0;
      return { ev, s, e, effectiveStart, effectiveEnd, isMultiDay, startedBefore, endsAfter, daysBefore, daysAfter };
    });

    const continuationEvents = mapped.filter(m => m.startedBefore);
    const todayEvents = mapped.filter(m => !m.startedBefore);

    let html = '';

    for (const { ev, s, e, isMultiDay, daysBefore, daysAfter } of continuationEvents) {
      const startISO = s.toISOString();
      const endISO = e.toISOString();
      const parts = ev.title.split(", ");
      const displayTitle = parts.length > 1 ? parts.slice(1).join(", ") : ev.title;
      const roomLabel = parts.length > 1 ? parts[0] : "";
      const owned = ev.owner == room;
      const startIndicator = `−${daysBefore}d`;
      const endIndicator = daysAfter > 0 ? `+${daysAfter}d` : '';

      html += `
        <div class="mc-booking-card${owned ? " owned" : ""}" style="border-left-color:${ev.color || '#2563eb'}; margin: 2px 12px 4px 18px;" data-event-id="${ev.id}" data-owned="${owned}" data-start="${startISO}" data-end="${endISO}" data-title="${escapeAttr(ev.title)}">
          <div class="mc-booking-info">
            <div class="mc-booking-title">${owned ? "You, " : ""}${escapeHTML(displayTitle)}</div>
            <div class="mc-booking-resource">${escapeHTML(roomLabel)}</div>
          </div>
          <div class="mc-booking-time mc-booking-time-multi">
            <span class="mc-time-part">${formatTimeFromDate(s)}<span class="mc-booking-day-indicator">${startIndicator}</span></span>
            <span class="mc-time-sep">–</span>
            <span class="mc-time-part">${formatTimeFromDate(e)}${endIndicator ? `<span class="mc-booking-day-indicator">${endIndicator}</span>` : ''}</span>
          </div>
        </div>
      `;
    }

    let minHour = 23, maxHour = 0;
    for (const { effectiveStart, effectiveEnd } of todayEvents) {
      minHour = Math.min(minHour, effectiveStart.getHours());
      const endH = effectiveEnd.getHours() + (effectiveEnd.getMinutes() > 0 ? 1 : 0);
      maxHour = Math.max(maxHour, endH);
    }
    if (todayEvents.length > 0) {
      minHour = Math.max(0, minHour - 1);
      maxHour = Math.min(24, maxHour + 1);
    }

    for (let h = minHour; h < maxHour; h++) {
      const hourLabel = formatTimeFromDate(new Date(1970, 0, 1, h, 0));
      const hourEvents = todayEvents.filter(m => m.effectiveStart.getHours() === h);

      html += `<div class="mc-agenda-hour">`;
      html += `<div class="mc-agenda-tick"><span class="mc-agenda-tick-label">${hourLabel}</span><span class="mc-agenda-tick-line"></span></div>`;

      for (const { ev, s, e, isMultiDay, daysAfter } of hourEvents) {
        const startISO = s.toISOString();
        const endISO = e.toISOString();
        const parts = ev.title.split(", ");
        const displayTitle = parts.length > 1 ? parts.slice(1).join(", ") : ev.title;
        const roomLabel = parts.length > 1 ? parts[0] : "";
        const owned = ev.owner == room;
        const timeStr = `${formatTimeFromDate(s)} – ${formatTimeFromDate(e)}`;
        const endIndicator = daysAfter > 0 ? `+${daysAfter}d` : '';

        html += `
          <div class="mc-booking-card${owned ? " owned" : ""}" style="border-left-color:${ev.color || '#2563eb'}" data-event-id="${ev.id}" data-owned="${owned}" data-start="${startISO}" data-end="${endISO}" data-title="${escapeAttr(ev.title)}">
            <div class="mc-booking-info">
              <div class="mc-booking-title">${owned ? "You, " : ""}${escapeHTML(displayTitle)}</div>
              <div class="mc-booking-resource">${escapeHTML(roomLabel)}</div>
            </div>
            <div class="mc-booking-time">${timeStr}${endIndicator ? `<div class="mc-booking-day-indicator">${endIndicator}</div>` : ''}</div>
          </div>
        `;
      }

      html += `</div>`;
    }

    return html;
  }

  function renderAgenda() {
    const dateEl = document.getElementById("mc-agenda-date");
    const agendaEl = document.getElementById("mc-agenda");
    dateEl.textContent = formatDateLabel(selectedDate);
    agendaEl.innerHTML = buildAgendaHTML(selectedDate);
  }

  // ---- New Booking Screen ----
  function renderNewBooking() {
    // Build default start/end as local datetime-local values
    const defaultStart = selectedDate + "T" + getDefaultStartTime();
    const defaultEnd = selectedDate + "T" + getDefaultEndTime();

    // Filter resources by availability (disallowed periods)
    const availableResources = getAvailableResources();

    newBookingEl.innerHTML = `
      <div class="mc-nb-header">
        <button class="mc-btn-icon" id="mc-nb-back">${ICONS.arrowLeft}</button>
        <div class="mc-nb-title">New Booking</div>
        <div></div>
      </div>
      <div class="mc-nb-body">
        <div class="mc-nb-section" style="margin-top:0">
          <label class="mc-nb-label">Resources</label>
          <div id="mc-nb-resources"></div>
        </div>

        <div class="mc-nb-section">
          <label class="mc-nb-label" for="mc-nb-start">Start</label>
          <input type="datetime-local" class="mc-nb-datetime" id="mc-nb-start" value="${defaultStart}" step="300">
        </div>

        <div class="mc-nb-section">
          <label class="mc-nb-label" for="mc-nb-end">End</label>
          <input type="datetime-local" class="mc-nb-datetime" id="mc-nb-end" value="${defaultEnd}" step="300">
        </div>
      </div>

      <div class="mc-nb-footer">
        <div class="mc-nb-summary" id="mc-nb-summary">${buildSummary()}</div>
        <button class="mc-nb-submit" id="mc-nb-submit" ${selectedResources.length === 0 ? "disabled" : ""}>Create Booking</button>
      </div>
    `;

    bindNewBookingEvents();

    // Initialize MultiSelect dropdown for resources
    var resContainer = document.getElementById("mc-nb-resources");
    var resOptions = availableResources.map(function ([key, res]) {
      return { id: key, text: res.name, color: resourceColors[key] || '#2563eb' };
    });
    mobileResourceSelect = new MultiSelect(resContainer, {
      placeholder: "Select resources",
      options: resOptions,
      onChange: function (selected) {
        selectedResources = selected.map(function (s) { return s.id; });
        updateSummaryAndButton();
      }
    });
  }

  function getDefaultStartTime() {
    const now = new Date();
    now.setMinutes(Math.ceil(now.getMinutes() / 15) * 15, 0, 0);
    return `${pad(now.getHours())}:${pad(now.getMinutes())}`;
  }

  function getDefaultEndTime() {
    const now = new Date();
    now.setMinutes(Math.ceil(now.getMinutes() / 15) * 15, 0, 0);
    const end = new Date(now.getTime() + 60 * 60000); // default 1 hour
    return `${pad(end.getHours())}:${pad(end.getMinutes())}`;
  }

  function getAvailableResources() {
    const month = parseInt(selectedDate.split("-")[1], 10);
    const day = parseInt(selectedDate.split("-")[2], 10);

    return Object.entries(resources).filter(([key, res]) => {
      if (!res.disallowed_periods) return true;
      for (const period of res.disallowed_periods) {
        if (isInRange(period.start, period.end, [month, day])) return false;
      }
      return true;
    });
  }

  function isInRange(start, end, target) {
    // start/end are [month, day], target is [month, day]
    const s = start[0] * 100 + start[1];
    const e = end[0] * 100 + end[1];
    const t = target[0] * 100 + target[1];
    if (s > e) {
      // wraps around year (e.g. Nov-Mar)
      return t >= s || t <= e;
    }
    return t >= s && t <= e;
  }

  function buildSummary() {
    const startEl = document.getElementById("mc-nb-start");
    const endEl = document.getElementById("mc-nb-end");
    const startVal = startEl ? startEl.value : (selectedDate + "T" + getDefaultStartTime());
    const endVal = endEl ? endEl.value : (selectedDate + "T" + getDefaultEndTime());

    const startDate = new Date(startVal);
    const endDate = new Date(endVal);

    const resNames = selectedResources.map(k => resources[k]?.name || k).join(", ");
    const dateStr = !isNaN(startDate) ? formatShortDate(startVal.slice(0, 10)) : "";
    const startStr = !isNaN(startDate) ? formatTimeFromDate(startDate) : "";
    const endStr = !isNaN(endDate) ? formatTimeFromDate(endDate) : "";
    return `${dateStr} · ${startStr} – ${endStr}${resNames ? " · " + resNames : ""}`;
  }

  function updateSummaryAndButton() {
    const summary = document.getElementById("mc-nb-summary");
    const submit = document.getElementById("mc-nb-submit");
    if (summary) summary.textContent = buildSummary();
    if (submit) submit.disabled = selectedResources.length === 0;
  }

  // ---- Event Binding ----
  function bindEvents() {
    // Month navigation
    // Day clicks (delegated)
    document.getElementById("mc-grid").addEventListener("click", (e) => {
      const btn = e.target.closest(".mc-day");
      if (!btn) return;
      finishPendingSwipe();
      const date = btn.dataset.date;
      const muted = btn.dataset.muted === "true";
      selectedDate = date;
      if (muted) {
        const d = new Date(date + "T12:00:00");
        viewYear = d.getFullYear();
        viewMonth = d.getMonth();
      }
      render();
      updateAgendaOverlayState();
    });

    // Agenda handle toggle
    document.getElementById("mc-agenda-handle").addEventListener("click", () => {
      sheetOpen = !sheetOpen;
      updateAgendaOverlayState();
    });

    // Agenda overlay drag
    setupAgendaOverlayDrag();

    // Agenda horizontal swipe for day navigation
    setupAgendaSwipe();

    // New booking button
    document.getElementById("mc-btn-new").addEventListener("click", openNewBooking);

    // Login button
    document.getElementById("mc-login-btn").addEventListener("click", () => {
      if (logged_in) {
        logout();
        setTimeout(() => renderLoginState(), 500);
      } else {
        showLoginForm().then(() => {
          renderLoginState();
          loadEventsFromAPI();
        });
      }
    });

    // Booking card clicks (delegated from agenda)
    document.getElementById("mc-agenda").addEventListener("click", (e) => {
      const card = e.target.closest(".mc-booking-card");
      if (!card) return;
      handleBookingClick(card);
    });

    // Smooth swipe left/right on grid for month nav
    setupGridSwipe();
  }

  let finishPendingSwipe = () => {};

  function setupGridSwipe() {
    const container = document.getElementById("mc-grid-container");
    let startX = 0;
    let currentX = 0;
    let dragging = false;
    let velocity = 0;
    let lastX = 0;
    let lastTime = 0;
    let track = null;
    let containerW = 0;

    function getAdjacentMonth(dir) {
      let m = viewMonth + dir;
      let y = viewYear;
      if (m > 11) { m = 0; y++; }
      if (m < 0) { m = 11; y--; }
      return { year: y, month: m };
    }

    function createTrack() {
      containerW = container.offsetWidth;
      const prev = getAdjacentMonth(-1);
      const next = getAdjacentMonth(1);

      // Lock the container height to prevent size jumps during swipe
      container.style.height = container.offsetHeight + "px";

      track = document.createElement("div");
      track.className = "mc-grid-track";
      track.style.width = (containerW * 3) + "px";
      track.style.transform = `translateX(${-containerW}px)`;

      const prevGrid = document.createElement("div");
      prevGrid.className = "mc-grid";
      prevGrid.style.width = containerW + "px";
      prevGrid.innerHTML = buildGridHTML(prev.year, prev.month);

      const curGrid = document.createElement("div");
      curGrid.className = "mc-grid";
      curGrid.style.width = containerW + "px";
      curGrid.innerHTML = buildGridHTML(viewYear, viewMonth);

      const nextGrid = document.createElement("div");
      nextGrid.className = "mc-grid";
      nextGrid.style.width = containerW + "px";
      nextGrid.innerHTML = buildGridHTML(next.year, next.month);

      track.appendChild(prevGrid);
      track.appendChild(curGrid);
      track.appendChild(nextGrid);

      // Hide the original grid, show track
      const origGrid = document.getElementById("mc-grid");
      origGrid.style.display = "none";
      container.appendChild(track);
    }

    function removeTrack() {
      if (track && track.parentNode) {
        track.parentNode.removeChild(track);
        track = null;
      }
      const origGrid = document.getElementById("mc-grid");
      origGrid.style.display = "";
    }

    let animating = false;
    let pendingOnEnd = null;

    finishPendingSwipe = function() {
      if (animating && pendingOnEnd) {
        pendingOnEnd();
        pendingOnEnd = null;
        animating = false;
      }
    };

    container.addEventListener("touchstart", (e) => {
      finishPendingSwipe();

      startX = e.touches[0].clientX;
      lastX = startX;
      currentX = startX;
      lastTime = Date.now();
      velocity = 0;
      dragging = true;
      track = null;
    }, { passive: true });

    container.addEventListener("touchmove", (e) => {
      if (!dragging) return;
      currentX = e.touches[0].clientX;
      const now = Date.now();
      const dt = now - lastTime;
      if (dt > 0) velocity = (currentX - lastX) / dt;
      lastX = currentX;
      lastTime = now;

      // Create track on first real movement
      if (!track && Math.abs(currentX - startX) > 5) {
        createTrack();
        track.style.transition = "none";
      }

      if (track) {
        const dx = currentX - startX;
        track.style.transform = `translateX(${-containerW + dx}px)`;
      }
    }, { passive: true });

    container.addEventListener("touchend", () => {
      if (!dragging) return;
      dragging = false;
      if (!track) return;
      const dx = currentX - startX;
      const flingThreshold = 0.3;

      let swipeDir = 0;
      if (velocity > flingThreshold || dx > containerW * 0.25) swipeDir = -1;
      else if (velocity < -flingThreshold || dx < -containerW * 0.25) swipeDir = 1;

      // Target position for the track
      const targetX = -containerW + (-swipeDir * containerW);

      track.style.transition = "transform 0.35s cubic-bezier(0.32, 0.72, 0, 1)";
      track.style.transform = `translateX(${targetX}px)`;

      animating = true;
      const onEnd = () => {
        track.removeEventListener("transitionend", onEnd);
        animating = false;
        pendingOnEnd = null;

        if (swipeDir !== 0) {
          viewMonth += swipeDir;
          if (viewMonth > 11) { viewMonth = 0; viewYear++; }
          if (viewMonth < 0) { viewMonth = 11; viewYear--; }
          renderHeader();
          renderGrid();
          renderAgenda();
        }

        removeTrack();

        // Calculate new natural height and animate container to it
        const oldHeight = parseInt(container.style.height, 10);
        const newHeight = container.scrollHeight;
        if (oldHeight !== newHeight) {
          container.style.transition = "height 0.3s cubic-bezier(0.32, 0.72, 0, 1)";
          container.style.height = newHeight + "px";
          const onHeightEnd = () => {
            container.removeEventListener("transitionend", onHeightEnd);
            container.style.transition = "";
            container.style.height = "";
          };
          container.addEventListener("transitionend", onHeightEnd);

          // Update overlay using the target position (container top + new height)
          if (!sheetOpen) {
            const overlay = document.getElementById("mc-agenda-overlay");
            const containerTop = container.getBoundingClientRect().top;
            const targetMidTop = containerTop + newHeight;
            overlay.style.transition = "transform 0.3s cubic-bezier(0.32, 0.72, 0, 1)";
            overlay.style.transform = `translateY(${targetMidTop}px)`;
          }
        } else {
          container.style.height = "";
          updateAgendaOverlayState();
        }
      };

      pendingOnEnd = onEnd;
      track.addEventListener("transitionend", onEnd);
      // Fallback in case transitionend doesn't fire
      setTimeout(() => { if (pendingOnEnd === onEnd) onEnd(); }, 400);
    });
  }

  function setupAgendaOverlayDrag() {
    const overlay = document.getElementById("mc-agenda-overlay");
    let startY = 0;
    let startTop = 0;
    let dragging = false;
    let velocity = 0;
    let lastY = 0;
    let lastTime = 0;

    // Two snap positions:
    // fullTop: covers calendar, just below app header
    // midTop: just below the calendar grid (lowest it can go)
    const headerHeight = 56;
    const getFullTop = () => headerHeight;
    const getMidTop = () => {
      const grid = document.getElementById("mc-grid-container");
      const rect = grid.getBoundingClientRect();
      return rect.bottom;
    };

    function getOverlayTop() {
      const transform = overlay.style.transform;
      if (transform) {
        const match = transform.match(/translateY\((.+)px\)/);
        if (match) return parseFloat(match[1]);
      }
      return sheetOpen ? getFullTop() : getMidTop();
    }

    function setOverlayTop(y) {
      const clamped = Math.max(getFullTop(), Math.min(getMidTop(), y));
      overlay.style.transform = `translateY(${clamped}px)`;
    }

    overlay.addEventListener("touchstart", (e) => {
      // Allow scrolling inside the agenda scroll area when content overflows
      const scrollEl = document.getElementById("mc-agenda-scroll");
      if (e.target.closest(".mc-agenda-scroll") && scrollEl.scrollTop > 0) return;
      overlay.style.transition = "none";
      startY = e.touches[0].clientY;
      startTop = getOverlayTop();
      lastY = startY;
      lastTime = Date.now();
      velocity = 0;
      dragging = true;
    }, { passive: true });

    overlay.addEventListener("touchmove", (e) => {
      if (!dragging) return;
      const y = e.touches[0].clientY;
      const now = Date.now();
      const dt = now - lastTime;
      if (dt > 0) velocity = (y - lastY) / dt;
      lastY = y;
      lastTime = now;
      const newTop = startTop + (y - startY);
      setOverlayTop(newTop);
    }, { passive: true });

    overlay.addEventListener("touchend", () => {
      if (!dragging) return;
      dragging = false;
      overlay.style.transition = "transform 0.35s cubic-bezier(0.32, 0.72, 0, 1)";

      const currentTop = getOverlayTop();
      const fullTop = getFullTop();
      const midTop = getMidTop();
      const midPoint = (fullTop + midTop) / 2;

      const flingThreshold = 0.3;
      let snapTo;

      if (velocity < -flingThreshold) {
        snapTo = fullTop;
      } else if (velocity > flingThreshold) {
        snapTo = midTop;
      } else {
        snapTo = currentTop < midPoint ? fullTop : midTop;
      }

      setOverlayTop(snapTo);
      sheetOpen = snapTo <= fullTop;
    });
  }

  function changeDay(offset) {
    const d = new Date(selectedDate + "T12:00:00");
    d.setDate(d.getDate() + offset);
    selectedDate = `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
    // If the new date is in a different month, update the calendar view
    if (d.getMonth() !== viewMonth || d.getFullYear() !== viewYear) {
      viewMonth = d.getMonth();
      viewYear = d.getFullYear();
      renderHeader();
      renderGrid();
    } else {
      renderGrid(); // update selected highlight
    }
    renderAgenda();
  }

  function setupAgendaSwipe() {
    const scrollEl = document.getElementById("mc-agenda-scroll");
    let startX = 0;
    let startY = 0;
    let tracking = false;
    let direction = null; // 'horizontal' or 'vertical'
    let velocity = 0;
    let lastX = 0;
    let lastTime = 0;
    let track = null;
    let containerW = 0;
    let agendaAnimating = false;
    let agendaPendingOnEnd = null;

    function getAdjacentDate(offset) {
      const d = new Date(selectedDate + "T12:00:00");
      d.setDate(d.getDate() + offset);
      return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
    }

    function createAgendaTrack() {
      containerW = scrollEl.offsetWidth;
      const prevDate = getAdjacentDate(-1);
      const nextDate = getAdjacentDate(1);

      track = document.createElement("div");
      track.className = "mc-agenda-track";
      track.style.width = (containerW * 3) + "px";
      track.style.transform = `translateX(${-containerW}px)`;

      const prevPanel = document.createElement("div");
      prevPanel.className = "mc-agenda-panel";
      prevPanel.style.width = containerW + "px";
      prevPanel.innerHTML = buildAgendaHTML(prevDate);

      const curPanel = document.createElement("div");
      curPanel.className = "mc-agenda-panel";
      curPanel.style.width = containerW + "px";
      curPanel.innerHTML = buildAgendaHTML(selectedDate);

      const nextPanel = document.createElement("div");
      nextPanel.className = "mc-agenda-panel";
      nextPanel.style.width = containerW + "px";
      nextPanel.innerHTML = buildAgendaHTML(nextDate);

      track.appendChild(prevPanel);
      track.appendChild(curPanel);
      track.appendChild(nextPanel);

      // Hide original agenda, show track
      const origAgenda = document.getElementById("mc-agenda");
      origAgenda.style.display = "none";
      scrollEl.appendChild(track);
    }

    function removeAgendaTrack() {
      if (track && track.parentNode) {
        track.parentNode.removeChild(track);
        track = null;
      }
      const origAgenda = document.getElementById("mc-agenda");
      origAgenda.style.display = "";
    }

    function finishPendingAgendaSwipe() {
      if (agendaAnimating && agendaPendingOnEnd) {
        agendaPendingOnEnd();
        agendaPendingOnEnd = null;
        agendaAnimating = false;
      }
    }

    scrollEl.addEventListener("touchstart", (e) => {
      finishPendingAgendaSwipe();
      startX = e.touches[0].clientX;
      startY = e.touches[0].clientY;
      lastX = startX;
      lastTime = Date.now();
      velocity = 0;
      tracking = true;
      direction = null;
      track = null;
    }, { passive: true });

    scrollEl.addEventListener("touchmove", (e) => {
      if (!tracking) return;
      const x = e.touches[0].clientX;
      const y = e.touches[0].clientY;
      const dx = x - startX;
      const dy = y - startY;

      // Lock direction on first significant movement
      if (!direction) {
        if (Math.abs(dx) > 10 || Math.abs(dy) > 10) {
          direction = Math.abs(dx) > Math.abs(dy) ? 'horizontal' : 'vertical';
        }
      }

      if (direction === 'horizontal') {
        const now = Date.now();
        const dt = now - lastTime;
        if (dt > 0) velocity = (x - lastX) / dt;
        lastX = x;
        lastTime = now;

        // Create track on first horizontal movement
        if (!track) {
          createAgendaTrack();
          track.style.transition = "none";
        }

        if (track) {
          track.style.transform = `translateX(${-containerW + dx}px)`;
        }
      }
    }, { passive: true });

    scrollEl.addEventListener("touchend", () => {
      if (!tracking) return;
      tracking = false;
      if (direction !== 'horizontal' || !track) return;

      const dx = lastX - startX;
      const flingThreshold = 0.3;

      let swipeDir = 0;
      if (velocity > flingThreshold || dx > containerW * 0.25) swipeDir = -1; // prev day
      else if (velocity < -flingThreshold || dx < -containerW * 0.25) swipeDir = 1; // next day

      const targetX = -containerW + (-swipeDir * containerW);

      track.style.transition = "transform 0.35s cubic-bezier(0.32, 0.72, 0, 1)";
      track.style.transform = `translateX(${targetX}px)`;

      agendaAnimating = true;
      const onEnd = () => {
        track.removeEventListener("transitionend", onEnd);
        agendaAnimating = false;
        agendaPendingOnEnd = null;

        if (swipeDir !== 0) {
          changeDay(swipeDir);
        }

        removeAgendaTrack();
      };

      agendaPendingOnEnd = onEnd;
      track.addEventListener("transitionend", onEnd);
      setTimeout(() => { if (agendaPendingOnEnd === onEnd) onEnd(); }, 400);
    });
  }

  function updateAgendaOverlayState() {
    const overlay = document.getElementById("mc-agenda-overlay");
    const grid = document.getElementById("mc-grid-container");
    const rect = grid.getBoundingClientRect();
    const midTop = rect.bottom;

    overlay.style.transition = "transform 0.35s cubic-bezier(0.32, 0.72, 0, 1)";
    overlay.style.transform = `translateY(${sheetOpen ? 56 : midTop}px)`;
  }

  function handleBookingClick(card) {
    const eventId = parseInt(card.dataset.eventId, 10);
    const owned = card.dataset.owned === "true";
    const title = card.dataset.title;
    const startStr = card.dataset.start;
    const endStr = card.dataset.end;

    // Convert ISO (UTC) to local datetime-local format
    function toLocalDTL(isoStr) {
      const d = new Date(isoStr);
      d.setMinutes(d.getMinutes() - d.getTimezoneOffset());
      return d.toISOString().slice(0, 16);
    }
    const startLocal = toLocalDTL(startStr);
    const endLocal = toLocalDTL(endStr);

    if (owned && new Date(startStr) > new Date()) {
      // Owner can reschedule/delete
      let confirmed = false;
      Swal.fire({
        titleText: title.split(", ").slice(1).join(", ") || title,
        html: `
          <label for="start">Start Time:</label>
          <input type="datetime-local" id="start" name="start" value="${startLocal}" required>
          <br>
          <label for="end">End Time:</label>
          <input type="datetime-local" id="end" name="end" value="${endLocal}" required>
        `,
        showCancelButton: true,
        confirmButtonText: "Reschedule",
        confirmButtonColor: "#2563eb",
        showDenyButton: true,
        denyButtonText: "Delete",
        denyButtonColor: "#ef4444",
        preDeny: () => {
          if (!confirmed) {
            confirmed = true;
            Swal.getDenyButton().textContent = "Are you sure?";
            return false;
          }
          return true;
        }
      }).then((result) => {
        if (result.isConfirmed) {
          const start = rfc3339(document.getElementById("start").value);
          const end = rfc3339(document.getElementById("end").value);
          reschedule(start, end, eventId);
          setTimeout(() => loadEventsFromAPI(), 500);
        } else if (result.isDenied) {
          sendPostRequest("/api/book/secure/delete", { id: eventId }).then((response) => {
            if (response.ok) {
              Toast.fire({ icon: "success", title: "Booking deleted" });
            } else {
              response.text().then(t => Toast.fire({ icon: "error", title: "Delete failed", text: t }));
            }
            loadEventsFromAPI();
          });
        }
      });
    } else {
      // View only
      Swal.fire({
        title: title,
        html: `
          <label for="start">Start Time:</label>
          <input type="datetime-local" id="start" style="cursor:default" value="${startLocal}" disabled>
          <br>
          <label for="end">End Time:</label>
          <input type="datetime-local" id="end" style="cursor:default" value="${endLocal}" disabled>
        `,
        showCancelButton: false,
        confirmButtonText: "OK",
        confirmButtonColor: "#2563eb"
      });
    }
  }

  // ---- New Booking Binding ----
  function bindNewBookingEvents() {
    document.getElementById("mc-nb-back").addEventListener("click", closeNewBooking);

    // Time change -> update summary
    const startEl = document.getElementById("mc-nb-start");
    const endEl = document.getElementById("mc-nb-end");
    startEl.addEventListener("change", () => {
      updateSummaryAndButton();
    });
    endEl.addEventListener("change", () => {
      updateSummaryAndButton();
    });

    // Submit
    document.getElementById("mc-nb-submit").addEventListener("click", submitNewBooking);
  }

  function openNewBooking() {
    if (!logged_in) {
      showLoginForm().then((result) => {
        renderLoginState();
        if (logged_in) {
          loadEventsFromAPI();
          openNewBooking();
        }
      });
      return;
    }
    selectedResources = [];
    newBookingOpen = true;
    newBookingEl.classList.add("active");
    renderNewBooking();
  }

  function closeNewBooking() {
    newBookingOpen = false;
    newBookingEl.classList.remove("active");
    if (mobileResourceSelect) {
      mobileResourceSelect.destroy();
      mobileResourceSelect = null;
    }
  }

  function submitNewBooking() {
    const startVal = document.getElementById("mc-nb-start").value;
    const endVal = document.getElementById("mc-nb-end").value;

    if (selectedResources.length === 0) {
      Toast.fire({ icon: "warning", title: "Select at least one resource" });
      return;
    }

    if (!startVal || !endVal) {
      Toast.fire({ icon: "warning", title: "Select start and end times" });
      return;
    }

    const startISO = rfc3339(startVal);
    const endISO = rfc3339(endVal);

    sendPostRequest("/api/book/secure/new", {
      start_time: startISO,
      end_time: endISO,
      resource_names: selectedResources,
    }).then((response) => {
      if (response.ok) {
        Toast.fire({ icon: "success", title: "Booking created" });
        closeNewBooking();
        loadEventsFromAPI();
      } else if (response.status === 401) {
        Toast.fire({ icon: "error", title: "Login required" });
      } else {
        response.text().then(t => Toast.fire({ icon: "error", title: "Booking failed", text: t }));
      }
    });
  }

  // ---- API ----
  function loadEventsFromAPI() {
    fetch("/api/book/events")
      .then(r => r.json())
      .then(data => {
        events = data.map(ev => {
          ev.start = new Date(ev.start);
          ev.end = new Date(ev.end);
          return ev;
        });
        // Build resource color map from events
        // Event titles are "Room X, ResourceName" — match against resource names
        for (const ev of events) {
          if (!ev.color) continue;
          const parts = ev.title.split(", ");
          const resName = parts.length > 1 ? parts.slice(1).join(", ") : null;
          if (resName) {
            // Find resource key by name
            for (const [key, res] of Object.entries(resources)) {
              if (res.name === resName && !resourceColors[key]) {
                resourceColors[key] = ev.color;
              }
            }
          }
        }
        render();
      })
      .catch(() => {});
  }

  function loadResourcesFromAPI() {
    fetch("/api/book/resources")
      .then(r => r.json())
      .then(data => {
        resources = data;
        // Load events after resources so color mapping works
        loadEventsFromAPI();
      })
      .catch(() => {});
  }

  // ---- Refresh on visibility ----
  document.addEventListener("visibilitychange", () => {
    if (!document.hidden && document.body.classList.contains("is-mobile")) {
      loadEventsFromAPI();
    }
  });

  // ---- Periodic refresh ----
  setInterval(() => {
    if (document.body.classList.contains("is-mobile")) {
      loadEventsFromAPI();
    }
  }, 30000);

  // ---- Initialization ----
  window.initMobileCalendar = function () {
    document.body.classList.add("is-mobile");
    init();
  };

})();
