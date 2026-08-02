addEventListener("DOMContentLoaded", () => {
  for (let time of document.body.getElementsByTagName('time')) {
    time.setAttribute('title', new Date(time.textContent));
  }

  let titleLinks = document.querySelector('.title-links');
  if (titleLinks) {
    let urls = [];
    for (let dt of document.querySelectorAll('dt')) {
      if (dt.textContent.trim().toLowerCase() === 'links') {
        let dd = dt.nextElementSibling;
        if (dd) {
          for (let item of dd.querySelectorAll('li, a')) {
            let raw = item.tagName === 'A' ? item.href : item.textContent.trim();
            try {
              let u = new URL(raw);
              if ((u.protocol === 'http:' || u.protocol === 'https:') && u.hostname) {
                urls.push(u);
              }
            } catch (_) {}
          }
        }
        break;
      }
    }
    // Title-link factories — 1 link renders as a direct anchor; 2+ render as
    // icon + caret + dropdown menu. Used uniformly for marketplace, website,
    // and X icons so adding a new multi-link kind is just another call.
    function makeDirect(src, alt, label, href) {
      let a = document.createElement('a');
      a.href = href;
      a.target = '_blank';
      a.rel = 'noopener noreferrer';
      a.title = label;
      let img = document.createElement('img');
      img.className = 'icon';
      img.src = src;
      img.alt = alt;
      a.appendChild(img);
      return a;
    }

    function makeDropdown(src, alt, label, items) {
      let wrap = document.createElement('span');
      wrap.className = 'title-dropdown';
      let toggle = document.createElement('button');
      toggle.type = 'button';
      toggle.className = 'title-dropdown-toggle';
      toggle.title = label;
      let img = document.createElement('img');
      img.className = 'icon';
      img.src = src;
      img.alt = alt;
      toggle.appendChild(img);
      let caret = document.createElement('span');
      caret.className = 'title-dropdown-caret';
      toggle.appendChild(caret);
      let menu = document.createElement('div');
      menu.className = 'title-dropdown-menu';
      for (let item of items) {
        let a = document.createElement('a');
        a.href = item.href;
        a.target = '_blank';
        a.rel = 'noopener noreferrer';
        a.textContent = item.name;
        menu.appendChild(a);
      }
      toggle.addEventListener('click', e => {
        e.stopPropagation();
        let isOpen = menu.classList.contains('open');
        for (let m of document.querySelectorAll('.crumb-menu.open, .title-dropdown-menu.open')) {
          m.classList.remove('open');
        }
        if (!isOpen) menu.classList.add('open');
      });
      wrap.appendChild(toggle);
      wrap.appendChild(menu);
      return wrap;
    }

    // Marketplace items — off-chain (see marketplaces.js). Slugs vary per
    // marketplace, so this is hand-maintained, not derived from on-chain data.
    let mItems = [];
    let mIdMatch = location.pathname.match(/\/inscription\/([0-9a-f]{64}i\d+)/);
    let mSlugs = mIdMatch
      && typeof GALLERY_MARKETPLACES !== 'undefined'
      && GALLERY_MARKETPLACES[mIdMatch[1]];
    if (mSlugs && typeof MARKETPLACES !== 'undefined') {
      for (let key of Object.keys(MARKETPLACES)) {
        if (mSlugs[key]) {
          mItems.push({name: MARKETPLACES[key].name, href: MARKETPLACES[key].base + mSlugs[key]});
        }
      }
    }

    // Group on-chain links from the inscription's `links` attribute.
    let websites = [], xLinks = [];
    for (let u of urls) {
      let host = u.hostname.replace(/^www\./, '');
      if (host === 'x.com' || host === 'twitter.com') xLinks.push(u);
      else websites.push(u);
    }

    // Render order: marketplace, website(s), X, ordinals.com.
    if (mItems.length === 1) {
      titleLinks.appendChild(makeDirect('/static/marketplace.svg', 'marketplace', mItems[0].name, mItems[0].href));
    } else if (mItems.length > 1) {
      titleLinks.appendChild(makeDropdown('/static/marketplace.svg', 'marketplaces', 'marketplaces', mItems));
    }

    if (websites.length === 1) {
      titleLinks.appendChild(makeDirect('/static/link.svg', 'website', 'website', websites[0].href));
    } else if (websites.length > 1) {
      let items = websites.map(u => ({name: u.hostname.replace(/^www\./, ''), href: u.href}));
      titleLinks.appendChild(makeDropdown('/static/link.svg', 'websites', 'websites', items));
    }

    if (xLinks.length === 1) {
      titleLinks.appendChild(makeDirect('/static/x.svg', 'X', 'X', xLinks[0].href));
    } else if (xLinks.length > 1) {
      let items = xLinks.map(u => {
        let handle = (u.pathname.split('/').filter(Boolean)[0]) || u.hostname;
        return {name: '@' + handle, href: u.href};
      });
      titleLinks.appendChild(makeDropdown('/static/x.svg', 'X accounts', 'X accounts', items));
    }

    let ordPath = titleLinks.dataset.ordPath || (location.pathname + location.search);
    titleLinks.appendChild(makeDirect('/static/ordinals.svg', 'view on ordinals.com', 'view on ordinals.com', 'https://ordinals.com' + ordPath));
  }

  for (let form of document.querySelectorAll('.sort-form, .inscriptions-toolbar')) {
    for (let control of form.querySelectorAll('select[name=sort], input[name=cursed]')) {
      control.addEventListener('change', () => form.submit());
    }
  }

  let themeToggle = document.getElementById('theme-toggle');
  if (themeToggle) {
    const THEMES = ['dark', 'light'];
    themeToggle.addEventListener('click', () => {
      let current = document.documentElement.getAttribute('data-theme') || 'dark';
      let next = THEMES[(THEMES.indexOf(current) + 1) % THEMES.length];
      document.documentElement.setAttribute('data-theme', next);
      localStorage.setItem('theme', next);
    });
  }

  // Thumbnails marked `data-scriptable` (HTML and SVG inscriptions) ship with
  // a valueless `sandbox`, so their scripts never run — a grid of live
  // inscriptions burns enough CPU that iOS kills the renderer. Recursive art
  // is script-driven and renders blank while inert, so grant allow-scripts to
  // thumbnails that are on screen and revoke it once they leave, never
  // exceeding MAX_LIVE_THUMBNAILS at once.
  //
  // Sandbox flags are only applied when a frame navigates, so each transition
  // swaps in a fresh clone: setting `sandbox` before insertion means the load
  // happens under the new flags. `allow-same-origin` is never granted, so an
  // upgraded thumbnail still runs on an opaque origin and cannot reach this
  // document or anything stored against it.
  //
  // The ceiling matches GALLERY_PAGE_SIZE below, so a full grid page renders
  // rather than trailing off into blank tiles. Lower it if iOS still sheds the
  // renderer on heavy collections — the strip view only ever runs four or five
  // at once, so this really bounds the grid and the ungated thumbnail lists on
  // /inscriptions and the home page.
  const MAX_LIVE_THUMBNAILS = 20;

  // On-screen frames, and the subset of them currently running scripts. Both
  // are insertion-ordered, so a grid with more visible thumbnails than slots
  // fills them in registration (document) order.
  let visibleThumbnails = new Set();
  let liveThumbnails = new Set();

  let thumbnailObserver = 'IntersectionObserver' in window
    ? new IntersectionObserver(entries => {
        for (let entry of entries) {
          if (entry.isIntersecting) visibleThumbnails.add(entry.target);
          else visibleThumbnails.delete(entry.target);
        }
        reconcileThumbnails();
      }, {rootMargin: '200px'})
    : null;

  function swapThumbnail(frame, live) {
    let replacement = frame.cloneNode(false);
    replacement.setAttribute('sandbox', live ? 'allow-scripts' : '');
    replacement.dataset.scriptable = live ? 'live' : '';

    // Visibility has to survive the swap. The observer re-reports the
    // replacement asynchronously, and until it does, reconciliation would
    // otherwise read a still-on-screen thumbnail as hidden — evicting it,
    // freeing a slot, and re-upgrading it on the next callback forever.
    let visible = visibleThumbnails.has(frame);
    visibleThumbnails.delete(frame);
    liveThumbnails.delete(frame);
    if (thumbnailObserver) thumbnailObserver.unobserve(frame);

    frame.replaceWith(replacement);

    if (visible) visibleThumbnails.add(replacement);
    if (live) liveThumbnails.add(replacement);
    if (thumbnailObserver) thumbnailObserver.observe(replacement);
  }

  // Drop frames that have left the screen, then fill the free slots. Runs to a
  // fixed point: swaps re-add their replacement to the sets they were in, so a
  // reconcile pass never creates work for the next one.
  function reconcileThumbnails() {
    for (let frame of [...liveThumbnails]) {
      if (!visibleThumbnails.has(frame)) swapThumbnail(frame, false);
    }
    for (let frame of visibleThumbnails) {
      if (liveThumbnails.size >= MAX_LIVE_THUMBNAILS) break;
      if (!liveThumbnails.has(frame)) swapThumbnail(frame, true);
    }
  }

  function registerThumbnail(frame) {
    if (thumbnailObserver) {
      thumbnailObserver.observe(frame);
    } else if (liveThumbnails.size < MAX_LIVE_THUMBNAILS) {
      // No IntersectionObserver: upgrade the first screenful and leave the
      // rest inert rather than letting an unbounded grid go live.
      swapThumbnail(frame, true);
    }
  }

  for (let frame of document.querySelectorAll('iframe[data-scriptable]')) {
    registerThumbnail(frame);
  }

  const GALLERY_PAGE_SIZE = 20;

  for (let row of document.querySelectorAll('.gallery-row')) {
    let track = row.querySelector('.thumbnails');
    let prevBtn = row.querySelector('.gallery-prev');
    let nextBtn = row.querySelector('.gallery-next');
    if (!track) continue;

    let totalAvailable = parseInt(row.dataset.galleryTotal) || 0;
    let loadMoreUrl = row.dataset.loadMoreUrl || null;
    let serverPage = 0;
    let loading = false;
    let exhausted = !loadMoreUrl;

    let items = track.querySelectorAll(':scope > a');
    let total = items.length;
    if (totalAvailable && total >= totalAvailable) exhausted = true;

    let toolbar = row.parentElement.previousElementSibling;
    let viewBtns = toolbar && toolbar.classList.contains('with-toolbar')
      ? toolbar.querySelectorAll('.gallery-view-btn')
      : [];
    let mode = 'scroll';
    let page = 0;
    let pageCount = Math.max(1, Math.ceil(total / GALLERY_PAGE_SIZE));
    let tailObserver = null;

    function makeThumbnail(id) {
      let a = document.createElement('a');
      a.href = `/inscription/${id}`;
      let iframe = document.createElement('iframe');
      // Inert to match the server-rendered thumbnails; registerThumbnail then
      // upgrades it under the same cap once it scrolls into view.
      iframe.dataset.scriptable = '';
      iframe.setAttribute('sandbox', '');
      iframe.setAttribute('scrolling', 'no');
      iframe.setAttribute('loading', 'lazy');
      iframe.src = `/preview/${id}?thumb=1`;
      a.appendChild(iframe);
      return a;
    }

    function refreshState() {
      items = track.querySelectorAll(':scope > a');
      total = items.length;
      pageCount = Math.max(1, Math.ceil(total / GALLERY_PAGE_SIZE));
      if (totalAvailable && total >= totalAvailable) exhausted = true;
    }

    async function loadMore() {
      if (exhausted || loading) return false;
      loading = true;
      try {
        serverPage++;
        let response = await fetch(`${loadMoreUrl}/${serverPage}`, {
          headers: {Accept: 'application/json'},
        });
        if (!response.ok) {
          exhausted = true;
          return false;
        }
        let data = await response.json();
        let frag = document.createDocumentFragment();
        let added = [];
        for (let id of (data.ids || [])) {
          let anchor = makeThumbnail(id);
          added.push(anchor.querySelector('iframe'));
          frag.appendChild(anchor);
        }
        track.appendChild(frag);
        // Register after insertion — observing a detached node never fires.
        for (let frame of added) registerThumbnail(frame);
        if (!data.more) exhausted = true;
        refreshState();
        applyPage();
        updateArrows();
        setupTailObserver();
        return true;
      } catch (_) {
        exhausted = true;
        return false;
      } finally {
        loading = false;
      }
    }

    function setupTailObserver() {
      if (tailObserver) {
        tailObserver.disconnect();
        tailObserver = null;
      }
      if (exhausted || mode !== 'scroll' || items.length === 0) return;
      let last = items[items.length - 1];
      tailObserver = new IntersectionObserver((entries) => {
        for (let entry of entries) {
          if (entry.isIntersecting) loadMore();
        }
      }, {root: track, rootMargin: '0px 300px 0px 0px', threshold: 0});
      tailObserver.observe(last);
    }

    function applyPage() {
      let start = page * GALLERY_PAGE_SIZE;
      let end = start + GALLERY_PAGE_SIZE;
      items.forEach((item, idx) => {
        item.toggleAttribute('hidden', mode === 'all' && (idx < start || idx >= end));
      });
    }

    function updateArrows() {
      if (mode === 'scroll') {
        let max = track.scrollWidth - track.clientWidth;
        if (prevBtn) prevBtn.disabled = track.scrollLeft <= 1;
        if (nextBtn) nextBtn.disabled = track.scrollLeft >= max - 1 && exhausted;
      } else {
        if (prevBtn) prevBtn.disabled = page <= 0;
        if (nextBtn) nextBtn.disabled = page >= pageCount - 1 && exhausted;
      }
    }

    function setMode(nextMode) {
      if (mode === nextMode) return;
      mode = nextMode;
      row.classList.toggle('gallery-mode-scroll', mode === 'scroll');
      row.classList.toggle('gallery-mode-all', mode === 'all');
      viewBtns.forEach(btn => btn.classList.toggle('active', btn.dataset.mode === mode));
      page = 0;
      applyPage();
      updateArrows();
      setupTailObserver();
    }

    if (prevBtn) prevBtn.addEventListener('click', () => {
      if (mode === 'scroll') {
        track.scrollBy({left: -track.clientWidth, behavior: 'smooth'});
      } else if (page > 0) {
        page--; applyPage(); updateArrows();
      }
    });
    if (nextBtn) nextBtn.addEventListener('click', async () => {
      if (mode === 'scroll') {
        let max = track.scrollWidth - track.clientWidth;
        if (track.scrollLeft >= max - 1 && !exhausted) {
          await loadMore();
        }
        track.scrollBy({left: track.clientWidth, behavior: 'smooth'});
      } else {
        while (page + 1 >= pageCount && !exhausted && !loading) {
          await loadMore();
        }
        if (page + 1 < pageCount) {
          page++; applyPage(); updateArrows();
        }
      }
    });
    viewBtns.forEach(btn => btn.addEventListener('click', () => setMode(btn.dataset.mode)));

    track.addEventListener('scroll', updateArrows);
    addEventListener('resize', updateArrows);
    updateArrows();
    setupTailObserver();

    if (matchMedia('(max-width: 38rem)').matches && track.scrollWidth > track.clientWidth) {
      let observer = new IntersectionObserver((entries) => {
        for (let entry of entries) {
          if (!entry.isIntersecting) continue;
          observer.disconnect();
          setTimeout(() => row.classList.add('scroll-hint'), 1200);
        }
      }, {threshold: 0.5});
      observer.observe(track);
    }
  }

  let next = document.querySelector('a.next');
  let prev = document.querySelector('a.prev');

  window.addEventListener('keydown', e => {
    if (document.activeElement.tagName == 'INPUT') {
      return;
    }

    switch (e.key) {
      case 'ArrowRight':
        if (next) {
          window.location = next.href;
        }
        return;
      case 'ArrowLeft':
        if (prev) {
          window.location = prev.href;
        }
        return;
    }
  });

  const search = document.querySelector('form[action="/search"]');
  const query = search.querySelector('input[name="query"]');

  search.addEventListener('submit', (e) => {
    if (!query.value) {
      e.preventDefault();
    }
  });

  let collapse = document.getElementsByClassName('collapse');

  for (let el of collapse) {
    let text = el.textContent.trim();
    let btn = document.createElement('button');
    btn.type = 'button';
    btn.className = 'copy-btn';
    btn.title = 'copy';
    btn.setAttribute('aria-label', 'copy');
    let img = document.createElement('img');
    img.className = 'icon';
    img.src = '/static/copy.svg';
    img.alt = 'copy';
    btn.appendChild(img);
    btn.addEventListener('click', () => {
      navigator.clipboard.writeText(text);
      btn.title = 'copied!';
      btn.classList.add('copied');
      setTimeout(() => {
        btn.title = 'copy';
        btn.classList.remove('copied');
      }, 1500);
    });
    if (el.tagName === 'DD') {
      el.appendChild(btn);
    } else {
      el.insertAdjacentElement('afterend', btn);
    }
  }

  let context = document.createElement('canvas').getContext('2d');

  function resize() {
    for (let node of collapse) {
      if (!('original' in node.dataset)) {
        node.dataset.original = node.textContent.trim();
      }
      let original = node.dataset.original;
      let length = original.length;
      let width = node.clientWidth;
      if (width == 0) {
        width = node.parentNode.getBoundingClientRect().width;
      }
      let container = node.tagName === 'DD' ? node : node.parentNode;
      let btn = container.querySelector('.copy-btn');
      if (btn) {
        let s = window.getComputedStyle(btn);
        width -= btn.offsetWidth + (parseFloat(s.marginLeft) || 0) + (parseFloat(s.marginRight) || 0);
      }
      context.font = window.getComputedStyle(node).font;
      let capacity = width / (context.measureText(original).width / length);
      let text;
      if (capacity >= length) {
        text = original;
      } else {
        let count = Math.floor((capacity - 1) / 2);
        let start = original.substring(0, count);
        let end = original.substring(length - count);
        text = `${start}…${end}`;
      }
      let textNode = null;
      for (let child of node.childNodes) {
        if (child.nodeType === Node.TEXT_NODE) { textNode = child; break; }
      }
      if (textNode) {
        textNode.nodeValue = text;
      } else {
        node.insertBefore(document.createTextNode(text), node.firstChild);
      }
    }
  }

  function copy(e) {
    if ('original' in e.target.dataset && window.getSelection().toString().includes('…')) {
      e.clipboardData.setData('text/plain', e.target.dataset.original);
      e.preventDefault();
    }
  }

  addEventListener('resize', resize);

  addEventListener('copy', copy);

  document
    .querySelectorAll(`nav a[href="${CSS.escape(window.location.pathname)}"]`)
    .forEach(a => a.classList.add('active'));

  // ====================================================================
  //  Breadcrumb enhancements: nested-tree restructure, popout dropdowns,
  //  horizontal scroll with affordance fade.
  //
  //  The server renders breadcrumbs as a flat .breadcrumb-fork with one
  //  row per parent trail. The IIFE below progressively enhances that:
  //
  //    1. NESTED TREE  — when multiple trails share a leading crumb
  //       (e.g. .../MoBA/Inscription Clubs/Sub 1k and
  //              .../MoBA/Inscription Clubs/Sub-10k), collapse the shared
  //       "Inscription Clubs" into one row containing a sub-fork for the
  //       divergent tails. Recurses for deeper sharing. Renders an inline
  //       <svg> branch on the left of every fork level so the structure
  //       reads as one continuous shape.
  //
  //    2. POPOUT DROPDOWNS — the .breadcrumbs container uses overflow-x:
  //       auto so the breadcrumb can scroll horizontally on narrow
  //       viewports without reflowing. That overflow context would
  //       otherwise vertically clip each .crumb-menu when it opens, so
  //       every menu is moved to document.body and re-positioned
  //       position: fixed at the toggle's viewport rect. Position is
  //       viewport-clamped — menus near the left drop down-right, menus
  //       near the right drop down-left, neither overflows. Any scroll
  //       closes them so they never float around detached from a toggle.
  //
  //    3. SCROLL FADE — mask-image on .breadcrumbs fades the right edge
  //       when there's more content to scroll, and the left edge once
  //       scrolled past the start, as a swipe affordance.
  //
  //  All of this is progressive: if the JS doesn't run, the server's
  //  flat breadcrumb still renders fine.
  // ====================================================================

  (function setupBreadcrumb() {
    const breadcrumbs = document.querySelector('.breadcrumbs');
    if (!breadcrumbs) return;

    const originalFork = breadcrumbs.querySelector('.breadcrumb-fork');
    if (originalFork) nestForkAndAddSvgBranches(originalFork);
    setupCrumbDropdownPopout(breadcrumbs);
    setupScrollFade(breadcrumbs);

    // ----- 1. Nested tree restructure + SVG branches -----------------

    function nestForkAndAddSvgBranches(originalFork) {
      const idOf = crumb => {
        const link = crumb.querySelector('a[href^="/inscription/"]');
        return link
          ? link.getAttribute('href').slice('/inscription/'.length)
          : null;
      };

      // Each fork row -> array of { id, element } crumbs.
      const trails = Array
        .from(originalFork.querySelectorAll(':scope > .breadcrumb-fork-row'))
        .map(row => Array
          .from(row.querySelectorAll(':scope > .crumb'))
          .map(c => ({ id: idOf(c), element: c })));

      if (trails.length < 2) return;

      // Recursively group trails by their leading crumb id. Single-trail
      // groups become leaves; multi-trail groups factor out their longest
      // common prefix and recurse on the divergent tails.
      const buildTree = trails => {
        if (trails.length === 0) return [];
        if (trails.length === 1) return [{ type: 'leaf', crumbs: trails[0] }];

        const groups = [];
        for (const trail of trails) {
          if (trail.length === 0) continue;
          const id = trail[0].id;
          let g = groups.find(g => g.id === id);
          if (!g) { g = { id, trails: [] }; groups.push(g); }
          g.trails.push(trail);
        }

        const nodes = [];
        for (const { trails: gTrails } of groups) {
          if (gTrails.length === 1) {
            nodes.push({ type: 'leaf', crumbs: gTrails[0] });
            continue;
          }
          let prefixLen = 0;
          const minLen = Math.min(...gTrails.map(t => t.length));
          while (prefixLen < minLen
            && gTrails.every(t => t[prefixLen].id === gTrails[0][prefixLen].id)) {
            prefixLen++;
          }
          const sharedPrefix = gTrails[0].slice(0, prefixLen);
          const subTrails = gTrails
            .map(t => t.slice(prefixLen))
            .filter(t => t.length > 0);
          nodes.push({
            type: 'branch',
            prefix: sharedPrefix,
            children: subTrails.length > 0 ? buildTree(subTrails) : [],
          });
        }
        return nodes;
      };

      const tree = buildTree(trails);

      // Build the nested DOM. At each fork level a .breadcrumb-tree-group
      // wraps an <svg> branch + its .breadcrumb-fork. allGroups is filled
      // bottom-up by the recursion, so we can compute SVG paths after
      // layout in insertion order (innermost first).
      const SVG_NS = 'http://www.w3.org/2000/svg';
      const STEM_FRACTION = 0.45; // x-position of the junction inside the SVG
      const allGroups = [];

      const renderForkGroup = nodes => {
        const fork = document.createElement('div');
        fork.className = 'breadcrumb-fork';

        nodes.forEach(node => {
          const row = document.createElement('div');
          row.className = 'breadcrumb-fork-row';
          row.appendChild(document.createTextNode('/'));
          const inline = node.type === 'leaf' ? node.crumbs : node.prefix;
          inline.forEach((c, i) => {
            if (i > 0) row.appendChild(document.createTextNode('/'));
            row.appendChild(c.element);
          });
          if (node.type === 'branch' && node.children.length > 0) {
            row.appendChild(renderForkGroup(node.children));
          }
          fork.appendChild(row);
        });

        const svg = document.createElementNS(SVG_NS, 'svg');
        svg.setAttribute('viewBox', '0 0 100 100');
        svg.setAttribute('preserveAspectRatio', 'none');
        svg.classList.add('breadcrumb-tree-branch');
        const path = document.createElementNS(SVG_NS, 'path');
        path.setAttribute('stroke', 'currentColor');
        path.setAttribute('stroke-width', '2');
        path.setAttribute('fill', 'none');
        path.setAttribute('vector-effect', 'non-scaling-stroke');
        path.setAttribute('stroke-linecap', 'square');
        path.setAttribute('stroke-linejoin', 'miter');
        svg.appendChild(path);

        const group = document.createElement('span');
        group.className = 'breadcrumb-tree-group';
        group.appendChild(svg);
        group.appendChild(fork);

        allGroups.push({ fork, svg, path });
        return group;
      };

      const outerGroup = renderForkGroup(tree);
      originalFork.parentNode.replaceChild(outerGroup, originalFork);

      // Set each SVG path. Arm y-positions are measured against actual row
      // positions so they line up on each row's vertical centre even when
      // one row is taller than its siblings (e.g. a branch row whose
      // content includes a sub-fork-group).
      const jx = STEM_FRACTION * 100;
      allGroups.forEach(({ fork, svg, path }) => {
        const forkRect = fork.getBoundingClientRect();
        const forkHeight = forkRect.height;
        if (forkHeight === 0) return;
        svg.style.height = forkHeight + 'px';
        let d = `M0 50 H${jx}`;
        const rows = Array
          .from(fork.querySelectorAll(':scope > .breadcrumb-fork-row'));
        rows.forEach(row => {
          const rowRect = row.getBoundingClientRect();
          const centerY = (rowRect.top + rowRect.bottom) / 2 - forkRect.top;
          const yPct = (centerY / forkHeight) * 100;
          d += ` M${jx} 50 L${jx} ${yPct} L100 ${yPct}`;
        });
        path.setAttribute('d', d);
      });
    }

    // ----- 2. Dropdown popout ----------------------------------------

    function setupCrumbDropdownPopout(breadcrumbs) {
      const menuPairs = [];

      breadcrumbs.querySelectorAll('.crumb-toggle').forEach(originalToggle => {
        const menu = originalToggle.nextElementSibling;
        if (!menu || !menu.classList.contains('crumb-menu')) return;

        // Preserve inherited color/font before pulling the menu out of
        // .breadcrumb (where `color: inherit` gives the breadcrumb's muted
        // tone; out in body it would otherwise pick up the default link
        // colour).
        const cs = window.getComputedStyle(menu);
        const inheritedColor = cs.color;
        const inheritedFont = cs.fontFamily;
        const inheritedSize = cs.fontSize;

        // Clone the toggle to drop any prior click handler (the global
        // outside-click listener still applies — see below), so we own the
        // open/close + positioning fully.
        const toggle = originalToggle.cloneNode(true);
        originalToggle.parentNode.replaceChild(toggle, originalToggle);

        // Move the menu to body so no overflow ancestor can clip it.
        document.body.appendChild(menu);
        menu.style.color = inheritedColor;
        menu.style.fontFamily = inheritedFont;
        menu.style.fontSize = inheritedSize;
        menu.querySelectorAll('a').forEach(a => {
          a.style.color = 'inherit';
          a.style.textDecoration = 'none';
        });

        const positionMenu = () => {
          const rect = toggle.getBoundingClientRect();
          const margin = 8;
          const vw = window.innerWidth;
          menu.style.position = 'fixed';
          menu.style.zIndex = '1000';
          menu.style.top = (rect.bottom + 4) + 'px';
          // Default: drop down-right, align menu's left with toggle's left.
          menu.style.left = rect.left + 'px';
          menu.style.right = 'auto';
          const mw = menu.offsetWidth;
          if (rect.left + mw > vw - margin) {
            // Wouldn't fit going right — try right-align (drop down-left).
            menu.style.left = 'auto';
            menu.style.right = (vw - rect.right) + 'px';
            if (rect.right - mw < margin) {
              // Wouldn't fit either way — clamp to viewport with margin.
              menu.style.left = margin + 'px';
              menu.style.right = 'auto';
            }
          }
        };

        toggle.addEventListener('click', e => {
          e.stopPropagation();
          const wasOpen = menu.classList.contains('open');
          document
            .querySelectorAll('.crumb-menu.open, .title-dropdown-menu.open')
            .forEach(m => m.classList.remove('open'));
          if (!wasOpen) {
            menu.classList.add('open');
            positionMenu();
          }
        });
        menuPairs.push({ toggle, menu });
      });

      // Any scroll closes any open menu so it never floats around detached
      // from its toggle.
      const closeAll = () =>
        menuPairs.forEach(({ menu }) => menu.classList.remove('open'));
      window.addEventListener('scroll', closeAll, { passive: true });
      breadcrumbs.addEventListener('scroll', closeAll, { passive: true });
    }

    // ----- 3. Scroll affordance fade ---------------------------------

    function setupScrollFade(breadcrumbs) {
      const FADE = '2rem';
      const update = () => {
        const hasOverflow =
          breadcrumbs.scrollWidth > breadcrumbs.clientWidth + 1;
        const atStart = breadcrumbs.scrollLeft <= 1;
        const atEnd = breadcrumbs.scrollLeft + breadcrumbs.clientWidth
          >= breadcrumbs.scrollWidth - 1;
        const fadeLeft = hasOverflow && !atStart;
        const fadeRight = hasOverflow && !atEnd;
        const left = fadeLeft
          ? `transparent 0, black ${FADE}`
          : 'black 0';
        const right = fadeRight
          ? `black calc(100% - ${FADE}), transparent 100%`
          : 'black 100%';
        const gradient = `linear-gradient(to right, ${left}, ${right})`;
        breadcrumbs.style.maskImage = gradient;
        breadcrumbs.style.webkitMaskImage = gradient;
      };
      update();
      breadcrumbs.addEventListener('scroll', update, { passive: true });
      window.addEventListener('resize', update, { passive: true });
    }
  })();

  addEventListener('click', () => {
    for (let m of document.querySelectorAll('.crumb-menu.open, .title-dropdown-menu.open')) {
      m.classList.remove('open');
    }
  });

  resize();
});
