/* ===== Custom Multi-Select Dropdown ===== */
/* Google Drive share-dialog style: type to search, arrow keys to navigate, */
/* Tab/Enter to insert highlighted item, Backspace to remove last chip */
/* Panel renders as a fixed overlay on document.body to avoid clipping */

(function () {
  "use strict";

  var CHECK_SVG = '<svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"/></svg>';
  var X_SVG = '<svg viewBox="0 0 24 24" width="12" height="12" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>';

  var instances = [];

  function MultiSelect(container, opts) {
    this.container = typeof container === "string" ? document.querySelector(container) : container;
    this.options = opts.options || [];
    this.placeholder = opts.placeholder || "Select...";
    this.onChange = opts.onChange || function () {};
    this.selected = new Map();
    this.isOpen = false;
    this.searchValue = "";
    this.highlightIndex = 0; // index into current filtered (unselected) list
    this._filteredOptions = []; // cached for keyboard nav
    this._destroyed = false;

    this._build();
    this._bind();
    instances.push(this);
  }

  // ---- Build DOM ----
  MultiSelect.prototype._build = function () {
    this.container.innerHTML = "";
    this.container.classList.add("ms-root");

    this.triggerEl = document.createElement("div");
    this.triggerEl.className = "ms-trigger";
    this.triggerEl.setAttribute("role", "combobox");
    this.triggerEl.setAttribute("aria-expanded", "false");

    this.chipsEl = document.createElement("div");
    this.chipsEl.className = "ms-chips";

    this.searchEl = document.createElement("input");
    this.searchEl.className = "ms-inline-search";
    this.searchEl.type = "text";
    this.searchEl.placeholder = this.placeholder;
    this.searchEl.setAttribute("autocomplete", "off");
    this.searchEl.setAttribute("autocorrect", "off");
    this.searchEl.setAttribute("spellcheck", "false");

    // Ghost text wrapper: input + prediction overlay
    this.searchWrap = document.createElement("div");
    this.searchWrap.className = "ms-search-wrap";

    this.ghostEl = document.createElement("span");
    this.ghostEl.className = "ms-ghost-text";

    this.searchWrap.appendChild(this.searchEl);
    this.searchWrap.appendChild(this.ghostEl);

    this.triggerEl.appendChild(this.chipsEl);
    this.triggerEl.appendChild(this.searchWrap);
    this.container.appendChild(this.triggerEl);

    // Panel on document.body
    this.panelEl = document.createElement("div");
    this.panelEl.className = "ms-panel";
    this.panelEl.setAttribute("role", "listbox");

    this.listEl = document.createElement("div");
    this.listEl.className = "ms-list";
    this.panelEl.appendChild(this.listEl);

    document.body.appendChild(this.panelEl);

    this._renderTrigger();
  };

  // ---- Get filtered, unselected options ----
  MultiSelect.prototype._getFilteredOptions = function () {
    var self = this;
    var filter = this.searchValue.toLowerCase();
    return this.options.filter(function (opt) {
      if (self.selected.has(opt.id)) return false;
      return !filter || opt.text.toLowerCase().indexOf(filter) !== -1;
    });
  };

  // ---- Render dropdown options ----
  MultiSelect.prototype._renderOptions = function () {
    var self = this;
    this._filteredOptions = this._getFilteredOptions();
    var filtered = this._filteredOptions;

    // Clamp highlight
    if (this.highlightIndex >= filtered.length) this.highlightIndex = Math.max(0, filtered.length - 1);

    this.listEl.innerHTML = "";

    if (filtered.length === 0) {
      var empty = document.createElement("div");
      empty.className = "ms-empty";
      empty.textContent = this.searchValue ? "No matches" : "All selected";
      this.listEl.appendChild(empty);
      this.ghostEl.textContent = "";
      return;
    }

    filtered.forEach(function (opt, idx) {
      var item = document.createElement("div");
      item.className = "ms-option";
      if (idx === self.highlightIndex) item.classList.add("highlighted");
      item.setAttribute("role", "option");
      item.setAttribute("data-id", opt.id);
      item.setAttribute("data-index", idx);

      var colorDot = "";
      if (opt.color) {
        colorDot = '<span class="ms-option-dot" style="background:' + opt.color + '"></span>';
      }

      item.innerHTML = colorDot + '<span class="ms-option-text">' + self._escapeHTML(opt.text) + '</span>';

      item.addEventListener("mousedown", function (e) {
        e.preventDefault();
      });

      item.addEventListener("click", function (e) {
        e.preventDefault();
        e.stopPropagation();
        self._selectOption(opt);
      });

      // Hover sets highlight
      item.addEventListener("mouseenter", function () {
        self.highlightIndex = idx;
        self._updateHighlight();
      });

      self.listEl.appendChild(item);
    });

    this._updateGhost();
  };

  // ---- Update ghost/prediction text ----
  MultiSelect.prototype._updateGhost = function () {
    var query = this.searchValue;
    var filtered = this._filteredOptions;
    if (!query || filtered.length === 0 || this.highlightIndex >= filtered.length) {
      this.ghostEl.textContent = "";
      return;
    }
    var highlighted = filtered[this.highlightIndex];
    var text = highlighted.text;
    // Case-insensitive prefix match: show typed portion + greyed completion
    var lower = text.toLowerCase();
    var queryLower = query.toLowerCase();
    if (lower.indexOf(queryLower) === 0) {
      this.ghostEl.textContent = query + text.slice(query.length);
    } else {
      // Not a prefix — no ghost
      this.ghostEl.textContent = "";
    }
  };

  // ---- Select an option ----
  MultiSelect.prototype._selectOption = function (opt) {
    this.selected.set(opt.id, { id: opt.id, text: opt.text });
    this.searchEl.value = "";
    this.searchValue = "";
    this.highlightIndex = 0;
    this.ghostEl.textContent = "";
    this._renderOptions();
    this._renderTrigger();
    this.onChange(this.getSelected());
    this.searchEl.focus();

    // Close panel if nothing left to select
    if (this._filteredOptions.length === 0) {
      this.close();
    } else {
      this._position();
    }
  };

  // ---- Update highlight without full re-render ----
  MultiSelect.prototype._updateHighlight = function () {
    var items = this.listEl.querySelectorAll(".ms-option");
    for (var i = 0; i < items.length; i++) {
      if (i === this.highlightIndex) {
        items[i].classList.add("highlighted");
        // Scroll into view if needed
        var itemTop = items[i].offsetTop;
        var itemBot = itemTop + items[i].offsetHeight;
        var scrollTop = this.listEl.scrollTop;
        var listH = this.listEl.clientHeight;
        if (itemTop < scrollTop) this.listEl.scrollTop = itemTop;
        else if (itemBot > scrollTop + listH) this.listEl.scrollTop = itemBot - listH;
      } else {
        items[i].classList.remove("highlighted");
      }
    }
    this._updateGhost();
  };

  // ---- Render chips in trigger ----
  MultiSelect.prototype._renderTrigger = function () {
    var self = this;
    this.chipsEl.innerHTML = "";

    if (this.selected.size === 0) {
      this.searchEl.placeholder = this.placeholder;
    } else {
      this.searchEl.placeholder = "";
      this.selected.forEach(function (val) {
        var chip = document.createElement("span");
        chip.className = "ms-chip";

        var colorOpt = self.options.find(function (o) { return o.id === val.id; });
        if (colorOpt && colorOpt.color) {
          var dot = document.createElement("span");
          dot.className = "ms-chip-dot";
          dot.style.background = colorOpt.color;
          chip.appendChild(dot);
        }

        var chipText = document.createElement("span");
        chipText.textContent = val.text;
        chip.appendChild(chipText);

        var chipX = document.createElement("button");
        chipX.className = "ms-chip-x";
        chipX.type = "button";
        chipX.innerHTML = X_SVG;
        chipX.addEventListener("mousedown", function (e) {
          e.preventDefault();
          e.stopPropagation();
        });
        chipX.addEventListener("click", function (e) {
          e.stopPropagation();
          self.selected.delete(val.id);
          self._renderTrigger();
          self._renderOptions();
          self.onChange(self.getSelected());
          self.searchEl.focus();
        });
        chip.appendChild(chipX);

        self.chipsEl.appendChild(chip);
      });
    }
    this._sizeInput();
  };

  MultiSelect.prototype._sizeInput = function () {
    var text = this.searchEl.value || this.searchEl.placeholder || "";
    this.searchEl.style.width = Math.max(60, text.length * 8 + 16) + "px";
  };

  // ---- Event binding ----
  MultiSelect.prototype._bind = function () {
    var self = this;

    // Click trigger → focus input
    this.triggerEl.addEventListener("click", function (e) {
      if (e.target === self.searchEl) return;
      self.searchEl.focus();
    });

    // Focus opens panel
    this.searchEl.addEventListener("focus", function () {
      if (!self.isOpen) self.open();
    });

    // Typing filters + opens
    this.searchEl.addEventListener("input", function () {
      self.searchValue = self.searchEl.value;
      self.highlightIndex = 0;
      self._sizeInput();
      self._renderOptions();
      if (!self.isOpen && self._filteredOptions.length > 0) self.open();
      if (self.isOpen) self._position();
    });

    // Panel interactions
    this.panelEl.addEventListener("mousedown", function (e) {
      e.preventDefault();
    });
    this.panelEl.addEventListener("touchstart", function (e) {
      e.stopPropagation();
    }, { passive: true });

    this.searchEl.addEventListener("click", function (e) {
      e.stopPropagation();
      if (!self.isOpen) self.open();
    });

    // Outside click/touch closes
    this._outsideHandler = function (e) {
      if (self._destroyed) return;
      if (self.container.contains(e.target)) return;
      if (self.panelEl.contains(e.target)) return;
      self.close();
    };
    document.addEventListener("mousedown", this._outsideHandler, true);
    document.addEventListener("touchstart", this._outsideHandler, { capture: true, passive: true });

    // Keyboard navigation
    this.searchEl.addEventListener("keydown", function (e) {
      var filtered = self._filteredOptions;

      if (e.key === "ArrowDown") {
        e.preventDefault();
        if (!self.isOpen) { self.open(); return; }
        self.highlightIndex = Math.min(self.highlightIndex + 1, filtered.length - 1);
        self._updateHighlight();
        return;
      }

      if (e.key === "ArrowUp") {
        e.preventDefault();
        if (!self.isOpen) return;
        self.highlightIndex = Math.max(self.highlightIndex - 1, 0);
        self._updateHighlight();
        return;
      }

      if (e.key === "Tab" || e.key === "Enter") {
        if (self.isOpen && filtered.length > 0 && self.highlightIndex < filtered.length) {
          e.preventDefault();
          e.stopPropagation();
          self._selectOption(filtered[self.highlightIndex]);
          return;
        }
        // If nothing to select, let Tab proceed naturally
        if (e.key === "Tab") return;
        e.stopPropagation();
        return;
      }

      if (e.key === "Escape") {
        if (self.isOpen) {
          e.preventDefault();
          self.close();
        }
        return;
      }

      // Backspace on empty input removes last chip
      if (e.key === "Backspace" && !self.searchEl.value && self.selected.size > 0) {
        var lastKey = null;
        self.selected.forEach(function (val, id) { lastKey = id; });
        if (lastKey !== null) {
          self.selected.delete(lastKey);
          self._renderTrigger();
          self._renderOptions();
          self.onChange(self.getSelected());
          if (self.isOpen) self._position();
        }
      }
    });

    // Reposition on scroll/resize
    this._repositionHandler = function () {
      if (self.isOpen) self._position();
    };
    window.addEventListener("scroll", this._repositionHandler, { capture: true, passive: true });
    window.addEventListener("resize", this._repositionHandler, { passive: true });
  };

  // ---- Positioning ----
  MultiSelect.prototype._position = function () {
    var rect = this.triggerEl.getBoundingClientRect();
    var panelHeight = this.panelEl.offsetHeight || 240;
    var viewportH = window.innerHeight;
    var spaceBelow = viewportH - rect.bottom - 8;
    var spaceAbove = rect.top - 8;

    var openAbove = spaceBelow < Math.min(panelHeight, 200) && spaceAbove > spaceBelow;
    var maxH;

    if (openAbove) {
      maxH = Math.min(240, spaceAbove);
      this.panelEl.style.top = "";
      this.panelEl.style.bottom = (viewportH - rect.top + 4) + "px";
    } else {
      maxH = Math.min(240, spaceBelow);
      this.panelEl.style.bottom = "";
      this.panelEl.style.top = (rect.bottom + 4) + "px";
    }

    this.panelEl.style.left = rect.left + "px";
    this.panelEl.style.width = Math.max(rect.width, 200) + "px";
    this.panelEl.style.maxHeight = maxH + "px";
    this.listEl.style.maxHeight = maxH + "px";
  };

  // ---- Open / Close ----
  MultiSelect.prototype.open = function () {
    this.isOpen = true;
    this.highlightIndex = 0;
    this.panelEl.classList.add("open");
    this.triggerEl.setAttribute("aria-expanded", "true");
    this.triggerEl.classList.add("ms-focused");
    this._renderOptions();
    this._position();
  };

  MultiSelect.prototype.close = function () {
    this.isOpen = false;
    this.panelEl.classList.remove("open");
    this.triggerEl.setAttribute("aria-expanded", "false");
    this.triggerEl.classList.remove("ms-focused");
    this.searchEl.value = "";
    this.searchValue = "";
    this.highlightIndex = 0;
    this.ghostEl.textContent = "";
    this._sizeInput();
  };

  // ---- Public API ----
  MultiSelect.prototype.getSelected = function () {
    var result = [];
    this.selected.forEach(function (val) { result.push(val); });
    return result;
  };

  MultiSelect.prototype.setOptions = function (options) {
    this.options = options;
    var ids = new Set(options.map(function (o) { return o.id; }));
    var self = this;
    this.selected.forEach(function (val, id) {
      if (!ids.has(id)) self.selected.delete(id);
    });
    this._renderOptions();
    this._renderTrigger();
  };

  MultiSelect.prototype.destroy = function () {
    this._destroyed = true;
    document.removeEventListener("mousedown", this._outsideHandler, true);
    document.removeEventListener("touchstart", this._outsideHandler, { capture: true });
    window.removeEventListener("scroll", this._repositionHandler, { capture: true });
    window.removeEventListener("resize", this._repositionHandler);
    if (this.panelEl.parentNode) this.panelEl.parentNode.removeChild(this.panelEl);
    this.container.innerHTML = "";
    this.container.classList.remove("ms-root");
    var idx = instances.indexOf(this);
    if (idx !== -1) instances.splice(idx, 1);
  };

  MultiSelect.prototype._escapeHTML = function (s) {
    var div = document.createElement("div");
    div.textContent = s;
    return div.innerHTML;
  };

  MultiSelect.isAnyOpen = function () {
    for (var i = 0; i < instances.length; i++) {
      if (instances[i].isOpen) return true;
    }
    return false;
  };

  window.MultiSelect = MultiSelect;
})();
