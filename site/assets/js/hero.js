/*
 * The hero figure: a working fragment of the editor's canvas.
 *
 * The page already contains a complete static figure, drawn from the same data
 * by site/tools/extract-outlines.py. This file adds what a static figure
 * cannot do — dragging a point, nudging it from the keyboard, switching
 * specimen, and rasterising the outline into a strike — and it reveals its own
 * controls, so a page without JavaScript never shows a control that does
 * nothing.
 *
 * Coordinates: the SVG is in font units with y flipped, so a node at font
 * position (x, y) is drawn at (x, -y). Nothing else converts.
 */
(function () {
  var data = window.FC_SPECIMENS;
  var stage = document.getElementById('stage');
  var svg = document.getElementById('stage-canvas');
  if (!data || !stage || !svg) return;

  var NS = 'http://www.w3.org/2000/svg';
  var STRIKES = [0, 12, 16, 24];

  var layers = {
    art: svg.querySelector('[data-layer="art"]'),
    pixels: svg.querySelector('[data-layer="pixels"]'),
    metrics: svg.querySelector('[data-layer="metrics"]'),
    outline: svg.querySelector('[data-layer="outline"]'),
    handles: svg.querySelector('[data-layer="handles"]'),
    nodes: svg.querySelector('[data-layer="nodes"]')
  };
  var hud = {
    point: svg.parentNode.querySelector('[data-hud="point"]'),
    points: svg.parentNode.querySelector('[data-hud="points"]'),
    advance: svg.parentNode.querySelector('[data-hud="advance"]'),
    curves: svg.parentNode.querySelector('[data-hud="curves"]'),
    ppem: svg.parentNode.querySelector('[data-hud="ppem"]')
  };
  var sourceLabel = stage.querySelector('[data-stage-source]');
  var slider = document.getElementById('stage-ppem');
  var resetButton = document.getElementById('stage-reset');
  var hint = document.getElementById('stage-hint');
  var labels = JSON.parse(stage.dataset.labels || '{}');

  var state = { key: 'latin', nodes: [], selected: -1, strike: 0 };

  /* ------------------------------------------------------------ geometry */

  function spec() {
    return data[state.key];
  }

  function element(name, attributes, className) {
    var node = document.createElementNS(NS, name);
    if (className) node.setAttribute('class', className);
    for (var key in attributes) node.setAttribute(key, attributes[key]);
    return node;
  }

  function reset() {
    state.nodes = spec().nodes.map(function (node) {
      return { x: node.x, y: node.y, t: node.t, s: node.s };
    });
    state.selected = -1;
  }

  /* Off-curve points attached to each on-curve point, and the two controls
     that define every implied point, both read once per specimen. */
  function relations() {
    var attached = {};
    var owners = {};
    spec().handles.forEach(function (pair) {
      (attached[pair[0]] = attached[pair[0]] || []).push(pair[1]);
      (owners[pair[1]] = owners[pair[1]] || []).push(pair[0]);
    });

    var implied = {};
    var commands = spec().commands;
    for (var index = 0; index < commands.length; index += 1) {
      var command = commands[index];
      if (command[0] !== 'Q') continue;
      var end = command[2];
      if (state.nodes[end] && state.nodes[end].t !== 'implied') continue;
      var next = commands[index + 1];
      if (next && next[0] === 'Q') implied[end] = [command[1], next[1]];
    }
    return { attached: attached, owners: owners, implied: implied };
  }

  var links = null;

  /* A TrueType implied point is the midpoint of the two controls around it.
     It is not something you drag; it is something that follows. */
  function settleImplied() {
    for (var index in links.implied) {
      var pair = links.implied[index];
      var a = state.nodes[pair[0]];
      var b = state.nodes[pair[1]];
      state.nodes[index].x = (a.x + b.x) / 2;
      state.nodes[index].y = (a.y + b.y) / 2;
    }
  }

  function moveNode(index, x, y) {
    var node = state.nodes[index];
    if (!node || node.t === 'implied') return;
    var dx = x - node.x;
    var dy = y - node.y;
    node.x = x;
    node.y = y;

    if (node.t === 'on') {
      // An on-curve point carries its own handles; the curve either side keeps
      // its shape and only its anchor moves.
      (links.attached[index] || []).forEach(function (control) {
        state.nodes[control].x += dx;
        state.nodes[control].y += dy;
      });
    } else {
      // Dragging a handle off a smooth point swings the opposite handle to
      // keep the tangent straight, which is what makes the node smooth.
      (links.owners[index] || []).forEach(function (owner) {
        var anchor = state.nodes[owner];
        if (!anchor.s) return;
        var siblings = (links.attached[owner] || []).filter(function (other) {
          return other !== index;
        });
        if (siblings.length !== 1) return;
        var sibling = state.nodes[siblings[0]];
        var length = Math.hypot(sibling.x - anchor.x, sibling.y - anchor.y);
        var reach = Math.hypot(anchor.x - node.x, anchor.y - node.y);
        if (reach < 1e-6 || length < 1e-6) return;
        sibling.x = anchor.x + ((anchor.x - node.x) / reach) * length;
        sibling.y = anchor.y + ((anchor.y - node.y) / reach) * length;
      });
    }
    settleImplied();
  }

  /* -------------------------------------------------------------- drawing */

  function pathString() {
    var out = [];
    spec().commands.forEach(function (command) {
      if (command[0] === 'Z') {
        out.push('Z');
        return;
      }
      var points = [];
      for (var index = 1; index < command.length; index += 1) {
        var node = state.nodes[command[index]];
        points.push(Math.round(node.x) + ' ' + Math.round(-node.y));
      }
      out.push(command[0] + points.join(' '));
    });
    return out.join('');
  }

  function drawMetrics() {
    var frame = data.frame;
    var current = spec();
    var fragment = document.createDocumentFragment();

    current.guides.forEach(function (guide) {
      var y = -guide[1];
      fragment.appendChild(
        element(
          'line',
          { x1: data.guideLeft, y1: y, x2: data.guideRight, y2: y },
          guide[1] === 0 ? 'cv-metric cv-metric--baseline' : 'cv-metric'
        )
      );
      var name = element(
        'text',
        { x: data.guideLeft, y: y - 12, 'font-size': data.labelSize },
        'cv-label'
      );
      name.textContent = guide[0];
      fragment.appendChild(name);
      var value = element(
        'text',
        {
          x: data.guideRight,
          y: y - 12,
          'text-anchor': 'end',
          'font-size': data.labelSize
        },
        'cv-label'
      );
      value.textContent = guide[1];
      fragment.appendChild(value);
    });

    layers.metrics.textContent = '';
    layers.metrics.appendChild(fragment);

    // The glyph rides in its own group, centred on its advance, so switching
    // specimen never moves the figure under the pointer.
    layers.art.setAttribute(
      'transform',
      'translate(' + Math.round(frame[0] + frame[2] / 2 - current.advance / 2) + ' 0)'
    );
    Array.prototype.forEach.call(layers.art.querySelectorAll('.cv-sidebearing'), function (line) {
      line.remove();
    });
    [0, current.advance].forEach(function (x) {
      layers.art.insertBefore(
        element(
          'line',
          { x1: x, y1: frame[1] + 40, x2: x, y2: frame[1] + frame[3] - 40 },
          'cv-sidebearing'
        ),
        layers.art.firstChild
      );
    });
  }

  function drawPoints() {
    var handles = document.createDocumentFragment();
    spec().handles.forEach(function (pair) {
      var a = state.nodes[pair[0]];
      var b = state.nodes[pair[1]];
      handles.appendChild(
        element('line', { x1: a.x, y1: -a.y, x2: b.x, y2: -b.y }, 'cv-handle')
      );
    });
    layers.handles.textContent = '';
    layers.handles.appendChild(handles);

    var nodes = document.createDocumentFragment();
    state.nodes.forEach(function (node, index) {
      var shape;
      var selected = index === state.selected ? ' cv-node--selected' : '';
      if (node.t === 'off') {
        shape = element('circle', { cx: node.x, cy: -node.y, r: 7 }, 'cv-node cv-node--off' + selected);
      } else if (node.t === 'implied') {
        shape = element(
          'circle',
          { cx: node.x, cy: -node.y, r: 5 },
          'cv-node cv-node--implied'
        );
      } else if (node.s) {
        shape = element('circle', { cx: node.x, cy: -node.y, r: 9 }, 'cv-node cv-node--smooth' + selected);
      } else {
        shape = element(
          'rect',
          { x: node.x - 8, y: -node.y - 8, width: 16, height: 16 },
          'cv-node' + selected
        );
      }
      nodes.appendChild(shape);

      if (node.t !== 'implied') {
        var hit = element('circle', { cx: node.x, cy: -node.y, r: 30 }, 'cv-hit');
        hit.dataset.index = index;
        nodes.appendChild(hit);
      }
    });
    layers.nodes.textContent = '';
    layers.nodes.appendChild(nodes);
  }

  /* The strike is rasterised from whatever outline is on screen right now,
     nonzero winding, sampled at each cell's centre — so dragging a point
     changes the pixels, which is the whole point of the two living in one
     document. The grid is anchored to the origin and the baseline, so a cell
     always falls where a cell would fall in a real strike. */

  var FLATTEN_STEPS = 12;

  function polygons() {
    function at(index) {
      var node = state.nodes[index];
      return [node.x, node.y];
    }

    function bezier(points, t) {
      var pts = points;
      while (pts.length > 1) {
        var next = [];
        for (var i = 0; i < pts.length - 1; i += 1) {
          next.push([
            pts[i][0] + (pts[i + 1][0] - pts[i][0]) * t,
            pts[i][1] + (pts[i + 1][1] - pts[i][1]) * t
          ]);
        }
        pts = next;
      }
      return pts[0];
    }

    var out = [];
    var current = [];
    var here = [0, 0];
    spec().commands.forEach(function (command) {
      if (command[0] === 'M') {
        current = [at(command[1])];
        here = at(command[1]);
      } else if (command[0] === 'L') {
        current.push(at(command[1]));
        here = at(command[1]);
      } else if (command[0] === 'Q' || command[0] === 'C') {
        var run = [here];
        for (var index = 1; index < command.length; index += 1) run.push(at(command[index]));
        for (var step = 1; step <= FLATTEN_STEPS; step += 1) {
          current.push(bezier(run, step / FLATTEN_STEPS));
        }
        here = at(command[command.length - 1]);
      } else if (command[0] === 'Z') {
        if (current.length > 2) out.push(current);
        current = [];
      }
    });
    return out;
  }

  function edges() {
    var out = [];
    polygons().forEach(function (polygon) {
      for (var index = 0; index < polygon.length; index += 1) {
        var a = polygon[index];
        var b = polygon[(index + 1) % polygon.length];
        if (a[1] !== b[1]) out.push([a[0], a[1], b[0], b[1]]);
      }
    });
    return out;
  }

  function show(node, visible) {
    if (visible) node.removeAttribute('hidden');
    else node.setAttribute('hidden', '');
  }

  function drawStrike() {
    layers.pixels.textContent = '';
    var ppem = STRIKES[state.strike];
    var showing = ppem > 0;
    show(layers.pixels, showing);
    show(layers.handles, !showing);
    stage.dataset.strike = showing ? 'on' : 'off';
    if (hud.ppem) hud.ppem.textContent = showing ? ppem + ' ppem' : labels.off || 'off';
    if (!showing) return;

    var cell = spec().upem / ppem;
    var lines = edges();
    if (!lines.length) return;

    var minX = Infinity;
    var maxX = -Infinity;
    var minY = Infinity;
    var maxY = -Infinity;
    lines.forEach(function (edge) {
      minX = Math.min(minX, edge[0], edge[2]);
      maxX = Math.max(maxX, edge[0], edge[2]);
      minY = Math.min(minY, edge[1], edge[3]);
      maxY = Math.max(maxY, edge[1], edge[3]);
    });

    var firstColumn = Math.floor(minX / cell);
    var lastColumn = Math.ceil(maxX / cell);
    var firstRow = Math.floor(minY / cell);
    var lastRow = Math.ceil(maxY / cell);
    var fragment = document.createDocumentFragment();

    for (var row = firstRow; row < lastRow; row += 1) {
      var y = (row + 0.5) * cell;
      var crossings = [];
      for (var index = 0; index < lines.length; index += 1) {
        var edge = lines[index];
        var y0 = edge[1];
        var y1 = edge[3];
        if ((y0 <= y && y < y1) || (y1 <= y && y < y0)) {
          var t = (y - y0) / (y1 - y0);
          crossings.push([edge[0] + (edge[2] - edge[0]) * t, y1 > y0 ? 1 : -1]);
        }
      }
      if (!crossings.length) continue;
      crossings.sort(function (a, b) {
        return a[0] - b[0];
      });

      var winding = 0;
      var cursor = 0;
      for (var column = firstColumn; column < lastColumn; column += 1) {
        var x = (column + 0.5) * cell;
        while (cursor < crossings.length && crossings[cursor][0] <= x) {
          winding += crossings[cursor][1];
          cursor += 1;
        }
        if (winding === 0) continue;
        fragment.appendChild(
          element(
            'rect',
            {
              x: column * cell,
              y: -(row + 1) * cell,
              width: cell,
              height: cell
            },
            'cv-pixel'
          )
        );
      }
    }
    layers.pixels.appendChild(fragment);
  }

  function drawHud() {
    var current = spec();
    if (hud.points) hud.points.textContent = current.points;
    if (hud.advance) hud.advance.textContent = current.advance + '/' + current.upem;
    if (hud.curves) {
      hud.curves.textContent =
        labels[current.curveType] ||
        (current.curveType === 'cubic' ? 'cubic' : 'quadratic');
    }
    if (!hud.point) return;
    if (state.selected < 0) {
      hud.point.textContent = labels.none || 'none selected';
    } else {
      var node = state.nodes[state.selected];
      hud.point.textContent =
        Math.round(node.x) + ', ' + Math.round(node.y) + ' ' + describe(node);
    }
  }

  function describe(node) {
    if (node.t === 'off') return labels.handle || 'handle';
    if (node.t === 'implied') return labels.implied || 'implied';
    return node.s ? labels.smooth || 'smooth' : labels.corner || 'corner';
  }

  function draw() {
    layers.outline.setAttribute('d', pathString());
    drawPoints();
    drawStrike();
    drawHud();
  }

  /* ------------------------------------------------------------- pointer */

  function toFontUnits(event) {
    var matrix = layers.art.getScreenCTM();
    if (!matrix) return null;
    var point = svg.createSVGPoint();
    point.x = event.clientX;
    point.y = event.clientY;
    var local = point.matrixTransform(matrix.inverse());
    return { x: local.x, y: -local.y };
  }

  var dragging = -1;

  svg.addEventListener('pointerdown', function (event) {
    var target = event.target;
    if (!target.dataset || target.dataset.index === undefined) return;
    dragging = Number(target.dataset.index);
    state.selected = dragging;
    svg.setPointerCapture(event.pointerId);
    svg.focus();
    event.preventDefault();
    draw();
  });

  svg.addEventListener('pointermove', function (event) {
    if (dragging < 0) return;
    var point = toFontUnits(event);
    if (!point) return;
    moveNode(dragging, Math.round(point.x), Math.round(point.y));
    draw();
  });

  function endDrag(event) {
    if (dragging < 0) return;
    dragging = -1;
    if (svg.hasPointerCapture(event.pointerId)) svg.releasePointerCapture(event.pointerId);
  }

  svg.addEventListener('pointerup', endDrag);
  svg.addEventListener('pointercancel', endDrag);

  /* ------------------------------------------------------------ keyboard */

  function draggableIndices() {
    var out = [];
    state.nodes.forEach(function (node, index) {
      if (node.t !== 'implied') out.push(index);
    });
    return out;
  }

  function step(direction) {
    var order = draggableIndices();
    if (!order.length) return;
    var position = order.indexOf(state.selected);
    position = position < 0 ? 0 : (position + direction + order.length) % order.length;
    state.selected = order[position];
  }

  svg.addEventListener('keydown', function (event) {
    var nudges = {
      ArrowLeft: [-1, 0],
      ArrowRight: [1, 0],
      ArrowUp: [0, 1],
      ArrowDown: [0, -1]
    };
    if (event.key === 'PageDown' || event.key === 'PageUp') {
      step(event.key === 'PageDown' ? 1 : -1);
      event.preventDefault();
      draw();
      return;
    }
    var nudge = nudges[event.key];
    if (!nudge) return;
    if (state.selected < 0) step(1);
    var amount = event.shiftKey ? 10 : 1;
    var node = state.nodes[state.selected];
    moveNode(state.selected, node.x + nudge[0] * amount, node.y + nudge[1] * amount);
    event.preventDefault();
    draw();
  });

  svg.addEventListener('focus', function () {
    if (state.selected < 0) {
      step(1);
      draw();
    }
  });

  /* ------------------------------------------------------------- controls */

  function select(key) {
    state.key = key;
    stage.dataset.specimen = key;
    state.strike = 0;
    if (slider) slider.value = '0';
    reset();
    links = relations();
    settleImplied();
    if (sourceLabel) sourceLabel.textContent = spec().source;
    drawMetrics();
    draw();
  }

  Array.prototype.forEach.call(stage.querySelectorAll('[data-specimen]'), function (tab) {
    tab.addEventListener('click', function () {
      Array.prototype.forEach.call(
        stage.querySelectorAll('[role="tab"]'),
        function (other) {
          other.setAttribute('aria-selected', String(other === tab));
        }
      );
      select(tab.dataset.specimen);
    });
  });

  if (slider) {
    slider.addEventListener('input', function () {
      state.strike = Number(slider.value);
      drawStrike();
    });
  }

  if (resetButton) {
    resetButton.addEventListener('click', function () {
      reset();
      settleImplied();
      draw();
    });
  }

  Array.prototype.forEach.call(stage.querySelectorAll('[data-controls]'), function (node) {
    node.hidden = false;
  });

  svg.setAttribute('tabindex', '0');
  if (hint) svg.setAttribute('aria-describedby', hint.id);

  reset();
  links = relations();
  settleImplied();
  draw();
})();
