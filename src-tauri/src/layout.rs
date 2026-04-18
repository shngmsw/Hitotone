use crate::state::{
    AppState, LayoutNode, LayoutTree, Pane, PaneId, PaneKind, SplitDirection, SplitSize,
};
use std::sync::Arc;

pub const DOCK_WIDTH: f32 = 64.0;
pub const TITLE_BAR_HEIGHT: f32 = 32.0;
#[allow(dead_code)]
pub const MIN_SERVICE_WIDTH: f32 = 100.0;
/// Gap between service webview right edge and AI webview left edge.
/// Exposes index.html's #resize-handle which is blocked by child webviews otherwise.
pub const RESIZE_GAP: f32 = 8.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// Derive a LayoutTree from flat AppState fields.
/// Flat fields remain authoritative; this is rebuilt each call.
pub fn build_tree_from_state(state: &AppState) -> LayoutTree {
    let chrome = LayoutNode::Leaf(Pane {
        id: PaneId("chrome".into()),
        kind: PaneKind::Chrome,
        webview_label: "chrome".into(),
        visible: true,
    });

    let service = LayoutNode::Leaf(Pane {
        id: PaneId(format!("service:{}", state.active_service_id)),
        kind: PaneKind::Service(state.active_service_id.clone()),
        webview_label: if state.active_service_id.is_empty() {
            String::new()
        } else {
            format!("service-{}", state.active_service_id)
        },
        visible: true,
    });

    let ai_visible = state.show_ai_companion && state.ai_webview_created;
    let ai = LayoutNode::Leaf(Pane {
        id: PaneId("ai".into()),
        kind: PaneKind::AiCompanion,
        webview_label: "ai-webview".into(),
        visible: ai_visible,
    });

    let ai_size = if state.show_ai_companion {
        SplitSize::Fixed((state.ai_width as f32).max(0.0))
    } else {
        SplitSize::Fixed(0.0)
    };

    let inner = LayoutNode::Split {
        direction: SplitDirection::Horizontal,
        sizes: vec![SplitSize::Fixed(DOCK_WIDTH), SplitSize::Flex(1.0), ai_size],
        children: vec![chrome, service, ai],
    };

    // Wrap in vertical split: [titlebar Fixed(32) | content Flex(1)]
    // Titlebar pane has empty webview_label — no child webview (it's a separate overlay).
    let titlebar_pane = LayoutNode::Leaf(Pane {
        id: PaneId("_titlebar".into()),
        kind: PaneKind::Chrome,
        webview_label: String::new(),
        visible: false,
    });

    Arc::new(LayoutNode::Split {
        direction: SplitDirection::Vertical,
        sizes: vec![SplitSize::Fixed(TITLE_BAR_HEIGHT), SplitSize::Flex(1.0)],
        children: vec![titlebar_pane, inner],
    })
}

