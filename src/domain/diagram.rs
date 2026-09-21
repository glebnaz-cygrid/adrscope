//! Diagram formats that can be embedded in ADR bodies.
//!
//! ADR authors write diagrams as fenced code blocks. The markdown renderer
//! turns those into `<pre><code class="language-*">`, and the viewer's
//! JavaScript layer upgrades them into interactive canvases. This module owns
//! the vocabulary shared by both sides.

use std::collections::BTreeSet;

/// A diagram format the HTML viewer can render inline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DiagramKind {
    /// Mermaid source, written as a ```` ```mermaid ```` block.
    Mermaid,
    /// draw.io / diagrams.net `mxfile` XML, written as a ```` ```drawio ```` block.
    Drawio,
    /// Excalidraw scene JSON, written as a ```` ```excalidraw ```` block.
    Excalidraw,
}

impl DiagramKind {
    /// Every supported diagram kind.
    pub const ALL: [Self; 3] = [Self::Mermaid, Self::Drawio, Self::Excalidraw];

    /// Returns the markdown fence info string that selects this kind.
    #[must_use]
    pub const fn fence(self) -> &'static str {
        match self {
            Self::Mermaid => "mermaid",
            Self::Drawio => "drawio",
            Self::Excalidraw => "excalidraw",
        }
    }

    /// Parses a markdown fence info string.
    #[must_use]
    pub fn from_fence(fence: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|kind| kind.fence().eq_ignore_ascii_case(fence))
    }

    /// Returns the attribute the markdown renderer emits for this kind.
    #[must_use]
    pub const fn html_marker(self) -> &'static str {
        match self {
            Self::Mermaid => "class=\"language-mermaid\"",
            Self::Drawio => "class=\"language-drawio\"",
            Self::Excalidraw => "class=\"language-excalidraw\"",
        }
    }

    /// Reports whether pre-rendered HTML contains at least one such block.
    #[must_use]
    pub fn occurs_in(self, html: &str) -> bool {
        html.contains(self.html_marker())
    }

    /// Collects the kinds used across the given pre-rendered ADR bodies.
    pub fn detect<'a, I: IntoIterator<Item = &'a str>>(bodies: I) -> BTreeSet<Self> {
        let mut found = BTreeSet::new();

        for body in bodies {
            for kind in Self::ALL {
                if kind.occurs_in(body) {
                    found.insert(kind);
                }
            }
            if found.len() == Self::ALL.len() {
                break;
            }
        }

        found
    }
}

impl std::fmt::Display for DiagramKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.fence())
    }
}

/// Which diagram renderers to embed in a generated viewer.
///
/// Each renderer is a multi-megabyte JavaScript bundle, so the default only
/// embeds the ones the ADRs actually need.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum DiagramSupport {
    /// Embed only the renderers the ADRs use.
    #[default]
    Auto,
    /// Embed every renderer, whether used or not.
    All,
    /// Embed nothing; diagram blocks stay as plain code.
    None,
    /// Embed exactly the given renderers.
    Only(BTreeSet<DiagramKind>),
}

impl DiagramSupport {
    /// Resolves the renderers to embed for the given pre-rendered ADR bodies.
    #[must_use]
    pub fn resolve<'a, I: IntoIterator<Item = &'a str>>(&self, bodies: I) -> BTreeSet<DiagramKind> {
        match self {
            Self::Auto => DiagramKind::detect(bodies),
            Self::All => DiagramKind::ALL.into_iter().collect(),
            Self::None => BTreeSet::new(),
            Self::Only(kinds) => kinds.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fence_round_trip() {
        for kind in DiagramKind::ALL {
            assert_eq!(DiagramKind::from_fence(kind.fence()), Some(kind));
        }
    }

    #[test]
    fn test_from_fence_is_case_insensitive() {
        assert_eq!(
            DiagramKind::from_fence("Mermaid"),
            Some(DiagramKind::Mermaid)
        );
        assert_eq!(DiagramKind::from_fence("DRAWIO"), Some(DiagramKind::Drawio));
    }

    #[test]
    fn test_from_fence_rejects_unknown() {
        assert_eq!(DiagramKind::from_fence("rust"), None);
        assert_eq!(DiagramKind::from_fence(""), None);
    }

    #[test]
    fn test_detect_finds_each_kind() {
        let tests = [
            (
                "<pre><code class=\"language-mermaid\">a</code></pre>",
                Some(DiagramKind::Mermaid),
            ),
            (
                "<pre><code class=\"language-drawio\">a</code></pre>",
                Some(DiagramKind::Drawio),
            ),
            (
                "<pre><code class=\"language-excalidraw\">a</code></pre>",
                Some(DiagramKind::Excalidraw),
            ),
            ("<pre><code class=\"language-rust\">a</code></pre>", None),
            ("<p>mermaid is mentioned in prose</p>", None),
        ];

        for (html, expected) in tests {
            let found = DiagramKind::detect([html]);
            assert_eq!(found.into_iter().next(), expected, "html: {html}");
        }
    }

    #[test]
    fn test_detect_across_multiple_bodies() {
        let found = DiagramKind::detect([
            "<code class=\"language-drawio\">",
            "<p>no diagrams</p>",
            "<code class=\"language-mermaid\">",
        ]);

        assert_eq!(found.len(), 2);
        assert!(found.contains(&DiagramKind::Mermaid));
        assert!(found.contains(&DiagramKind::Drawio));
    }

    #[test]
    fn test_detect_empty_input() {
        assert!(DiagramKind::detect(std::iter::empty()).is_empty());
    }

    #[test]
    fn test_support_auto_follows_content() {
        let resolved = DiagramSupport::Auto.resolve(["<code class=\"language-mermaid\">"]);
        assert_eq!(
            resolved.into_iter().collect::<Vec<_>>(),
            vec![DiagramKind::Mermaid]
        );
    }

    #[test]
    fn test_support_all_ignores_content() {
        let resolved = DiagramSupport::All.resolve(["<p>nothing</p>"]);
        assert_eq!(resolved.len(), DiagramKind::ALL.len());
    }

    #[test]
    fn test_support_none_embeds_nothing() {
        let resolved = DiagramSupport::None.resolve(["<code class=\"language-mermaid\">"]);
        assert!(resolved.is_empty());
    }

    #[test]
    fn test_support_only_is_explicit() {
        let support = DiagramSupport::Only(std::iter::once(DiagramKind::Drawio).collect());
        let resolved = support.resolve(["<code class=\"language-mermaid\">"]);
        assert_eq!(
            resolved.into_iter().collect::<Vec<_>>(),
            vec![DiagramKind::Drawio]
        );
    }

    #[test]
    fn test_display() {
        assert_eq!(DiagramKind::Excalidraw.to_string(), "excalidraw");
    }
}
