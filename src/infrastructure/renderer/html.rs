//! HTML viewer generation using askama templates.

use std::collections::BTreeSet;

use askama::Template;
use serde::Serialize;
use time::OffsetDateTime;

use crate::domain::{Adr, DiagramKind, DiagramSupport, Facets, Graph};
use crate::error::{Error, Result};

const DIAGRAMS_CSS: &str = include_str!("../../../templates/diagrams.css");
const DIAGRAMS_JS: &str = include_str!("../../../templates/diagrams.js");
const MERMAID_JS: &str = include_str!("../../../templates/vendor/mermaid.min.js");
const DRAWIO_PRELUDE_JS: &str = include_str!("../../../templates/vendor/drawio-prelude.js");
const DRAWIO_VIEWER_JS: &str = include_str!("../../../templates/vendor/drawio-viewer.min.js");
const EXCALIDRAW_JS: &str = include_str!("../../../templates/vendor/excalidraw-utils.min.js");

/// Theme for the HTML viewer.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Theme {
    /// Light theme.
    Light,
    /// Dark theme.
    Dark,
    /// Auto (follows system preference).
    #[default]
    Auto,
}

impl Theme {
    /// Returns the theme as a string for use in templates.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Light => "light",
            Self::Dark => "dark",
            Self::Auto => "auto",
        }
    }
}

impl std::str::FromStr for Theme {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "light" => Ok(Self::Light),
            "dark" => Ok(Self::Dark),
            "auto" => Ok(Self::Auto),
            _ => Err(format!("invalid theme: {s}")),
        }
    }
}

/// Configuration for HTML rendering.
#[derive(Debug, Clone, Default)]
pub struct RenderConfig {
    /// Page title.
    pub title: String,
    /// Theme preference.
    pub theme: Theme,
    /// Whether to embed all assets inline.
    pub embed_assets: bool,
    /// Which diagram renderers to embed.
    pub diagrams: DiagramSupport,
}

impl RenderConfig {
    /// Creates a new render configuration with the given title.
    #[must_use]
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            theme: Theme::default(),
            embed_assets: true,
            diagrams: DiagramSupport::default(),
        }
    }

    /// Sets the theme.
    #[must_use]
    pub const fn with_theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }

    /// Sets which diagram renderers to embed.
    #[must_use]
    pub fn with_diagrams(mut self, diagrams: DiagramSupport) -> Self {
        self.diagrams = diagrams;
        self
    }
}

/// Inlines one script, neutralising any sequence that would close the element
/// early. Vendored bundles are third-party code and get re-fetched on version
/// bumps, so this cannot rely on them staying free of `</script`.
fn push_script(out: &mut String, module: bool, body: &str) {
    out.push_str(if module {
        "<script type=\"module\">\n"
    } else {
        "<script>\n"
    });
    out.push_str(&body.replace("</script", "<\\/script"));
    out.push_str("\n</script>\n");
}

/// Builds the `<script>` blocks for the requested diagram renderers.
///
/// Returns an empty string when nothing is requested, so viewers for ADRs
/// without diagrams stay as small as they were.
fn diagram_scripts(kinds: &BTreeSet<DiagramKind>) -> String {
    if kinds.is_empty() {
        return String::new();
    }

    let mut out = String::with_capacity(
        MERMAID_JS.len() + DRAWIO_VIEWER_JS.len() + EXCALIDRAW_JS.len() + DIAGRAMS_JS.len(),
    );

    for kind in kinds {
        match kind {
            DiagramKind::Mermaid => push_script(&mut out, false, MERMAID_JS),
            DiagramKind::Drawio => {
                // The prelude pins every resource path before the bundle reads it.
                push_script(&mut out, false, DRAWIO_PRELUDE_JS);
                push_script(&mut out, false, DRAWIO_VIEWER_JS);
            },
            // Excalidraw ships as an ES module, so it evaluates after the
            // classic scripts and announces itself with an event.
            DiagramKind::Excalidraw => push_script(&mut out, true, EXCALIDRAW_JS),
        }
    }

    push_script(&mut out, false, DIAGRAMS_JS);
    out
}

/// Data structure embedded in the HTML for JavaScript consumption.
#[derive(Debug, Clone, Serialize)]
pub struct ViewerData {
    /// Metadata about the generation.
    pub meta: ViewerMeta,
    /// All parsed ADRs.
    pub records: Vec<Adr>,
    /// Faceted filter data.
    pub facets: Facets,
    /// Relationship graph.
    pub graph: Graph,
}

