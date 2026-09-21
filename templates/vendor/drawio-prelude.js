/**
 * Offline configuration for the draw.io viewer.
 *
 * Every path below defaults to a viewer.diagrams.net URL, so it has to be
 * pinned before the bundle evaluates or the generated HTML stops being
 * self-contained. The math URL needs a body that stays valid JavaScript once
 * the loader appends "/startup.js" to it, hence the trailing comment marker.
 */
window.PROXY_URL = '.';
window.STYLE_PATH = '.';
window.SHAPES_PATH = '.';
window.STENCIL_PATH = '.';
window.GRAPH_IMAGE_PATH = '.';
window.mxImageBasePath = '.';
window.mxBasePath = '.';
window.DRAW_MATH_URL = 'data:text/javascript,//';
window.mxLoadStylesheets = false;
window.mxLoadResources = false;
window.mxForceIncludes = false;