/// Compute rects for all leaf panes from a layout tree and viewport.
/// Pure function — no Tauri dependency.
pub fn compute_rects(node: &LayoutNode, viewport: Rect) -> Vec<(PaneId, Rect)> {
    match node {
        LayoutNode::Leaf(pane) => vec![(pane.id.clone(), viewport)],
        LayoutNode::Split {
            direction,
            sizes,
            children,
        } => {
            let count = children.len();
            if count == 0 || sizes.len() != count {
                return vec![];
            }

            let total = match direction {
                SplitDirection::Horizontal => viewport.width,
                SplitDirection::Vertical => viewport.height,
            };

            let fixed_total: f32 = sizes
                .iter()
                .map(|s| match s {
                    SplitSize::Fixed(v) => *v,
                    SplitSize::Flex(_) => 0.0,
                })
                .sum();

            let flex_total: f32 = sizes
                .iter()
                .map(|s| match s {
                    SplitSize::Fixed(_) => 0.0,
                    SplitSize::Flex(w) => *w,
                })
                .sum();

            let remaining = (total - fixed_total).max(0.0);

            let mut offset = match direction {
                SplitDirection::Horizontal => viewport.x,
                SplitDirection::Vertical => viewport.y,
            };

            let mut result = Vec::new();
            for (i, child) in children.iter().enumerate() {
                let size = match sizes[i] {
                    SplitSize::Fixed(v) => v,
                    SplitSize::Flex(w) => {
                        if flex_total > 0.0 {
                            (w / flex_total * remaining).max(0.0)
                        } else {
                            0.0
                        }
                    }
                };

                let child_rect = match direction {
                    SplitDirection::Horizontal => Rect {
                        x: offset,
                        y: viewport.y,
                        width: size,
                        height: viewport.height,
                    },
                    SplitDirection::Vertical => Rect {
                        x: viewport.x,
                        y: offset,
                        width: viewport.width,
                        height: size,
                    },
                };

                result.extend(compute_rects(child, child_rect));
                offset += size;
            }
            result
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::AppState;

    fn three_zone_viewport() -> Rect {
        Rect {
            x: 0.0,
            y: 0.0,
            width: 1400.0,
            height: 900.0,
        }
    }

    fn make_state(show_ai: bool, ai_width: u32, ai_created: bool) -> AppState {
        let mut s = AppState::default();
        s.active_service_id = "svc1".into();
        s.show_ai_companion = show_ai;
        s.ai_width = ai_width;
        s.ai_webview_created = ai_created;
        s
    }

    #[test]
    fn three_zone_with_ai() {
        let state = make_state(true, 400, true);
        let tree = build_tree_from_state(&state);
        let rects = compute_rects(&tree, three_zone_viewport());

        let tb = TITLE_BAR_HEIGHT;
        let content_h = 900.0 - tb;

        let chrome = rects.iter().find(|(id, _)| id.0 == "chrome").unwrap();
        let service = rects.iter().find(|(id, _)| id.0 == "service:svc1").unwrap();
        let ai = rects.iter().find(|(id, _)| id.0 == "ai").unwrap();

        assert_eq!(
            chrome.1,
            Rect {
                x: 0.0,
                y: tb,
                width: 64.0,
                height: content_h
            }
        );
        assert_eq!(
            service.1,
            Rect {
                x: 64.0,
                y: tb,
                width: 936.0,
                height: content_h
            }
        );
        assert_eq!(
            ai.1,
            Rect {
                x: 1000.0,
                y: tb,
                width: 400.0,
                height: content_h
            }
        );
    }

    #[test]
    fn three_zone_ai_hidden() {
        let state = make_state(false, 400, true);
        let tree = build_tree_from_state(&state);
        let rects = compute_rects(&tree, three_zone_viewport());

        let service = rects.iter().find(|(id, _)| id.0 == "service:svc1").unwrap();
        let ai = rects.iter().find(|(id, _)| id.0 == "ai").unwrap();

        assert_eq!(service.1.x, 64.0);
        assert_eq!(service.1.width, 1336.0);
        assert_eq!(ai.1.width, 0.0);
    }

    #[test]
    fn vertical_split() {
        use crate::state::{LayoutNode, Pane, PaneId, PaneKind, SplitDirection, SplitSize};
        let top = LayoutNode::Leaf(Pane {
            id: PaneId("top".into()),
            kind: PaneKind::Chrome,
            webview_label: "top".into(),
            visible: true,
        });
        let bottom = LayoutNode::Leaf(Pane {
            id: PaneId("bot".into()),
            kind: PaneKind::Chrome,
            webview_label: "bot".into(),
            visible: true,
        });
        let node = LayoutNode::Split {
            direction: SplitDirection::Vertical,
            sizes: vec![SplitSize::Flex(1.0), SplitSize::Flex(1.0)],
            children: vec![top, bottom],
        };
        let vp = Rect {
            x: 0.0,
            y: 0.0,
            width: 800.0,
            height: 600.0,
        };
        let rects = compute_rects(&node, vp);

        let t = rects.iter().find(|(id, _)| id.0 == "top").unwrap();
        let b = rects.iter().find(|(id, _)| id.0 == "bot").unwrap();
        assert_eq!(
            t.1,
            Rect {
                x: 0.0,
                y: 0.0,
                width: 800.0,
                height: 300.0
            }
        );
        assert_eq!(
            b.1,
            Rect {
                x: 0.0,
                y: 300.0,
                width: 800.0,
                height: 300.0
            }
        );
    }

    #[test]
    fn build_tree_labels_and_visibility() {
        let state = make_state(true, 300, true);
        let tree = build_tree_from_state(&state);
        let rects = compute_rects(&tree, three_zone_viewport());

        // 4 panes: _titlebar + chrome + service + ai
        assert_eq!(rects.len(), 4);

        // Root is vertical split [titlebar | inner_horizontal]
        if let crate::state::LayoutNode::Split { children, .. } = tree.as_ref() {
            let inner = if let LayoutNode::Split { children, .. } = &children[1] {
                children
            } else {
                panic!()
            };
            let chrome_pane = if let LayoutNode::Leaf(p) = &inner[0] {
                p
            } else {
                panic!()
            };
            assert_eq!(chrome_pane.webview_label, "chrome");
            let svc_pane = if let LayoutNode::Leaf(p) = &inner[1] {
                p
            } else {
                panic!()
            };
            assert_eq!(svc_pane.webview_label, "service-svc1");
            let ai_pane = if let LayoutNode::Leaf(p) = &inner[2] {
                p
            } else {
                panic!()
            };
            assert_eq!(ai_pane.webview_label, "ai-webview");
            assert!(ai_pane.visible);
        } else {
            panic!("expected Split at root");
        }
    }
}