/// Metadata embedded in the viewer.
#[derive(Debug, Clone, Serialize)]
pub struct ViewerMeta {
    /// When the viewer was generated.
    pub generated: String,
    /// Generator name and version.
    pub generator: String,
    /// Schema version.
    pub schema_version: String,
    /// Source directory.
    pub source_dir: String,
}

impl ViewerMeta {
    /// Creates metadata for the current generation.
    #[must_use]
    pub fn new(source_dir: impl Into<String>) -> Self {
        Self {
            generated: OffsetDateTime::now_utc()
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap_or_else(|_| "unknown".to_string()),
            generator: format!("adrscope/{}", env!("CARGO_PKG_VERSION")),
            schema_version: "1.0.0".to_string(),
            source_dir: source_dir.into(),
        }
    }
}

/// The main HTML viewer template.
#[derive(Template)]
#[template(path = "viewer.html", escape = "none")]
pub struct ViewerTemplate<'a> {
    /// Page title.
    pub title: &'a str,
    /// Theme preference.
    pub theme: &'a str,
    /// Serialized JSON data for embedding.
    pub data_json: &'a str,
    /// Embedded CSS.
    pub css: &'a str,
    /// Embedded JavaScript.
    pub js: &'a str,
    /// Embedded diagram styles.
    pub diagrams_css: &'a str,
    /// Embedded diagram renderer scripts, empty when none are needed.
    pub diagram_scripts: &'a str,
}

/// HTML renderer for generating self-contained viewers.
#[derive(Debug, Clone, Default)]
pub struct HtmlRenderer;

