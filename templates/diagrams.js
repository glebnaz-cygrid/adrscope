/**
 * ADRScope Diagram Layer
 *
 * Turns the fenced code blocks that the Rust markdown pipeline emits as
 * <pre><code class="language-{mermaid,drawio,excalidraw}"> into live,
 * pan- and zoomable canvases.
 *
 * All three back-ends produce SVG, so a single CSS-transform viewport gives
 * every diagram the same interaction model. Renderer bundles are embedded by
 * the generator only when the ADR corpus actually uses them, so this file has
 * to degrade gracefully when a bundle is absent.
 */
(function(global) {
    'use strict';

    var KINDS = ['mermaid', 'drawio', 'excalidraw'];
    var MIN_SCALE = 0.1;
    var MAX_SCALE = 8;
    var ZOOM_STEP = 1.25;
    var WHEEL_STEP = 1.12;

    var seq = 0;
    var cache = new Map();
    var mermaidTheme = null;

    // =========================================================================
    // Theme
    // =========================================================================
    function resolveTheme() {
        var theme = document.documentElement.dataset.theme;
        if (theme === 'light' || theme === 'dark') return theme;
        return global.matchMedia && global.matchMedia('(prefers-color-scheme: dark)').matches
            ? 'dark'
            : 'light';
    }

    // =========================================================================
    // Pan / zoom shell
    // =========================================================================
    function makeCanvas(kind) {
        var frame = document.createElement('figure');
        frame.className = 'diagram-frame';
        frame.dataset.diagram = kind;

        var toolbar = document.createElement('div');
        toolbar.className = 'diagram-toolbar';

        var label = document.createElement('span');
        label.className = 'diagram-label';
        label.textContent = kind;
        toolbar.appendChild(label);

        var viewport = document.createElement('div');
        viewport.className = 'diagram-viewport';

        var stage = document.createElement('div');
        stage.className = 'diagram-stage';
        viewport.appendChild(stage);

        frame.appendChild(toolbar);
        frame.appendChild(viewport);

        var view = { scale: 1, x: 0, y: 0 };

        function apply() {
            stage.style.transform =
                'translate(' + view.x + 'px, ' + view.y + 'px) scale(' + view.scale + ')';
        }

        function zoomAt(factor, cx, cy) {
            var next = Math.min(MAX_SCALE, Math.max(MIN_SCALE, view.scale * factor));
            var ratio = next / view.scale;
            view.x = cx - ratio * (cx - view.x);
            view.y = cy - ratio * (cy - view.y);
            view.scale = next;
            apply();
        }

        function zoomCenter(factor) {
            zoomAt(factor, viewport.clientWidth / 2, viewport.clientHeight / 2);
        }

        function fit() {
            var content = stage.firstElementChild;
            if (!content) return;

            view.scale = 1;
            view.x = 0;
            view.y = 0;
            apply();

            var contentBox = content.getBoundingClientRect();
            var viewBox = viewport.getBoundingClientRect();
            if (!contentBox.width || !contentBox.height) return;

            // 0.94 keeps a little breathing room around the diagram.
            var scale = Math.min(
                viewBox.width / contentBox.width,
                viewBox.height / contentBox.height
            ) * 0.94;

            view.scale = Math.min(MAX_SCALE, Math.max(MIN_SCALE, scale));
            view.x = (viewBox.width - contentBox.width * view.scale) / 2;
            view.y = (viewBox.height - contentBox.height * view.scale) / 2;
            apply();
        }

        function toggleFullscreen() {
            if (document.fullscreenElement === frame) {
                document.exitFullscreen();
            } else if (frame.requestFullscreen) {
                frame.requestFullscreen().then(function() {
                    global.requestAnimationFrame(fit);
                }, function() {});
            }
        }

        viewport.addEventListener('wheel', function(e) {
            e.preventDefault();
            var box = viewport.getBoundingClientRect();
            zoomAt(e.deltaY < 0 ? WHEEL_STEP : 1 / WHEEL_STEP, e.clientX - box.left, e.clientY - box.top);
        }, { passive: false });

        var drag = null;
        viewport.addEventListener('pointerdown', function(e) {
            drag = { x: e.clientX, y: e.clientY };
            viewport.setPointerCapture(e.pointerId);
            viewport.classList.add('dragging');
        });
        viewport.addEventListener('pointermove', function(e) {
            if (!drag) return;
            view.x += e.clientX - drag.x;
            view.y += e.clientY - drag.y;
            drag.x = e.clientX;
            drag.y = e.clientY;
            apply();
        });
        ['pointerup', 'pointercancel'].forEach(function(name) {
            viewport.addEventListener(name, function() {
                drag = null;
                viewport.classList.remove('dragging');
            });
        });
        viewport.addEventListener('dblclick', fit);

        var buttons = [
            ['−', 'Zoom out', function() { zoomCenter(1 / ZOOM_STEP); }],
            ['+', 'Zoom in', function() { zoomCenter(ZOOM_STEP); }],
            ['⤡', 'Fit to view (double-click)', fit],
            ['⛶', 'Fullscreen', toggleFullscreen]
        ];
        buttons.forEach(function(spec) {
            var button = document.createElement('button');
            button.type = 'button';
            button.className = 'diagram-btn';
            button.textContent = spec[0];
            button.title = spec[1];
            button.setAttribute('aria-label', spec[1]);
            button.addEventListener('click', spec[2]);
            toolbar.appendChild(button);
        });

        return {
            frame: frame,
            stage: stage,
            fit: fit,
            width: function() { return viewport.clientWidth; }
        };
    }

    // =========================================================================
    // Helpers
    // =========================================================================

    /**
     * Gives an SVG an intrinsic pixel size taken from its viewBox. Inside the
     * absolutely positioned stage a responsive SVG collapses to zero.
     */
    function sizeFromViewBox(svg) {
        var box = (svg.getAttribute('viewBox') || '').split(/[\s,]+/).map(Number);
        var width = box.length === 4 && box[2] ? box[2] : parseFloat(svg.getAttribute('width')) || 0;
        var height = box.length === 4 && box[3] ? box[3] : parseFloat(svg.getAttribute('height')) || 0;

        svg.removeAttribute('width');
        svg.removeAttribute('height');
        svg.style.maxWidth = 'none';
        if (width && height) {
            svg.style.width = width + 'px';
            svg.style.height = height + 'px';
        }
    }

    function showError(canvas, kind, error) {
        var box = document.createElement('div');
        box.className = 'diagram-error';
        box.textContent = 'Failed to render ' + kind + ' diagram: ' +
            (error && error.message ? error.message : String(error));
        canvas.stage.appendChild(box);
    }

    function showSource(canvas, source) {
        var pre = document.createElement('pre');
        var code = document.createElement('code');
        code.textContent = source;
        pre.appendChild(code);
        pre.className = 'diagram-source';
        canvas.stage.appendChild(pre);
    }

    // =========================================================================
    // Renderers
    // =========================================================================
    function renderMermaid(source, canvas) {
        if (!global.mermaid) throw new Error('mermaid renderer is not embedded');

        var theme = resolveTheme();
        if (mermaidTheme !== theme) {
            global.mermaid.initialize({
                startOnLoad: false,
                securityLevel: 'strict',
                theme: theme === 'dark' ? 'dark' : 'default',
                fontFamily: getComputedStyle(document.documentElement)
                    .getPropertyValue('--font-sans').trim() || undefined
            });
            mermaidTheme = theme;
        }

        seq += 1;
        return global.mermaid.render('adrscope-mermaid-' + seq, source).then(function(result) {
            var holder = document.createElement('div');
            holder.innerHTML = result.svg;
            var svg = holder.querySelector('svg');
            if (!svg) throw new Error('mermaid returned no SVG');
            sizeFromViewBox(svg);
            canvas.stage.appendChild(svg);
            if (result.bindFunctions) result.bindFunctions(canvas.stage);
        });
    }

    function renderDrawio(source, canvas) {
        if (!global.GraphViewer) throw new Error('draw.io viewer is not embedded');

        var host = document.createElement('div');
        host.className = 'mxgraph';
        // GraphViewer measures its container and renders nothing at zero width.
        host.style.width = Math.max(320, canvas.width()) + 'px';
        host.setAttribute('data-mxgraph', JSON.stringify({
            highlight: '#3b82f6',
            lightbox: false,
            nav: false,
            resize: true,
            toolbar: null,
            xml: source
        }));

        canvas.stage.appendChild(host);
        global.GraphViewer.createViewerForElement(host);
        return Promise.resolve();
    }

    function whenExcalidrawReady() {
        if (global.ExcalidrawUtils) return Promise.resolve(global.ExcalidrawUtils);

        // The bundle is an ES module, so it evaluates after the classic scripts.
        return new Promise(function(resolve, reject) {
            var timer = global.setTimeout(function() {
                reject(new Error('excalidraw renderer is not embedded'));
            }, 15000);

            global.addEventListener('adrscope:excalidraw-ready', function() {
                global.clearTimeout(timer);
                resolve(global.ExcalidrawUtils);
            }, { once: true });
        });
    }

    function renderExcalidraw(source, canvas) {
        var scene = JSON.parse(source);

        return whenExcalidrawReady().then(function(api) {
            return api.exportToSvg({
                data: {
                    elements: scene.elements || [],
                    files: scene.files || null,
                    appState: scene.appState || {}
                },
                config: {
                    padding: 16,
                    theme: resolveTheme()
                }
            });
        }).then(function(svg) {
            sizeFromViewBox(svg);
            canvas.stage.appendChild(svg);
        });
    }

    var RENDERERS = {
        mermaid: renderMermaid,
        drawio: renderDrawio,
        excalidraw: renderExcalidraw
    };

    // =========================================================================
    // Entry points
    // =========================================================================

    /**
     * Replaces every recognised diagram code block inside `root` with a canvas.
     * Resolves once all of them have settled; individual failures degrade to an
     * error note plus the original source rather than rejecting.
     */
    function enhance(root) {
        if (!root) return Promise.resolve();

        var theme = resolveTheme();
        var jobs = [];

        KINDS.forEach(function(kind) {
            var blocks = root.querySelectorAll('pre > code.language-' + kind);

            Array.prototype.forEach.call(blocks, function(code) {
                var pre = code.parentNode;
                // textContent undoes the entity escaping the markdown renderer
                // applied, which draw.io XML and mermaid both need verbatim.
                var source = code.textContent;
                var canvas = makeCanvas(kind);
                pre.parentNode.replaceChild(canvas.frame, pre);

                var key = kind + '\u0000' + theme + '\u0000' + source;
                var cached = cache.get(key);
                if (cached !== undefined) {
                    canvas.stage.innerHTML = cached;
                    canvas.fit();
                    return;
                }

                var job;
                try {
                    job = RENDERERS[kind](source, canvas);
                } catch (e) {
                    showError(canvas, kind, e);
                    showSource(canvas, source);
                    return;
                }

                jobs.push(Promise.resolve(job).then(function() {
                    canvas.fit();
                    cache.set(key, canvas.stage.innerHTML);
                }, function(e) {
                    showError(canvas, kind, e);
                    showSource(canvas, source);
                }));
            });
        });

        return Promise.all(jobs);
    }

    /** Drops cached renders, e.g. after a theme switch. */
    function reset() {
        cache.clear();
        mermaidTheme = null;
    }

    global.ADRScopeDiagrams = { enhance: enhance, reset: reset };
})(window);
