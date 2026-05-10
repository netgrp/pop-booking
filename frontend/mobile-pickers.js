/* ===== Mobile Date & Time Pickers ===== */
/* Inline mini-calendar + scroll-wheel time picker */
/* Zero dependencies */

(function () {
  "use strict";

  var MONTH_NAMES = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun",
    "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
  ];
  var WEEKDAYS_SHORT = ["M", "T", "W", "T", "F", "S", "S"];
  var CHEVRON_L = '<svg viewBox="0 0 24 24"><polyline points="15 18 9 12 15 6"/></svg>';
  var CHEVRON_R = '<svg viewBox="0 0 24 24"><polyline points="9 6 15 12 9 18"/></svg>';

  function pad(n) { return String(n).padStart(2, "0"); }

  /* ============================================
   *  MiniCalendar — inline date picker
   * ============================================ */
  function MiniCalendar(container, opts) {
    this.el = typeof container === "string" ? document.querySelector(container) : container;
    this.selectedDate = opts.value || todayStr(); // "YYYY-MM-DD"
    this.onChange = opts.onChange || function () {};

    var d = new Date(this.selectedDate + "T12:00:00");
    this.viewYear = d.getFullYear();
    this.viewMonth = d.getMonth();

    this._build();
  }

  function todayStr() {
    var d = new Date();
    return d.getFullYear() + "-" + pad(d.getMonth() + 1) + "-" + pad(d.getDate());
  }

  function dateKey(y, m, day) {
    var dt = new Date(y, m, day);
    return dt.getFullYear() + "-" + pad(dt.getMonth() + 1) + "-" + pad(dt.getDate());
  }

  function buildMonthDays(year, month) {
    var firstDay = new Date(year, month, 1);
    var daysInMonth = new Date(year, month + 1, 0).getDate();
    var prevDays = new Date(year, month, 0).getDate();
    var mondayIdx = (firstDay.getDay() + 6) % 7;
    var days = [];

    for (var i = mondayIdx - 1; i >= 0; i--) {
      days.push({ day: prevDays - i, date: dateKey(year, month - 1, prevDays - i), muted: true });
    }
    for (var d = 1; d <= daysInMonth; d++) {
      days.push({ day: d, date: dateKey(year, month, d), muted: false });
    }
    var next = 1;
    while (days.length < 42) {
      days.push({ day: next, date: dateKey(year, month + 1, next), muted: true });
      next++;
    }
    // Trim trailing week if all muted
    while (days.length > 35 && days.slice(-7).every(function (d) { return d.muted; })) {
      days.splice(-7);
    }
    return days;
  }

  MiniCalendar.prototype._build = function () {
    this.el.innerHTML = "";
    this.el.classList.add("mp-cal");

    this.headerEl = document.createElement("div");
    this.headerEl.className = "mp-cal-header";

    this.prevBtn = document.createElement("button");
    this.prevBtn.type = "button";
    this.prevBtn.className = "mp-cal-nav";
    this.prevBtn.innerHTML = CHEVRON_L;

    this.titleEl = document.createElement("span");
    this.titleEl.className = "mp-cal-title";

    this.nextBtn = document.createElement("button");
    this.nextBtn.type = "button";
    this.nextBtn.className = "mp-cal-nav";
    this.nextBtn.innerHTML = CHEVRON_R;

    this.headerEl.appendChild(this.prevBtn);
    this.headerEl.appendChild(this.titleEl);
    this.headerEl.appendChild(this.nextBtn);
    this.el.appendChild(this.headerEl);

    // Weekday header
    var wdEl = document.createElement("div");
    wdEl.className = "mp-cal-weekdays";
    for (var i = 0; i < 7; i++) {
      var sp = document.createElement("span");
      sp.textContent = WEEKDAYS_SHORT[i];
      wdEl.appendChild(sp);
    }
    this.el.appendChild(wdEl);

    this.gridEl = document.createElement("div");
    this.gridEl.className = "mp-cal-grid";
    this.el.appendChild(this.gridEl);

    this._bindNav();
    this._render();
  };

  MiniCalendar.prototype._bindNav = function () {
    var self = this;
    this.prevBtn.addEventListener("click", function (e) {
      e.preventDefault();
      self.viewMonth--;
      if (self.viewMonth < 0) { self.viewMonth = 11; self.viewYear--; }
      self._render();
    });
    this.nextBtn.addEventListener("click", function (e) {
      e.preventDefault();
      self.viewMonth++;
      if (self.viewMonth > 11) { self.viewMonth = 0; self.viewYear++; }
      self._render();
    });
  };

  MiniCalendar.prototype._render = function () {
    var self = this;
    this.titleEl.textContent = MONTH_NAMES[this.viewMonth] + " " + this.viewYear;
    var days = buildMonthDays(this.viewYear, this.viewMonth);
    var today = todayStr();

    this.gridEl.innerHTML = "";
    days.forEach(function (d) {
      var btn = document.createElement("button");
      btn.type = "button";
      btn.className = "mp-cal-day";
      if (d.muted) btn.classList.add("muted");
      if (d.date === self.selectedDate) btn.classList.add("selected");
      if (d.date === today) btn.classList.add("today");
      btn.textContent = d.day;

      btn.addEventListener("click", function (e) {
        e.preventDefault();
        self.selectedDate = d.date;
        if (d.muted) {
          var dt = new Date(d.date + "T12:00:00");
          self.viewYear = dt.getFullYear();
          self.viewMonth = dt.getMonth();
        }
        self._render();
        self.onChange(self.selectedDate);
      });

      self.gridEl.appendChild(btn);
    });
  };

  MiniCalendar.prototype.getValue = function () {
    return this.selectedDate;
  };

  MiniCalendar.prototype.setValue = function (dateStr) {
    this.selectedDate = dateStr;
    var d = new Date(dateStr + "T12:00:00");
    this.viewYear = d.getFullYear();
    this.viewMonth = d.getMonth();
    this._render();
  };

  MiniCalendar.prototype.destroy = function () {
    this.el.innerHTML = "";
    this.el.classList.remove("mp-cal");
  };


  /* ============================================
   *  TimeWheel — scrolling hour:minute picker
   * ============================================ */
  function TimeWheel(container, opts) {
    this.el = typeof container === "string" ? document.querySelector(container) : container;
    this.onChange = opts.onChange || function () {};

    // Parse initial value "HH:MM"
    var parts = (opts.value || "12:00").split(":");
    this.hour = parseInt(parts[0], 10);
    this.minute = parseInt(parts[1], 10);
    // Snap minute to nearest 5
    this.minute = Math.round(this.minute / 5) * 5;
    if (this.minute >= 60) this.minute = 55;

    this.ITEM_H = 40; // height of each wheel item in px
    this.VISIBLE = 5; // visible items (should be odd)

    this._build();
  }

  TimeWheel.prototype._build = function () {
    this.el.innerHTML = "";
    this.el.classList.add("mp-time");

    // Highlight band
    var highlight = document.createElement("div");
    highlight.className = "mp-time-highlight";
    this.el.appendChild(highlight);

    // Hour wheel
    this.hourWheel = this._createWheel(24, this.hour, "hour");
    this.el.appendChild(this.hourWheel.container);

    // Separator
    var sep = document.createElement("span");
    sep.className = "mp-time-separator";
    sep.textContent = ":";
    this.el.appendChild(sep);

    // Minute wheel (5-min increments)
    this.minuteWheel = this._createWheel(12, this.minute / 5, "minute");
    this.el.appendChild(this.minuteWheel.container);
  };

  TimeWheel.prototype._createWheel = function (count, selected, type) {
    var self = this;
    var container = document.createElement("div");
    container.className = "mp-wheel";

    var inner = document.createElement("div");
    inner.className = "mp-wheel-inner";

    var items = [];
    for (var i = 0; i < count; i++) {
      var item = document.createElement("div");
      item.className = "mp-wheel-item";
      if (type === "minute") {
        item.textContent = pad(i * 5);
      } else {
        item.textContent = pad(i);
      }
      item.setAttribute("data-index", i);
      if (i === selected) item.classList.add("active");
      inner.appendChild(item);
      items.push(item);
    }

    container.appendChild(inner);

    // Center offset: position so selected item is in center
    var centerOffset = (this.VISIBLE * this.ITEM_H) / 2 - this.ITEM_H / 2;

    var wheel = {
      container: container,
      inner: inner,
      items: items,
      count: count,
      selected: selected,
      type: type,
      centerOffset: centerOffset,
    };

    this._positionWheel(wheel);
    this._bindWheel(wheel);
    return wheel;
  };

  TimeWheel.prototype._positionWheel = function (wheel) {
    var y = wheel.centerOffset - wheel.selected * this.ITEM_H;
    wheel.inner.style.transform = "translateY(" + y + "px)";

    // Update active class
    for (var i = 0; i < wheel.items.length; i++) {
      if (i === wheel.selected) {
        wheel.items[i].classList.add("active");
      } else {
        wheel.items[i].classList.remove("active");
      }
    }
  };

  TimeWheel.prototype._bindWheel = function (wheel) {
    var self = this;
    var startY = 0;
    var startTranslate = 0;
    var dragging = false;
    var lastY = 0;
    var velocity = 0;
    var lastTime = 0;
    var animFrame = null;

    function getTranslateY() {
      var style = window.getComputedStyle(wheel.inner);
      var matrix = style.transform || style.webkitTransform;
      if (matrix && matrix !== "none") {
        var values = matrix.match(/matrix.*\((.+)\)/);
        if (values) {
          var parts = values[1].split(", ");
          return parseFloat(parts[parts.length - 1]);
        }
      }
      return wheel.centerOffset - wheel.selected * self.ITEM_H;
    }

    function snapToNearest(currentY) {
      // Find which index this Y corresponds to
      var idx = Math.round((wheel.centerOffset - currentY) / self.ITEM_H);
      idx = Math.max(0, Math.min(wheel.count - 1, idx));
      wheel.selected = idx;
      self._positionWheel(wheel);
      self._emitChange();
    }

    // Touch events
    wheel.container.addEventListener("touchstart", function (e) {
      dragging = true;
      wheel.container.classList.add("dragging");
      if (animFrame) cancelAnimationFrame(animFrame);

      startY = e.touches[0].clientY;
      startTranslate = getTranslateY();
      lastY = startY;
      lastTime = Date.now();
      velocity = 0;
    }, { passive: true });

    wheel.container.addEventListener("touchmove", function (e) {
      if (!dragging) return;
      e.preventDefault();
      var y = e.touches[0].clientY;
      var diff = y - startY;
      var now = Date.now();
      var dt = now - lastTime;
      if (dt > 0) {
        velocity = (y - lastY) / dt;
      }
      lastY = y;
      lastTime = now;
      wheel.inner.style.transform = "translateY(" + (startTranslate + diff) + "px)";
    }, { passive: false });

    wheel.container.addEventListener("touchend", function () {
      if (!dragging) return;
      dragging = false;
      wheel.container.classList.remove("dragging");

      var currentY = getTranslateY();

      // Momentum scroll
      if (Math.abs(velocity) > 0.3) {
        var momentum = velocity * 120; // px to coast
        var targetY = currentY + momentum;
        var idx = Math.round((wheel.centerOffset - targetY) / self.ITEM_H);
        idx = Math.max(0, Math.min(wheel.count - 1, idx));
        wheel.selected = idx;
        self._positionWheel(wheel);
        self._emitChange();
      } else {
        snapToNearest(currentY);
      }
    }, { passive: true });

    // Mouse events for desktop testing
    wheel.container.addEventListener("mousedown", function (e) {
      e.preventDefault();
      dragging = true;
      wheel.container.classList.add("dragging");
      startY = e.clientY;
      startTranslate = getTranslateY();
      lastY = startY;
      lastTime = Date.now();
      velocity = 0;

      function onMouseMove(e) {
        if (!dragging) return;
        var y = e.clientY;
        var diff = y - startY;
        var now = Date.now();
        var dt = now - lastTime;
        if (dt > 0) {
          velocity = (y - lastY) / dt;
        }
        lastY = y;
        lastTime = now;
        wheel.inner.style.transform = "translateY(" + (startTranslate + diff) + "px)";
      }

      function onMouseUp() {
        dragging = false;
        wheel.container.classList.remove("dragging");
        document.removeEventListener("mousemove", onMouseMove);
        document.removeEventListener("mouseup", onMouseUp);

        var currentY = getTranslateY();
        if (Math.abs(velocity) > 0.3) {
          var momentum = velocity * 120;
          var targetY = currentY + momentum;
          var idx = Math.round((wheel.centerOffset - targetY) / self.ITEM_H);
          idx = Math.max(0, Math.min(wheel.count - 1, idx));
          wheel.selected = idx;
          self._positionWheel(wheel);
          self._emitChange();
        } else {
          snapToNearest(currentY);
        }
      }

      document.addEventListener("mousemove", onMouseMove);
      document.addEventListener("mouseup", onMouseUp);
    });

    // Click on item to select
    wheel.items.forEach(function (item, idx) {
      item.addEventListener("click", function (e) {
        e.stopPropagation();
        wheel.selected = idx;
        self._positionWheel(wheel);
        self._emitChange();
      });
    });
  };

  TimeWheel.prototype._emitChange = function () {
    this.hour = this.hourWheel.selected;
    this.minute = this.minuteWheel.selected * 5;
    this.onChange(this.getValue());
  };

  TimeWheel.prototype.getValue = function () {
    return pad(this.hour) + ":" + pad(this.minute);
  };

  TimeWheel.prototype.setValue = function (timeStr) {
    var parts = timeStr.split(":");
    this.hour = parseInt(parts[0], 10);
    this.minute = Math.round(parseInt(parts[1], 10) / 5) * 5;
    if (this.minute >= 60) this.minute = 55;
    this.hourWheel.selected = this.hour;
    this.minuteWheel.selected = this.minute / 5;
    this._positionWheel(this.hourWheel);
    this._positionWheel(this.minuteWheel);
  };

  TimeWheel.prototype.destroy = function () {
    this.el.innerHTML = "";
    this.el.classList.remove("mp-time");
  };


  /* ============================================
   *  Exports
   * ============================================ */
  window.MiniCalendar = MiniCalendar;
  window.TimeWheel = TimeWheel;

})();