impl HtmlRenderer {
    /// Creates a new HTML renderer.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Renders a collection of ADRs to a self-contained HTML viewer.
    pub fn render(
        &self,
        adrs: Vec<Adr>,
        source_dir: &str,
        config: &RenderConfig,
    ) -> Result<String> {
        let kinds = config
            .diagrams
            .resolve(adrs.iter().map(crate::domain::Adr::body_html));
        let scripts = diagram_scripts(&kinds);

        // Build the embedded data
        let data = ViewerData {
            meta: ViewerMeta::new(source_dir),
            facets: Facets::from_adrs(&adrs),
            graph: Graph::from_adrs(&adrs),
            records: adrs,
        };

        // Serialize to JSON
        let data_json =
            serde_json::to_string(&data).map_err(|e| Error::JsonSerialize(e.to_string()))?;

        // Render the template
        let template = ViewerTemplate {
            title: &config.title,
            theme: config.theme.as_str(),
            data_json: &data_json,
            css: include_str!("../../../templates/styles.css"),
            js: include_str!("../../../templates/app.js"),
            diagrams_css: DIAGRAMS_CSS,
            diagram_scripts: &scripts,
        };

        template.render().map_err(Error::from)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::domain::{AdrId, Frontmatter};

    fn adr_with_body(id: &str, body_html: &str) -> Adr {
        Adr::new(
            AdrId::new(id),
            format!("{id}.md"),
            PathBuf::from(format!("{id}.md")),
            Frontmatter::new("Test"),
            String::new(),
            body_html.to_string(),
            String::new(),
        )
    }

    fn render_with(body_html: &str, diagrams: DiagramSupport) -> String {
        let config = RenderConfig::new("Test").with_diagrams(diagrams);
        HtmlRenderer::new()
            .render(vec![adr_with_body("adr-0001", body_html)], "docs", &config)
            .expect("render should succeed")
    }

    #[test]
    fn test_theme_from_str() {
        assert_eq!("light".parse::<Theme>().ok(), Some(Theme::Light));
        assert_eq!("DARK".parse::<Theme>().ok(), Some(Theme::Dark));
        assert_eq!("Auto".parse::<Theme>().ok(), Some(Theme::Auto));
        assert!("invalid".parse::<Theme>().is_err());
    }

    #[test]
    fn test_theme_as_str() {
        assert_eq!(Theme::Light.as_str(), "light");
        assert_eq!(Theme::Dark.as_str(), "dark");
        assert_eq!(Theme::Auto.as_str(), "auto");
    }

    #[test]
    fn test_render_config_builder() {
        let config = RenderConfig::new("My ADRs").with_theme(Theme::Dark);

        assert_eq!(config.title, "My ADRs");
        assert_eq!(config.theme, Theme::Dark);
    }

    #[test]
    fn test_viewer_meta_creation() {
        let meta = ViewerMeta::new("docs/decisions");

        assert!(meta.generated.contains("T")); // ISO 8601 format
        assert!(meta.generator.starts_with("adrscope/"));
        assert_eq!(meta.schema_version, "1.0.0");
        assert_eq!(meta.source_dir, "docs/decisions");
    }
    // Markers unique to each vendored bundle; the diagram layer itself mentions
    // all three renderers, so it cannot be used to detect them.
    const MERMAID_MARKER: &str = "__esbuild_esm_mermaid_nm";
    const DRAWIO_MARKER: &str = "mxStencilRegistry";
    const EXCALIDRAW_MARKER: &str = "<script type=\"module\">";
    const LAYER_MARKER: &str = "ADRScope Diagram Layer";

    #[test]
    fn test_diagram_bundles_are_embedded_only_when_used() {
        let tests = [
            ("<p>no diagrams here</p>", false, false, false),
            (
                "<code class=\"language-mermaid\">a</code>",
                true,
                false,
                false,
            ),
            (
                "<code class=\"language-drawio\">a</code>",
                false,
                true,
                false,
            ),
            (
                "<code class=\"language-excalidraw\">a</code>",
                false,
                false,
                true,
            ),
            (
                "<code class=\"language-mermaid\">a</code><code class=\"language-drawio\">b</code>",
                true,
                true,
                false,
            ),
        ];

        for (body, wants_mermaid, wants_drawio, wants_excalidraw) in tests {
            let html = render_with(body, DiagramSupport::Auto);

            assert_eq!(
                html.contains(MERMAID_MARKER),
                wants_mermaid,
                "mermaid, body: {body}"
            );
            assert_eq!(
                html.contains(DRAWIO_MARKER),
                wants_drawio,
                "drawio, body: {body}"
            );
            assert_eq!(
                html.contains(EXCALIDRAW_MARKER),
                wants_excalidraw,
                "excalidraw, body: {body}"
            );
        }
    }

    #[test]
    fn test_diagram_layer_ships_with_any_renderer() {
        let with_diagram = render_with(
            "<code class=\"language-mermaid\">a</code>",
            DiagramSupport::Auto,
        );
        let without = render_with("<p>plain</p>", DiagramSupport::Auto);

        assert!(with_diagram.contains(LAYER_MARKER));
        assert!(!without.contains(LAYER_MARKER));
    }

    #[test]
    fn test_diagram_support_none_skips_every_bundle() {
        let html = render_with(
            "<code class=\"language-mermaid\">a</code>",
            DiagramSupport::None,
        );

        assert!(!html.contains(LAYER_MARKER));
        assert!(!html.contains(MERMAID_MARKER));
    }

    #[test]
    fn test_diagram_support_all_embeds_every_bundle() {
        let html = render_with("<p>plain</p>", DiagramSupport::All);

        assert!(html.contains(MERMAID_MARKER));
        assert!(html.contains(DRAWIO_MARKER));
        assert!(html.contains(EXCALIDRAW_MARKER));
    }

    #[test]
    fn test_diagram_support_only_embeds_the_named_renderer() {
        let html = render_with(
            "<code class=\"language-mermaid\">a</code>",
            DiagramSupport::Only(std::iter::once(DiagramKind::Drawio).collect()),
        );

        assert!(html.contains(DRAWIO_MARKER));
        assert!(!html.contains(MERMAID_MARKER));
    }

    #[test]
    fn test_diagram_styles_are_always_present() {
        assert!(render_with("<p>plain</p>", DiagramSupport::None).contains(".diagram-viewport"));
    }

    #[test]
    fn test_push_script_neutralises_closing_tag() {
        let mut out = String::new();
        push_script(&mut out, false, "var s = \"</script>\";");

        assert!(!out.contains("\"</script>\""));
        assert!(out.contains("<\\/script"));
        assert!(out.ends_with("</script>\n"));
    }

    #[test]
    fn test_push_script_marks_modules() {
        let mut out = String::new();
        push_script(&mut out, true, "export {};");

        assert!(out.starts_with("<script type=\"module\">"));
    }
}
