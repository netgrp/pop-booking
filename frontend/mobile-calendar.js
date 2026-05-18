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
  let eventsVersion = 0;
  let eventsByDateCache = null;
  const monthCache = new Map();
  const gridHtmlCache = new Map();
  const agendaHtmlCache = new Map();
  let newBookingOpen = false;
  let selectedResources = []; // for new booking form (multi-select)
  let mobileResourceSelect = null; // MultiSelect instance

  // ---- Helpers ----
  function pad(n) { return String(n).padStart(2, "0"); }

  function translateX(el, x) {
    el.style.transform = `translate3d(${x}px, 0, 0)`;
  }

  function translateY(el, y) {
    el.style.transform = `translate3d(0, ${y}px, 0)`;
  }

  function positionAgendaOverlay(overlay, top) {
    const viewportHeight = window.visualViewport?.height || window.innerHeight;
    overlay.style.height = Math.max(120, viewportHeight - 56) + "px";
    translateY(overlay, top);
  }

  function makeTransformScheduler(apply) {
    let frame = 0;
    let latest = 0;

    function schedule(value) {
      latest = value;
      if (frame) return;
      frame = requestAnimationFrame(() => {
        frame = 0;
        apply(latest);
      });
    }

    schedule.cancel = function() {
      if (!frame) return;
      cancelAnimationFrame(frame);
      frame = 0;
    };

    return schedule;
  }

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
    const cacheKey = `${year}-${month}`;
    if (monthCache.has(cacheKey)) return monthCache.get(cacheKey);

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
    monthCache.set(cacheKey, days);
    return days;
  }

  // Group events by date key (YYYY-MM-DD)
  // ev.start / ev.end are Date objects after loadEventsFromAPI
  function eventsByDate() {
    if (eventsByDateCache) return eventsByDateCache;

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
    eventsByDateCache = map;
    return eventsByDateCache;
  }

  function eventsForDate(dateStr) {
    const map = eventsByDate();
    return (map[dateStr] || []).slice().sort((a, b) => {
      const sa = a.start instanceof Date ? a.start : new Date(a.start);
      const sb = b.start instanceof Date ? b.start : new Date(b.start);
      return sa - sb;
    });
  }

  function invalidateEventRenderCaches() {
    eventsVersion++;
    eventsByDateCache = null;
    gridHtmlCache.clear();
    agendaHtmlCache.clear();
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
  // Cached DOM references (set once in init/bindEvents)
  let _gridContainer, _gridTrack, _gridPrev, _gridCur, _gridNext;
  let _agendaOverlay, _agendaScroll, _agendaEl, _agendaDate;
  let _monthTitle, _nameBadge, _loginBtn;
  let _adjacentPanelIdle = 0; // requestIdleCallback / setTimeout handle

  // ---- URL Routing (Mobile) ----
  // Parameters: ?date=YYYY-MM-DD  &newbooking=1

  let _mobileUrlSuppressPush = false;

  function pushMobileUrlState() {
    if (_mobileUrlSuppressPush) return;
    const params = new URLSearchParams();
    if (selectedDate && selectedDate !== todayKey()) {
      params.set("date", selectedDate);
    }
    if (newBookingOpen) {
      params.set("newbooking", "1");
    }
    const qs = params.toString();
    const url = qs ? "?" + qs : location.pathname;
    const currentQs = location.search.replace(/^\?/, "");
    if (currentQs !== qs) {
      history.pushState(null, "", url);
    }
  }

  function restoreMobileUrlState() {
    _mobileUrlSuppressPush = true;
    try {
      const params = new URLSearchParams(location.search);
      const date = params.get("date");
      if (date && /^\d{4}-\d{2}-\d{2}$/.test(date)) {
        selectedDate = date;
        const d = new Date(date + "T12:00:00");
        viewYear = d.getFullYear();
        viewMonth = d.getMonth();
      } else {
        selectedDate = todayKey();
        const today = new Date();
        viewYear = today.getFullYear();
        viewMonth = today.getMonth();
      }

      // Handle new booking screen
      const nb = params.get("newbooking");
      if (nb === "1" && !newBookingOpen) {
        openNewBooking();
      } else if (nb !== "1" && newBookingOpen) {
        closeNewBooking();
      }

      render();
      loadEventsFromAPI(); // refetch for potentially new month range
      requestAnimationFrame(() => updateAgendaOverlayState());
    } finally {
      _mobileUrlSuppressPush = false;
    }
  }

  window.addEventListener("popstate", () => {
    if (!window.mobilecheck()) return;
    restoreMobileUrlState();
  });

  function init() {
    // Read initial date from URL
    const params = new URLSearchParams(location.search);
    const urlDate = params.get("date");
    if (urlDate && /^\d{4}-\d{2}-\d{2}$/.test(urlDate)) {
      selectedDate = urlDate;
      const d = new Date(urlDate + "T12:00:00");
      viewYear = d.getFullYear();
      viewMonth = d.getMonth();
    } else {
      const today = new Date();
      viewYear = today.getFullYear();
      viewMonth = today.getMonth();
    }

    rootEl = document.createElement("div");
    rootEl.className = "mobile-cal";
    rootEl.innerHTML = buildCalendarHTML();
    document.body.appendChild(rootEl);

    // Cache DOM references
    _gridContainer = document.getElementById("mc-grid-container");
    _gridTrack = document.getElementById("mc-grid-track");
    _gridPrev = document.getElementById("mc-grid-prev");
    _gridCur = document.getElementById("mc-grid");
    _gridNext = document.getElementById("mc-grid-next");
    _agendaOverlay = document.getElementById("mc-agenda-overlay");
    _agendaScroll = document.getElementById("mc-agenda-scroll");
    _agendaEl = document.getElementById("mc-agenda");
    _agendaDate = document.getElementById("mc-agenda-date");
    _monthTitle = document.getElementById("mc-month-title");
    _nameBadge = document.getElementById("mc-name-badge");
    _loginBtn = document.getElementById("mc-login-btn");

    // New booking screen
    newBookingEl = document.createElement("div");
    newBookingEl.className = "mc-new-booking";
    document.body.appendChild(newBookingEl);

    bindEvents();
    loadResourcesFromAPI(); // This will also trigger loadEventsFromAPI
    render();

    // Open new booking screen if URL says so
    if (params.get("newbooking") === "1") {
      requestAnimationFrame(() => openNewBooking());
    }

    // Replace initial history entry
    history.replaceState(null, "", location.search || location.pathname);

    // Set initial grid overlay position (no animation)
    requestAnimationFrame(() => {
      const midTop = _gridContainer.getBoundingClientRect().bottom;
      positionAgendaOverlay(_agendaOverlay, midTop);
      // Enable transitions after initial position is painted
      requestAnimationFrame(() => {
        _agendaOverlay.style.transition = "transform 0.35s cubic-bezier(0.32, 0.72, 0, 1)";
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
        <div class="mc-grid-track" id="mc-grid-track">
          <div class="mc-grid" id="mc-grid-prev"></div>
          <div class="mc-grid" id="mc-grid"></div>
          <div class="mc-grid" id="mc-grid-next"></div>
        </div>
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
    _monthTitle.textContent = `${MONTH_NAMES[viewMonth]} ${viewYear}`;
  }

  function renderLoginState() {
    if (logged_in) {
      _nameBadge.textContent = `Room ${room}`;
      _loginBtn.innerHTML = `${ICONS.logout} Logout`;
    } else {
      _nameBadge.textContent = "";
      _loginBtn.innerHTML = `${ICONS.user} Login`;
    }
  }

  function buildGridHTML(year, month) {
    const cacheKey = `${eventsVersion}|${room}|${selectedDate}|${year}|${month}`;
    if (gridHtmlCache.has(cacheKey)) return gridHtmlCache.get(cacheKey);

    const days = buildMonth(year, month);
    const byDate = eventsByDate();
    const todayStr = todayKey();
    const parts = [];

    for (let i = 0; i < days.length; i++) {
      const day = days[i];
      const isSelected = day.date === selectedDate;
      const isToday = day.date === todayStr;
      const dayEvents = byDate[day.date];

      let cls = "mc-day";
      if (day.muted) cls += " muted";
      if (isSelected) cls += " selected";
      if (isToday) cls += " today";

      parts.push('<button class="', cls, '" data-date="', day.date, '" data-muted="', day.muted ? 'true' : 'false', '"><span class="mc-day-num">', String(day.day), '</span><span class="mc-dots">');

      if (dayEvents) {
        const n = dayEvents.length < 3 ? dayEvents.length : 3;
        for (let j = 0; j < n; j++) {
          parts.push('<i class="mc-dot" style="background:', dayEvents[j].color || '#2563eb', '"></i>');
        }
      }

      parts.push('</span></button>');
    }

    const html = parts.join('');
    gridHtmlCache.set(cacheKey, html);
    return html;
  }

  function scheduleAdjacentPanels() {
    // Cancel any pending idle render
    if (_adjacentPanelIdle) {
      (typeof cancelIdleCallback === 'function' ? cancelIdleCallback : clearTimeout)(_adjacentPanelIdle);
      _adjacentPanelIdle = 0;
    }
    const prev = getAdjacentMonth(-1);
    const next = getAdjacentMonth(1);
    const doRender = () => {
      _adjacentPanelIdle = 0;
      _gridPrev.innerHTML = buildGridHTML(prev.year, prev.month);
      _gridNext.innerHTML = buildGridHTML(next.year, next.month);
    };
    if (typeof requestIdleCallback === 'function') {
      _adjacentPanelIdle = requestIdleCallback(doRender, { timeout: 200 });
    } else {
      _adjacentPanelIdle = setTimeout(doRender, 30);
    }
  }

  function flushAdjacentPanels() {
    if (!_adjacentPanelIdle) return;
    (typeof cancelIdleCallback === 'function' ? cancelIdleCallback : clearTimeout)(_adjacentPanelIdle);
    _adjacentPanelIdle = 0;
    const prev = getAdjacentMonth(-1);
    const next = getAdjacentMonth(1);
    _gridPrev.innerHTML = buildGridHTML(prev.year, prev.month);
    _gridNext.innerHTML = buildGridHTML(next.year, next.month);
  }

  function renderGrid() {
    const width = _gridContainer.offsetWidth;

    _gridContainer.style.height = "";
    _gridContainer.style.transition = "";

    for (const panel of _gridTrack.children) {
      panel.style.width = width + "px";
    }
    _gridTrack.style.width = (width * 3) + "px";
    _gridTrack.style.transition = "none";
    translateX(_gridTrack, -width);

    // Only render current month eagerly; adjacent panels on idle
    gridEl = _gridCur;
    gridEl.innerHTML = buildGridHTML(viewYear, viewMonth);
    scheduleAdjacentPanels();
  }

  // Optimized grid update after a swipe: rotate panels in the DOM instead of
  // re-rendering all three. Only the one new panel (the direction we swiped
  // towards) needs fresh innerHTML — the other two are already correct.
  // Does NOT reset track position — caller handles that for animation continuity.
  function rotateGridAfterSwipe(swipeDir) {
    // swipeDir: +1 = swiped to next month, -1 = swiped to prev month
    // Update model
    viewMonth += swipeDir;
    if (viewMonth > 11) { viewMonth = 0; viewYear++; }
    if (viewMonth < 0) { viewMonth = 11; viewYear--; }

    const width = _gridContainer.offsetWidth;

    // Rotate DOM order: move trailing panel to the opposite end
    if (swipeDir === 1) {
      // Going forward: prev is no longer needed, move it to become the new next
      _gridTrack.appendChild(_gridPrev);
    } else {
      // Going backward: next is no longer needed, move it to become the new prev
      _gridTrack.insertBefore(_gridNext, _gridTrack.firstChild);
    }

    // Update cached references to match new DOM order
    const panels = _gridTrack.children;
    _gridPrev = panels[0];
    _gridCur = panels[1];
    _gridNext = panels[2];
    gridEl = _gridCur;

    // Update panel widths
    for (const panel of panels) {
      panel.style.width = width + "px";
    }
    _gridTrack.style.width = (width * 3) + "px";

    // Only render the one new edge panel; current and opposite edge are reused
    if (swipeDir === 1) {
      const next = getAdjacentMonth(1);
      _gridNext.innerHTML = buildGridHTML(next.year, next.month);
    } else {
      const prev = getAdjacentMonth(-1);
      _gridPrev.innerHTML = buildGridHTML(prev.year, prev.month);
    }

    renderHeader();
    renderAgenda();
    loadEventsFromAPI();
  }

  function buildAgendaHTML(forDate) {
    const cacheKey = `${eventsVersion}|${room}|${forDate}`;
    if (agendaHtmlCache.has(cacheKey)) return agendaHtmlCache.get(cacheKey);

    const dayEvents = eventsForDate(forDate);

    if (dayEvents.length === 0) {
      const emptyHtml = `
        <div class="mc-empty">
          <div class="mc-empty-title">No bookings</div>
          <div class="mc-empty-sub">Tap + to create a booking for this day.</div>
        </div>
      `;
      agendaHtmlCache.set(cacheKey, emptyHtml);
      return emptyHtml;
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

    // Group events by owner + start + end time
    function groupEvents(items) {
      const groups = [];
      const groupMap = new Map();
      for (const m of items) {
        const key = `${m.ev.group_id}`;
        if (groupMap.has(key)) {
          groupMap.get(key).members.push(m);
        } else {
          const g = { key, members: [m], ...m };
          groups.push(g);
          groupMap.set(key, g);
        }
      }
      return groups;
    }

    function renderCard(group, extraStyle, isMultiTime) {
      const { members } = group;
      const first = members[0];
      const { ev, s, e, daysBefore, daysAfter } = first;
      const owned = ev.owner == room;
      const startISO = s.toISOString();
      const endISO = e.toISOString();
      const parts = ev.title.split(", ");
      const roomLabel = parts.length > 1 ? parts[0] : "";

      const eventIds = members.map(m => m.ev.id).join(",");
      const titles = members.map(m => m.ev.title).join("|||");

      // Always use pill layout
      const pillsHtml = members.map(m => {
        const p = m.ev.title.split(", ");
        const name = p.length > 1 ? p.slice(1).join(", ") : m.ev.title;
        return `<span class="mc-booking-pill" style="background:${m.ev.color || '#2563eb'}">${escapeHTML(name)}</span>`;
      }).join('');

      let timeHtml;
      if (isMultiTime) {
        const startIndicator = daysBefore > 0 ? `−${daysBefore}d` : '';
        const endIndicator = daysAfter > 0 ? `+${daysAfter}d` : '';
        timeHtml = `<div class="mc-booking-time mc-booking-time-multi">
          <span class="mc-time-part">${formatTimeFromDate(s)}${startIndicator ? `<span class="mc-booking-day-indicator">${startIndicator}</span>` : ''}</span>
          <span class="mc-time-sep">–</span>
          <span class="mc-time-part">${formatTimeFromDate(e)}${endIndicator ? `<span class="mc-booking-day-indicator">${endIndicator}</span>` : ''}</span>
        </div>`;
      } else {
        const endIndicator = daysAfter > 0 ? `+${daysAfter}d` : '';
        timeHtml = `<div class="mc-booking-time">${formatTimeFromDate(s)} – ${formatTimeFromDate(e)}${endIndicator ? `<div class="mc-booking-day-indicator">${endIndicator}</div>` : ''}</div>`;
      }

      return `<div class="mc-booking-card${owned ? " owned" : ""}" style="border-left-color:${ev.color || '#2563eb'}${extraStyle ? '; ' + extraStyle : ''}" data-event-ids="${eventIds}" data-owned="${owned}" data-start="${startISO}" data-end="${endISO}" data-title="${escapeAttr(ev.title)}" data-titles="${escapeAttr(titles)}">
        <div class="mc-booking-info">
          <div class="mc-booking-title">${owned ? "You, " : ""}${escapeHTML(roomLabel)}</div>
          <div class="mc-booking-pills">${pillsHtml}</div>
        </div>
        ${timeHtml}
      </div>`;
    }

    const continuationEvents = mapped.filter(m => m.startedBefore);
    const todayEvents = mapped.filter(m => !m.startedBefore);

    let html = '';

    // Continuation cards (started before today)
    const contGroups = groupEvents(continuationEvents);
    for (const group of contGroups) {
      html += renderCard(group, 'margin: 2px 12px 4px 18px', true);
    }

    // Build the set of hours that should show a tick mark:
    // - The hour each event starts in
    // - The hour immediately after each event's start hour (visual context)
    const tickHours = new Set();
    for (const { effectiveStart } of todayEvents) {
      const startH = effectiveStart.getHours();
      tickHours.add(startH);
      tickHours.add(Math.min(23, startH + 1));
    }

    // Determine the full range so we iterate in order
    let minHour = 23, maxHour = 0;
    for (const h of tickHours) {
      minHour = Math.min(minHour, h);
      maxHour = Math.max(maxHour, h);
    }

    for (let h = minHour; h <= maxHour; h++) {
      const hourEvents = todayEvents.filter(m => m.effectiveStart.getHours() === h);
      if (!tickHours.has(h)) continue;

      const hourLabel = formatTimeFromDate(new Date(1970, 0, 1, h, 0));
      html += `<div class="mc-agenda-hour">`;
      html += `<div class="mc-agenda-tick"><span class="mc-agenda-tick-label">${hourLabel}</span><span class="mc-agenda-tick-line"></span></div>`;

      const hourGroups = groupEvents(hourEvents);
      for (const group of hourGroups) {
        html += renderCard(group, '', false);
      }

      html += `</div>`;
    }

    agendaHtmlCache.set(cacheKey, html);
    return html;
  }

  function renderAgenda() {
    if (!preservingAgendaSwipeTrack) {
      finishPendingAgendaSwipe(false);
    }
    _agendaDate.textContent = formatDateLabel(selectedDate);
    if (deferAgendaBodyRender) return;
    _agendaEl.innerHTML = buildAgendaHTML(selectedDate);
    _agendaEl.style.visibility = "";
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
      return { id: key, text: res.name, color: resourceColors[key] || '#2563eb', depends_on: res.depends_on || null };
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
    }).sort(([, a], [, b]) => a.name.localeCompare(b.name));
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

  function getAdjacentMonth(dir) {
    let m = viewMonth + dir;
    let y = viewYear;
    if (m > 11) { m = 0; y++; }
    if (m < 0) { m = 11; y--; }
    return { year: y, month: m };
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
    _gridContainer.addEventListener("click", (e) => {
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
        loadEventsFromAPI(); // refetch for new month range
      }
      render();
      updateAgendaOverlayState();
      pushMobileUrlState();
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
    _loginBtn.addEventListener("click", () => {
      if (logged_in) {
        logout();
        setTimeout(() => renderLoginState(), 500);
      } else {
        showMobileLogin().then(() => {
          renderLoginState();
          loadEventsFromAPI();
        });
      }
    });

    // Booking card clicks (delegated from agenda)
    _agendaEl.addEventListener("click", (e) => {
      const card = e.target.closest(".mc-booking-card");
      if (!card) return;
      handleBookingClick(card);
    });

    // Smooth swipe left/right on grid for month nav
    setupGridSwipe();
  }

  let finishPendingSwipe = () => {};
  let finishPendingAgendaSwipe = () => {};
  let preservingAgendaSwipeTrack = false;
  let deferAgendaBodyRender = false;

  function setupGridSwipe() {
    const container = _gridContainer;
    const track = _gridTrack;
    let startX = 0;
    let startY = 0;
    let currentX = 0;
    let currentY = 0;
    let dragging = false;
    let swiping = false;
    let direction = null;
    let velocity = 0;
    let lastX = 0;
    let lastTime = 0;
    let containerW = 0;
    let animating = false;
    const scheduleTrackX = makeTransformScheduler(x => {
      translateX(track, x);
    });

    finishPendingSwipe = function() {
      if (animating) {
        animating = false;
        track.style.transition = "none";
      }
    };

    container.addEventListener("touchstart", (e) => {
      // Stop any in-flight animation — state is already committed
      if (animating) {
        animating = false;
        track.style.transition = "none";
        // Track is mid-animation; snap to center since content is already correct
        translateX(track, -container.offsetWidth);
      }

      // Ensure adjacent panels are rendered
      flushAdjacentPanels();

      startX = e.touches[0].clientX;
      startY = e.touches[0].clientY;
      lastX = startX;
      currentX = startX;
      currentY = startY;
      lastTime = Date.now();
      velocity = 0;
      dragging = true;
      swiping = false;
      direction = null;
      containerW = container.offsetWidth;
    }, { passive: true });

    container.addEventListener("touchmove", (e) => {
      if (!dragging) return;
      currentX = e.touches[0].clientX;
      currentY = e.touches[0].clientY;
      const dx = currentX - startX;
      const dy = currentY - startY;

      if (!direction && (Math.abs(dx) > 8 || Math.abs(dy) > 8)) {
        direction = Math.abs(dx) > Math.abs(dy) ? "horizontal" : "vertical";
      }
      if (direction === "vertical") return;
      if (direction !== "horizontal") return;

      e.preventDefault();
      const now = Date.now();
      const dt = now - lastTime;
      if (dt > 0) velocity = (currentX - lastX) / dt;
      lastX = currentX;
      lastTime = now;

      if (!swiping) {
        swiping = true;
        track.style.transition = "none";
      }

      scheduleTrackX(-containerW + dx);
    }, { passive: false });

    container.addEventListener("touchend", () => {
      if (!dragging) return;
      dragging = false;
      scheduleTrackX.cancel();
      if (!swiping) return;
      swiping = false;

      const dx = currentX - startX;
      const flingThreshold = 0.3;

      let swipeDir = 0;
      if (velocity > flingThreshold || dx > containerW * 0.25) swipeDir = -1;
      else if (velocity < -flingThreshold || dx < -containerW * 0.25) swipeDir = 1;

      if (swipeDir === 0) {
        // Snap back to center
        animating = true;
        track.style.transition = "transform 0.35s cubic-bezier(0.32, 0.72, 0, 1)";
        translateX(track, -containerW);
        const onEnd = () => {
          track.removeEventListener("transitionend", onEnd);
          if (animating) {
            animating = false;
            track.style.transition = "none";
          }
        };
        track.addEventListener("transitionend", onEnd);
        setTimeout(onEnd, 400);
        return;
      }

      // Commit the month change immediately.
      // The visual offset before rotation: -containerW + dx
      // After rotateGridAfterSwipe the panels shift by one slot:
      //   swipeDir=+1 (forward): old panel indices shift -1, so visual offset += containerW
      //   swipeDir=-1 (backward): old panel indices shift +1, so visual offset -= containerW
      const preRotateX = -containerW + dx;
      rotateGridAfterSwipe(swipeDir);

      // Set track to the corrected position so it visually appears unchanged
      const correctedX = preRotateX + (swipeDir * containerW);
      track.style.transition = "none";
      translateX(track, correctedX);

      // Force the browser to commit the position before starting the animation
      track.offsetWidth;

      // Animate to center
      animating = true;
      track.style.transition = "transform 0.35s cubic-bezier(0.32, 0.72, 0, 1)";
      translateX(track, -containerW);

      const onEnd = () => {
        track.removeEventListener("transitionend", onEnd);
        if (animating) {
          animating = false;
          track.style.transition = "none";
        }
      };
      track.addEventListener("transitionend", onEnd);
      setTimeout(onEnd, 400);

      updateAgendaOverlayState();
    });

    container.addEventListener("touchcancel", () => {
      dragging = false;
      swiping = false;
      direction = null;
      scheduleTrackX.cancel();
      track.style.transition = "none";
      translateX(track, -containerW);
    });
  }

  function setupAgendaOverlayDrag() {
    const overlay = _agendaOverlay;
    const grid = _gridContainer;
    const weekdays = document.querySelector(".mc-weekdays");

    // Two snap positions:
    // fullTop: covers calendar, just below app header
    // midTop: just below the calendar grid (lowest it can go)
    const headerHeight = 56;
    let dragBounds = null;
    const getFullTop = () => headerHeight;
    const getMidTop = () => {
      const rect = _gridContainer.getBoundingClientRect();
      return rect.bottom;
    };
    const getBounds = () => dragBounds || { fullTop: getFullTop(), midTop: getMidTop() };

    function getOverlayTop() {
      const transform = overlay.style.transform;
      if (transform) {
        const match = transform.match(/translateY\((.+)px\)/) || transform.match(/translate3d\(0px?,\s*(.+?)px,/);
        if (match) return parseFloat(match[1]);
      }
      return sheetOpen ? getFullTop() : getMidTop();
    }

    function getLiveOverlayTop() {
      return overlay.getBoundingClientRect().top;
    }

    function clampOverlayTop(y) {
      const bounds = getBounds();
      return Math.max(bounds.fullTop, Math.min(bounds.midTop, y));
    }

    function setOverlayTop(y) {
      const clamped = clampOverlayTop(y);
      positionAgendaOverlay(overlay, clamped);
      return clamped;
    }

    const scheduleOverlayTop = makeTransformScheduler(setOverlayTop);

    function snapSheet(velocity, currentTopOverride) {
      overlay.style.transition = "transform 0.35s cubic-bezier(0.32, 0.72, 0, 1)";

      const currentTop = currentTopOverride ?? getOverlayTop();
      const bounds = getBounds();
      const midPoint = (bounds.fullTop + bounds.midTop) / 2;

      const flingThreshold = 0.3;
      let snapTo;

      if (velocity < -flingThreshold) {
        snapTo = bounds.fullTop;
      } else if (velocity > flingThreshold) {
        snapTo = bounds.midTop;
      } else {
        snapTo = currentTop < midPoint ? bounds.fullTop : bounds.midTop;
      }

      setOverlayTop(snapTo);
      sheetOpen = snapTo <= bounds.fullTop;
      dragBounds = null;
    }

    function bindSheetDrag(surface, options = {}) {
      let startX = 0;
      let startY = 0;
      let startTop = 0;
      let dragging = false;
      let pendingDrag = false;
      let direction = null;
      let velocity = 0;
      let lastY = 0;
      let lastTime = 0;
      let latestTop = 0;

      surface.addEventListener("touchstart", (e) => {
        scheduleOverlayTop.cancel();
        dragBounds = { fullTop: getFullTop(), midTop: getMidTop() };
        startX = e.touches[0].clientX;
        startY = e.touches[0].clientY;
        startTop = getLiveOverlayTop();
        latestTop = startTop;
        lastY = startY;
        lastTime = Date.now();
        velocity = 0;
        dragging = false;
        pendingDrag = true;
        direction = null;
      }, { passive: true });

      surface.addEventListener("touchmove", (e) => {
        if (!pendingDrag) return;
        const x = e.touches[0].clientX;
        const y = e.touches[0].clientY;
        const dx = x - startX;
        let dy = y - startY;

        if (!direction && (Math.abs(dx) > 8 || Math.abs(dy) > 8)) {
          direction = Math.abs(dy) >= Math.abs(dx) ? "vertical" : "horizontal";
        }
        if (direction === "horizontal") return;
        if (direction !== "vertical") return;
        if (!dragging && options.nativeDownRefresh && dy > 0) return;

        const scrollEl = _agendaScroll;
        if (!dragging && options.allowAgendaScroll && e.target.closest(".mc-agenda-scroll")) {
          const maxScroll = scrollEl.scrollHeight - scrollEl.clientHeight;
          const sheetAtTop = getLiveOverlayTop() <= getFullTop() + 2;
          const canScrollUp = scrollEl.scrollTop > 0 && dy > 0;
          const canScrollDown = scrollEl.scrollTop < maxScroll && dy < 0;
          if (sheetAtTop && maxScroll > 1 && (canScrollUp || canScrollDown)) {
            startY = y;
            startTop = getLiveOverlayTop();
            latestTop = startTop;
            lastY = y;
            lastTime = Date.now();
            return;
          }
          if (!sheetAtTop || dy > 0) scrollEl.scrollTop = 0;
        }

        if (!dragging) {
          dragging = true;
          overlay.style.transition = "none";
          startY = y;
          startTop = getLiveOverlayTop();
          latestTop = startTop;
          lastY = y;
          lastTime = Date.now();
          setOverlayTop(startTop);
          dy = 0;
        }

        e.preventDefault();
        const now = Date.now();
        const dt = now - lastTime;
        if (dt > 0) velocity = (y - lastY) / dt;
        lastY = y;
        lastTime = now;
        latestTop = clampOverlayTop(startTop + dy);
        scheduleOverlayTop(startTop + dy);
      }, { passive: false });

      function finishDrag() {
        if (!pendingDrag) return;
        pendingDrag = false;
        if (!dragging) {
          dragBounds = null;
          return;
        }
        dragging = false;
        scheduleOverlayTop.cancel();
        setOverlayTop(latestTop);
        snapSheet(velocity, latestTop);
      }

      surface.addEventListener("touchend", finishDrag);
      surface.addEventListener("touchcancel", finishDrag);
    }

    bindSheetDrag(overlay, { allowAgendaScroll: true });
    bindSheetDrag(grid, { nativeDownRefresh: true });
    if (weekdays) bindSheetDrag(weekdays, { nativeDownRefresh: true });
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
      loadEventsFromAPI(); // refetch for new month range
    } else {
      renderGrid(); // update selected highlight
    }
    renderAgenda();
    pushMobileUrlState();
  }

  function setupAgendaSwipe() {
    const scrollEl = _agendaScroll;
    let startX = 0;
    let startY = 0;
    let tracking = false;
    let direction = null; // 'horizontal' or 'vertical'
    let velocity = 0;
    let lastX = 0;
    let lastTime = 0;
    let track = null;
    let containerW = 0;
    let agendaSwipeState = null;
    const scheduleAgendaTrackX = makeTransformScheduler(x => {
      if (track) translateX(track, x);
    });

    function getAdjacentDate(offset) {
      const d = new Date(selectedDate + "T12:00:00");
      d.setDate(d.getDate() + offset);
      return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
    }

    function createAgendaTrack() {
      removeAgendaTrack();
      containerW = scrollEl.offsetWidth;
      const scrollRect = scrollEl.getBoundingClientRect();
      const prevDate = getAdjacentDate(-1);
      const nextDate = getAdjacentDate(1);

      track = document.createElement("div");
      track.className = "mc-agenda-track";
      track.style.width = (containerW * 3) + "px";
      track.style.height = scrollRect.height + "px";
      track.style.position = "fixed";
      track.style.left = scrollRect.left + "px";
      track.style.top = scrollRect.top + "px";
      track.style.zIndex = "30";
      track.style.overflow = "hidden";
      translateX(track, -containerW);

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
      _agendaEl.style.display = "none";
      rootEl.appendChild(track);
    }

    function removeAgendaTrack() {
      rootEl.querySelectorAll(".mc-agenda-track").forEach(t => t.remove());
      track = null;
      _agendaEl.style.display = "";
      if (deferAgendaBodyRender) {
        deferAgendaBodyRender = false;
        renderAgenda();
      }
    }

    function clearAgendaSwipeState() {
      if (!agendaSwipeState) return null;
      const state = agendaSwipeState;
      if (state.timer) clearTimeout(state.timer);
      if (state.track && state.onEnd) {
        state.track.removeEventListener("transitionend", state.onEnd);
      }
      agendaSwipeState = null;
      return state;
    }

    function completeAgendaSwipe(commit) {
      const state = clearAgendaSwipeState();
      const swipeDir = state ? state.swipeDir : 0;

      tracking = false;
      direction = null;
      removeAgendaTrack();

      if (commit && swipeDir !== 0) {
        changeDay(swipeDir);
      }
    }

    finishPendingAgendaSwipe = function(commit = false) {
      completeAgendaSwipe(commit);
    };

    function cancelTracking() {
      tracking = false;
      direction = null;
      completeAgendaSwipe(false);
    }

    function finishSuccessfulSwipeAnimation(swipeDir) {
      const animatingTrack = track;
      track = null;
      _agendaEl.style.display = "";
      _agendaEl.style.visibility = "hidden";
      animatingTrack.style.pointerEvents = "none";

      deferAgendaBodyRender = true;
      preservingAgendaSwipeTrack = true;
      try {
        changeDay(swipeDir);
      } finally {
        preservingAgendaSwipeTrack = false;
      }

      const removeAnimatingTrack = () => {
        animatingTrack.removeEventListener("transitionend", removeAnimatingTrack);
        if (animatingTrack.parentNode) {
          animatingTrack.parentNode.removeChild(animatingTrack);
        }
        if (!rootEl.querySelector(".mc-agenda-track") && deferAgendaBodyRender) {
          deferAgendaBodyRender = false;
          renderAgenda();
        }
      };
      animatingTrack.addEventListener("transitionend", removeAnimatingTrack);
      setTimeout(removeAnimatingTrack, 400);
    }

    scrollEl.addEventListener("touchstart", (e) => {
      finishPendingAgendaSwipe(true);
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
        e.preventDefault();
        const now = Date.now();
        const dt = now - lastTime;
        if (dt > 0) velocity = (x - lastX) / dt;
        lastX = x;
        lastTime = now;

        // Create track on first horizontal movement
        if (!track) {
          createAgendaTrack();
          track.style.transition = "none";
          translateX(track, -containerW + dx);
        }

        if (track) {
          scheduleAgendaTrackX(-containerW + dx);
        }
      }
    }, { passive: false });

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
      translateX(track, targetX);

      if (swipeDir !== 0) {
        finishSuccessfulSwipeAnimation(swipeDir);
        return;
      }

      const onEnd = () => {
        completeAgendaSwipe(false);
      };

      agendaSwipeState = { track, swipeDir, onEnd, timer: null };
      track.addEventListener("transitionend", onEnd);
      agendaSwipeState.timer = setTimeout(() => {
        if (agendaSwipeState && agendaSwipeState.onEnd === onEnd) onEnd();
      }, 400);
    });

    scrollEl.addEventListener("touchcancel", cancelTracking);
  }

  function updateAgendaOverlayState() {
    const rect = _gridContainer.getBoundingClientRect();
    const midTop = rect.bottom;

    _agendaOverlay.style.transition = "transform 0.35s cubic-bezier(0.32, 0.72, 0, 1)";
    positionAgendaOverlay(_agendaOverlay, sheetOpen ? 56 : midTop);
  }

  // ---- Mobile Login Popup ----
  function showMobileLogin() {
    return new Promise((resolve) => {
      const overlay = document.createElement("div");
      overlay.className = "mc-login-overlay";
      overlay.innerHTML = `
        <div class="mc-login-backdrop"></div>
        <div class="mc-login-dialog">
          <div class="mc-login-header">
            <div class="mc-login-icon">${ICONS.user}</div>
            <div class="mc-detail-title">Login</div>
          </div>
          <div class="mc-login-body">
            <div class="mc-login-field">
              <label class="mc-nb-label" for="mc-login-user">Username</label>
              <input type="text" class="mc-nb-datetime" id="mc-login-user" placeholder="Username" autocomplete="username" autocapitalize="none">
            </div>
            <div class="mc-login-field">
              <label class="mc-nb-label" for="mc-login-pass">Password</label>
              <input type="password" class="mc-nb-datetime" id="mc-login-pass" placeholder="Password" autocomplete="current-password">
            </div>
            <div class="mc-login-error" id="mc-login-error"></div>
          </div>
          <div class="mc-login-actions">
            <button class="mc-detail-btn mc-detail-btn-close" id="mc-login-cancel">Cancel</button>
            <button class="mc-detail-btn mc-detail-btn-save" id="mc-login-submit">Login</button>
          </div>
        </div>
      `;

      document.body.appendChild(overlay);

      const backdrop = overlay.querySelector(".mc-login-backdrop");
      const dialog = overlay.querySelector(".mc-login-dialog");
      const userInput = overlay.querySelector("#mc-login-user");
      const passInput = overlay.querySelector("#mc-login-pass");
      const errorEl = overlay.querySelector("#mc-login-error");
      const submitBtn = overlay.querySelector("#mc-login-submit");

      function repositionDialog() {
        const vv = window.visualViewport;
        if (vv) {
          overlay.style.top = vv.offsetTop + "px";
          overlay.style.height = vv.height + "px";
        }
      }

      repositionDialog();
      if (window.visualViewport) {
        window.visualViewport.addEventListener("resize", repositionDialog);
        window.visualViewport.addEventListener("scroll", repositionDialog);
      }

      requestAnimationFrame(() => {
        backdrop.style.opacity = "1";
        dialog.style.opacity = "1";
        dialog.style.transform = "scale(1)";
        setTimeout(() => userInput.focus(), 300);
      });

      function closeOverlay() {
        if (window.visualViewport) {
          window.visualViewport.removeEventListener("resize", repositionDialog);
          window.visualViewport.removeEventListener("scroll", repositionDialog);
        }
        backdrop.style.opacity = "0";
        dialog.style.opacity = "0";
        dialog.style.transform = "scale(0.95)";
        setTimeout(() => {
          if (overlay.parentNode) overlay.remove();
        }, 250);
      }

      backdrop.addEventListener("click", () => {
        closeOverlay();
        resolve(false);
      });

      overlay.querySelector("#mc-login-cancel").addEventListener("click", () => {
        closeOverlay();
        resolve(false);
      });

      async function doLogin() {
        const user = userInput.value.trim();
        const pass = passInput.value;
        if (!user || !pass) {
          errorEl.textContent = "Please enter username and password";
          return;
        }

        submitBtn.disabled = true;
        submitBtn.textContent = "Logging in…";
        errorEl.textContent = "";

        try {
          const response = await sendPostRequest("/api/login", {
            username: user,
            password: pass,
          });

          if (response.status === 200) {
            const data = await response.json();
            onSignIn(data.user);
            closeOverlay();
            Toast.fire({ icon: "success", title: "Login successful" });
            resolve(true);
          } else {
            const errorText = await response.text();
            errorEl.textContent = errorText || "Login failed";
            submitBtn.disabled = false;
            submitBtn.textContent = "Login";
          }
        } catch (err) {
          errorEl.textContent = "Something went wrong";
          submitBtn.disabled = false;
          submitBtn.textContent = "Login";
        }
      }

      submitBtn.addEventListener("click", doLogin);

      passInput.addEventListener("keydown", (e) => {
        if (e.key === "Enter") doLogin();
      });
      userInput.addEventListener("keydown", (e) => {
        if (e.key === "Enter") passInput.focus();
      });
    });
  }

  function handleBookingClick(card) {
    const eventIds = (card.dataset.eventIds || card.dataset.eventId || '').split(',').map(id => parseInt(id, 10));
    const owned = card.dataset.owned === "true";
    const titles = (card.dataset.titles || card.dataset.title || '').split('|||');
    const startStr = card.dataset.start;
    const endStr = card.dataset.end;

    const startDate = new Date(startStr);
    const endDate = new Date(endStr);

    // Convert ISO (UTC) to local datetime-local format
    function toLocalDTL(isoStr) {
      const d = new Date(isoStr);
      d.setMinutes(d.getMinutes() - d.getTimezoneOffset());
      return d.toISOString().slice(0, 16);
    }
    const startLocal = toLocalDTL(startStr);
    const endLocal = toLocalDTL(endStr);

    const firstTitle = titles[0] || '';
    const parts = firstTitle.split(", ");
    const roomLabel = parts.length > 1 ? parts[0] : "";
    const isFuture = startDate > new Date();

    const timeStr = `${formatTimeFromDate(startDate)} – ${formatTimeFromDate(endDate)}`;
    const dateStr = formatDateLabel(startLocal.slice(0, 10));
    const isMultiDay = startDate.toDateString() !== endDate.toDateString();
    const dateRange = isMultiDay
      ? `${formatShortDate(startLocal.slice(0, 10))} – ${formatShortDate(endLocal.slice(0, 10))}`
      : dateStr;

    // Build header — always use pills
    const pillsHtml = eventIds.map((id, i) => {
      const ev = events.find(e => e.id === id);
      const t = titles[i] || '';
      const p = t.split(", ");
      const name = p.length > 1 ? p.slice(1).join(", ") : t;
      const color = ev ? ev.color : '#2563eb';
      return `<span class="mc-booking-pill" style="background:${color}">${escapeHTML(name)}</span>`;
    }).join('');
    const headerHtml = `
      <div class="mc-detail-header">
        <div>
          <div class="mc-detail-title">${escapeHTML(roomLabel)}</div>
          <div class="mc-detail-pills">${pillsHtml}</div>
        </div>
      </div>
    `;

    // Build bottom sheet
    const sheet = document.createElement("div");
    sheet.className = "mc-detail-sheet";
    sheet.innerHTML = `
      <div class="mc-detail-backdrop"></div>
      <div class="mc-detail-panel">
        ${headerHtml}
        <div class="mc-detail-body">
          <div class="mc-detail-row">
            <span class="mc-detail-icon">${ICONS.calendar}</span>
            <span>${dateRange}</span>
          </div>
          <div class="mc-detail-row">
            <span class="mc-detail-icon">${ICONS.clock}</span>
            <span>${timeStr}</span>
          </div>
          ${owned ? `<div class="mc-detail-row"><span class="mc-detail-icon">${ICONS.user}</span><span>Your booking</span></div>` : ''}
          ${owned && isFuture ? `
            <div class="mc-detail-edit" id="mc-detail-edit">
              <div class="mc-detail-edit-section">
                <label class="mc-nb-label" for="mc-detail-start">Start</label>
                <input type="datetime-local" class="mc-nb-datetime" id="mc-detail-start" value="${startLocal}" step="300">
              </div>
              <div class="mc-detail-edit-section">
                <label class="mc-nb-label" for="mc-detail-end">End</label>
                <input type="datetime-local" class="mc-nb-datetime" id="mc-detail-end" value="${endLocal}" step="300">
              </div>
            </div>
          ` : ''}
        </div>
        <div class="mc-detail-actions">
          ${owned && isFuture ? `
            <button class="mc-detail-btn mc-detail-btn-delete" id="mc-detail-delete">${ICONS.logout} Delete</button>
            <button class="mc-detail-btn mc-detail-btn-save" id="mc-detail-save">Reschedule</button>
          ` : `
            <button class="mc-detail-btn mc-detail-btn-close" id="mc-detail-close">Close</button>
          `}
        </div>
      </div>
    `;

    document.body.appendChild(sheet);

    // Animate in
    const backdrop = sheet.querySelector(".mc-detail-backdrop");
    const panel = sheet.querySelector(".mc-detail-panel");
    requestAnimationFrame(() => {
      backdrop.style.opacity = "1";
      panel.style.transform = "translateY(0)";
    });

    function closeSheet() {
      backdrop.style.opacity = "0";
      panel.style.transform = "translateY(100%)";
      setTimeout(() => {
        if (sheet.parentNode) sheet.remove();
      }, 300);
    }

    // Close on backdrop tap
    backdrop.addEventListener("click", closeSheet);

    // Drag down to dismiss
    {
      let startY = 0;
      let currentY = 0;
      let dragging = false;
      let panelStartY = 0;

      panel.addEventListener("touchstart", (e) => {
        startY = e.touches[0].clientY;
        currentY = startY;
        panelStartY = 0;
        dragging = true;
        panel.style.transition = "none";
      }, { passive: true });

      panel.addEventListener("touchmove", (e) => {
        if (!dragging) return;
        currentY = e.touches[0].clientY;
        const dy = currentY - startY;
        if (dy > 0) {
          e.preventDefault();
          panelStartY = dy;
          panel.style.transform = `translateY(${dy}px)`;
          backdrop.style.opacity = String(Math.max(0, 1 - dy / 300));
        }
      }, { passive: false });

      panel.addEventListener("touchend", () => {
        if (!dragging) return;
        dragging = false;
        panel.style.transition = "transform 0.35s cubic-bezier(0.32, 0.72, 0, 1)";
        backdrop.style.transition = "opacity 0.3s ease";
        if (panelStartY > 80) {
          closeSheet();
        } else {
          panel.style.transform = "translateY(0)";
          backdrop.style.opacity = "1";
        }
      });

      panel.addEventListener("touchcancel", () => {
        if (!dragging) return;
        dragging = false;
        panel.style.transition = "transform 0.35s cubic-bezier(0.32, 0.72, 0, 1)";
        backdrop.style.transition = "opacity 0.3s ease";
        panel.style.transform = "translateY(0)";
        backdrop.style.opacity = "1";
      });
    }

    if (owned && isFuture) {
      let deleteConfirmed = false;
      sheet.querySelector("#mc-detail-delete").addEventListener("click", (e) => {
        if (!deleteConfirmed) {
          deleteConfirmed = true;
          e.currentTarget.innerHTML = `${ICONS.logout} Are you sure?`;
          e.currentTarget.classList.add("mc-detail-btn-delete-confirm");
          return;
        }
        // Delete all events in the group
        sendPostRequest("/api/book/secure/delete", { ids: eventIds }).then(response => {
          if (response.ok) {
            Toast.fire({ icon: "success", title: eventIds.length > 1 ? `${eventIds.length} bookings deleted` : "Booking deleted" });
          } else {
            response.text().then(errorText => {
              Toast.fire({ icon: "error", title: "Deletion failed: " + errorText });
            });
          }
          loadEventsFromAPI();
        });
        closeSheet();
      });

      sheet.querySelector("#mc-detail-save").addEventListener("click", () => {
        const start = rfc3339(sheet.querySelector("#mc-detail-start").value);
        const end = rfc3339(sheet.querySelector("#mc-detail-end").value);
        // Reschedule all events in the group
        sendPostRequest("/api/book/secure/change", {
          ids: eventIds,
          start_time: start,
          end_time: end,
        }).then(response => {
          if (response.ok) {
            Toast.fire({ icon: "success", title: "Booking updated" });
          } else {
            response.text().then(errorText => {
              Toast.fire({ icon: "error", title: "Reschedule failed: " + errorText });
            });
          }
          loadEventsFromAPI();
        });
        closeSheet();
      });
    } else {
      sheet.querySelector("#mc-detail-close").addEventListener("click", closeSheet);
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
      showMobileLogin().then((result) => {
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
    pushMobileUrlState();
  }

  function closeNewBooking() {
    newBookingOpen = false;
    newBookingEl.classList.remove("active");
    if (mobileResourceSelect) {
      mobileResourceSelect.destroy();
      mobileResourceSelect = null;
    }
    pushMobileUrlState();
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
    // Fetch events for the visible month range plus a buffer for grid overflow days
    const rangeStart = new Date(viewYear, viewMonth - 1, 1);
    const rangeEnd = new Date(viewYear, viewMonth + 2, 1);
    const params = new URLSearchParams();
    params.set("start", rangeStart.toISOString());
    params.set("end", rangeEnd.toISOString());

    fetch("/api/book/events?" + params.toString())
      .then(r => r.json())
      .then(data => {
        events = data.map(ev => {
          ev.start = new Date(ev.start);
          ev.end = new Date(ev.end);
          return ev;
        });
        invalidateEventRenderCaches();
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
