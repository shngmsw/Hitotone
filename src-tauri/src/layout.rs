use crate::state::{
    AppState, LayoutNode, LayoutPreset, LayoutTree, Pane, PaneId, PaneKind, SplitDirection,
    SplitSize,
};
use std::sync::Arc;

pub const DOCK_WIDTH: f32 = 64.0;
pub const TITLE_BAR_HEIGHT: f32 = 32.0;
pub const MIN_SERVICE_WIDTH: f32 = 100.0;
/// Gap between service webview right edge and AI webview left edge.
/// Exposes index.html's #resize-handle which is blocked by child webviews otherwise.
pub const RESIZE_GAP: f32 = 8.0;
/// Width/height of split divider elements in the DOM (.split-divider-h / -v).
pub const DIVIDER_SIZE: f32 = 4.0;

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
/// Accounts for DIVIDER_SIZE pixels of space between each child pair (matching DOM flex layout).
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

            let divider_total = DIVIDER_SIZE * (count as f32 - 1.0);
            let remaining = (total - fixed_total - divider_total).max(0.0);

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
                if i < count - 1 {
                    offset += DIVIDER_SIZE;
                }
            }
            result
        }
    }
}

// ========================================
// Service zone rect (Phase 3)
// ========================================

/// Compute the rect of the service area (viewport minus titlebar, dock, AI panel, gap).
pub fn compute_service_zone_rect(viewport: Rect, state: &AppState) -> Rect {
    let ai_w = if state.show_ai_companion {
        state.ai_width as f32
    } else {
        0.0
    };
    let ai_gap = if state.show_ai_companion {
        RESIZE_GAP
    } else {
        0.0
    };
    Rect {
        x: DOCK_WIDTH,
        y: TITLE_BAR_HEIGHT,
        width: (viewport.width - DOCK_WIDTH - ai_w - ai_gap).max(MIN_SERVICE_WIDTH),
        height: viewport.height - TITLE_BAR_HEIGHT,
    }
}

// ========================================
// Tree mutation helpers (Phase 3)
// ========================================

/// Find a pane by id anywhere in the tree.
pub fn find_pane<'a>(node: &'a LayoutNode, id: &PaneId) -> Option<&'a Pane> {
    match node {
        LayoutNode::Leaf(pane) => {
            if &pane.id == id {
                Some(pane)
            } else {
                None
            }
        }
        LayoutNode::Split { children, .. } => {
            children.iter().find_map(|child| find_pane(child, id))
        }
    }
}

/// Collect all non-empty service IDs in the tree.
pub fn collect_service_ids(node: &LayoutNode) -> Vec<String> {
    match node {
        LayoutNode::Leaf(pane) => {
            if let PaneKind::Service(id) = &pane.kind {
                if !id.is_empty() {
                    vec![id.clone()]
                } else {
                    vec![]
                }
            } else {
                vec![]
            }
        }
        LayoutNode::Split { children, .. } => {
            children.iter().flat_map(collect_service_ids).collect()
        }
    }
}

/// Split the target leaf into a Split containing the original leaf and a new leaf.
/// Returns a new tree. The new leaf is appended as the second child.
pub fn split_pane_in_tree(
    tree: &LayoutNode,
    target: &PaneId,
    dir: SplitDirection,
    new_pane: Pane,
) -> Result<LayoutNode, String> {
    match tree {
        LayoutNode::Leaf(pane) => {
            if &pane.id == target {
                Ok(LayoutNode::Split {
                    direction: dir,
                    sizes: vec![SplitSize::Flex(1.0), SplitSize::Flex(1.0)],
                    children: vec![LayoutNode::Leaf(pane.clone()), LayoutNode::Leaf(new_pane)],
                })
            } else {
                Err(format!("pane '{}' not found", target.0))
            }
        }
        LayoutNode::Split {
            direction,
            sizes,
            children,
        } => {
            let mut new_children = Vec::with_capacity(children.len());
            let mut found = false;
            for child in children {
                if found {
                    new_children.push(child.clone());
                    continue;
                }
                match split_pane_in_tree(child, target, dir, new_pane.clone()) {
                    Ok(new_child) => {
                        found = true;
                        new_children.push(new_child);
                    }
                    Err(_) => {
                        new_children.push(child.clone());
                    }
                }
            }
            if found {
                Ok(LayoutNode::Split {
                    direction: *direction,
                    sizes: sizes.clone(),
                    children: new_children,
                })
            } else {
                Err(format!("pane '{}' not found", target.0))
            }
        }
    }
}

/// Remove target pane from tree. Returns (new_tree, new_focus_pane_id).
/// Collapses a Split with one remaining child into that child.
/// Errors if target is the only pane (root leaf).
pub fn close_pane_in_tree(
    tree: &LayoutNode,
    target: &PaneId,
) -> Result<(LayoutNode, Option<PaneId>), String> {
    match tree {
        LayoutNode::Leaf(pane) => {
            if &pane.id == target {
                Err("cannot close the last pane".into())
            } else {
                Err(format!("pane '{}' not found", target.0))
            }
        }
        LayoutNode::Split {
            direction,
            sizes,
            children,
        } => {
            let idx = children
                .iter()
                .position(|c| find_pane(c, target).is_some())
                .ok_or_else(|| format!("pane '{}' not found", target.0))?;

            if matches!(&children[idx], LayoutNode::Leaf(p) if &p.id == target) {
                // Direct child leaf — remove it
                let new_children: Vec<LayoutNode> = children
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| *i != idx)
                    .map(|(_, c)| c.clone())
                    .collect();
                let new_sizes: Vec<SplitSize> = sizes
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| *i != idx)
                    .map(|(_, s)| *s)
                    .collect();

                if new_children.len() == 1 {
                    let sibling = new_children.into_iter().next().unwrap();
                    let new_focus = first_pane_id(&sibling);
                    return Ok((sibling, new_focus));
                }

                let focus_idx = if idx > 0 { idx - 1 } else { 0 };
                let new_focus = first_pane_id(&new_children[focus_idx]);
                Ok((
                    LayoutNode::Split {
                        direction: *direction,
                        sizes: new_sizes,
                        children: new_children,
                    },
                    new_focus,
                ))
            } else {
                // Recurse deeper
                let (new_child, new_focus) = close_pane_in_tree(&children[idx], target)?;
                let mut new_children = children.clone();
                new_children[idx] = new_child;
                Ok((
                    LayoutNode::Split {
                        direction: *direction,
                        sizes: sizes.clone(),
                        children: new_children,
                    },
                    new_focus,
                ))
            }
        }
    }
}

/// Update sizes of the Split node reached by following path from root.
/// path = [] targets root; path = [i] targets root.children[i]; etc.
pub fn apply_split_sizes(
    tree: &LayoutNode,
    path: &[usize],
    sizes: Vec<SplitSize>,
) -> Result<LayoutNode, String> {
    match (tree, path) {
        (
            LayoutNode::Split {
                direction,
                children,
                ..
            },
            [],
        ) => {
            if sizes.len() != children.len() {
                return Err(format!(
                    "sizes length {} != children length {}",
                    sizes.len(),
                    children.len()
                ));
            }
            Ok(LayoutNode::Split {
                direction: *direction,
                sizes,
                children: children.clone(),
            })
        }
        (LayoutNode::Leaf(_), []) => Err("cannot apply sizes to a Leaf node".into()),
        (
            LayoutNode::Split {
                direction,
                sizes: old_sizes,
                children,
            },
            [idx, rest @ ..],
        ) => {
            let i = *idx;
            if i >= children.len() {
                return Err(format!(
                    "path index {} out of bounds (len {})",
                    i,
                    children.len()
                ));
            }
            let new_child = apply_split_sizes(&children[i], rest, sizes)?;
            let mut new_children = children.clone();
            new_children[i] = new_child;
            Ok(LayoutNode::Split {
                direction: *direction,
                sizes: old_sizes.clone(),
                children: new_children,
            })
        }
        (LayoutNode::Leaf(_), _) => Err("path leads to a Leaf, not a Split".into()),
    }
}

/// Pane header height rendered by PaneTreeManager (HTML overlay above the webview).
/// The webview rect is offset downward by this amount so the header is visible.
pub const PANE_HEADER_HEIGHT: f32 = 28.0;

/// Assign a service to a specific pane (mutates kind + webview_label).
pub fn assign_service_to_pane(
    tree: &LayoutNode,
    pane_id: &PaneId,
    service_id: String,
) -> Result<LayoutNode, String> {
    match tree {
        LayoutNode::Leaf(pane) => {
            if &pane.id == pane_id {
                let mut new_pane = pane.clone();
                new_pane.kind = PaneKind::Service(service_id.clone());
                new_pane.webview_label = format!("service-{}", service_id);
                Ok(LayoutNode::Leaf(new_pane))
            } else {
                Err(format!("pane '{}' not found", pane_id.0))
            }
        }
        LayoutNode::Split {
            direction,
            sizes,
            children,
        } => {
            let mut new_children = Vec::with_capacity(children.len());
            let mut found = false;
            for child in children {
                if found {
                    new_children.push(child.clone());
                    continue;
                }
                match assign_service_to_pane(child, pane_id, service_id.clone()) {
                    Ok(new_child) => {
                        found = true;
                        new_children.push(new_child);
                    }
                    Err(_) => {
                        new_children.push(child.clone());
                    }
                }
            }
            if found {
                Ok(LayoutNode::Split {
                    direction: *direction,
                    sizes: sizes.clone(),
                    children: new_children,
                })
            } else {
                Err(format!("pane '{}' not found", pane_id.0))
            }
        }
    }
}

/// サービスが削除されたとき、tree 内の該当 pane の kind を Service("") に戻す。
pub fn remove_service_from_tree(tree: &LayoutNode, service_id: &str) -> LayoutNode {
    match tree {
        LayoutNode::Leaf(pane) => {
            if let PaneKind::Service(id) = &pane.kind {
                if id == service_id {
                    let mut new_pane = pane.clone();
                    new_pane.kind = PaneKind::Service(String::new());
                    new_pane.webview_label = String::new();
                    return LayoutNode::Leaf(new_pane);
                }
            }
            LayoutNode::Leaf(pane.clone())
        }
        LayoutNode::Split {
            direction,
            sizes,
            children,
        } => LayoutNode::Split {
            direction: *direction,
            sizes: sizes.clone(),
            children: children
                .iter()
                .map(|c| remove_service_from_tree(c, service_id))
                .collect(),
        },
    }
}

pub fn first_pane_id(node: &LayoutNode) -> Option<PaneId> {
    match node {
        LayoutNode::Leaf(pane) => Some(pane.id.clone()),
        LayoutNode::Split { children, .. } => children.first().and_then(first_pane_id),
    }
}

// ========================================
// Preset layout builders (Phase 4)
// ========================================

fn empty_leaf(id: &str) -> LayoutNode {
    LayoutNode::Leaf(Pane {
        id: PaneId(id.into()),
        kind: PaneKind::Service(String::new()),
        webview_label: String::new(),
        visible: true,
    })
}

/// Build a tree skeleton for the given preset (all leaves empty).
pub fn build_tree_for_preset(preset: LayoutPreset) -> LayoutNode {
    match preset {
        LayoutPreset::Single => empty_leaf("p0"),
        LayoutPreset::TwoH => LayoutNode::Split {
            direction: SplitDirection::Horizontal,
            sizes: vec![SplitSize::Flex(1.0), SplitSize::Flex(1.0)],
            children: vec![empty_leaf("p0"), empty_leaf("p1")],
        },
        LayoutPreset::TwoV => LayoutNode::Split {
            direction: SplitDirection::Vertical,
            sizes: vec![SplitSize::Flex(1.0), SplitSize::Flex(1.0)],
            children: vec![empty_leaf("p0"), empty_leaf("p1")],
        },
        LayoutPreset::TwoByTwo => LayoutNode::Split {
            direction: SplitDirection::Horizontal,
            sizes: vec![SplitSize::Flex(1.0), SplitSize::Flex(1.0)],
            children: vec![
                LayoutNode::Split {
                    direction: SplitDirection::Vertical,
                    sizes: vec![SplitSize::Flex(1.0), SplitSize::Flex(1.0)],
                    children: vec![empty_leaf("p0"), empty_leaf("p1")],
                },
                LayoutNode::Split {
                    direction: SplitDirection::Vertical,
                    sizes: vec![SplitSize::Flex(1.0), SplitSize::Flex(1.0)],
                    children: vec![empty_leaf("p2"), empty_leaf("p3")],
                },
            ],
        },
        LayoutPreset::ThreeH => LayoutNode::Split {
            direction: SplitDirection::Horizontal,
            sizes: vec![
                SplitSize::Flex(1.0),
                SplitSize::Flex(1.0),
                SplitSize::Flex(1.0),
            ],
            children: vec![empty_leaf("p0"), empty_leaf("p1"), empty_leaf("p2")],
        },
        LayoutPreset::FourH => LayoutNode::Split {
            direction: SplitDirection::Horizontal,
            sizes: vec![
                SplitSize::Flex(1.0),
                SplitSize::Flex(1.0),
                SplitSize::Flex(1.0),
                SplitSize::Flex(1.0),
            ],
            children: vec![
                empty_leaf("p0"),
                empty_leaf("p1"),
                empty_leaf("p2"),
                empty_leaf("p3"),
            ],
        },
    }
}

/// Fill empty leaves of a preset tree with service IDs in tree-walk order.
/// Leaves that exceed service_ids length remain empty.
pub fn fill_preset_with_services(tree: LayoutNode, service_ids: &[String]) -> LayoutNode {
    let mut idx = 0usize;
    fill_services_recursive(tree, service_ids, &mut idx)
}

fn fill_services_recursive(node: LayoutNode, services: &[String], idx: &mut usize) -> LayoutNode {
    match node {
        LayoutNode::Leaf(mut pane) => {
            if *idx < services.len() {
                let svc_id = services[*idx].clone();
                *idx += 1;
                pane.kind = PaneKind::Service(svc_id.clone());
                pane.webview_label = format!("service-{}", svc_id);
            }
            LayoutNode::Leaf(pane)
        }
        LayoutNode::Split {
            direction,
            sizes,
            children,
        } => {
            let new_children = children
                .into_iter()
                .map(|c| fill_services_recursive(c, services, idx))
                .collect();
            LayoutNode::Split {
                direction,
                sizes,
                children: new_children,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{AppState, LayoutNode, Pane, PaneId, PaneKind, SplitDirection, SplitSize};

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

        // Root V-Split (2 children) adds 1 divider: inner starts at y=32+4=36, h=900-32-4=864
        // Inner H-Split (3 children) adds 2 dividers: service x=64+4=68, w=1400-64-400-8=928
        let chrome = rects.iter().find(|(id, _)| id.0 == "chrome").unwrap();
        let service = rects.iter().find(|(id, _)| id.0 == "service:svc1").unwrap();
        let ai = rects.iter().find(|(id, _)| id.0 == "ai").unwrap();

        assert_eq!(
            chrome.1,
            Rect {
                x: 0.0,
                y: 36.0,
                width: 64.0,
                height: 864.0
            }
        );
        assert_eq!(
            service.1,
            Rect {
                x: 68.0,
                y: 36.0,
                width: 928.0,
                height: 864.0
            }
        );
        assert_eq!(
            ai.1,
            Rect {
                x: 1000.0,
                y: 36.0,
                width: 400.0,
                height: 864.0
            }
        );
    }

    #[test]
    fn three_zone_ai_hidden() {
        let state = make_state(false, 400, true);
        let tree = build_tree_from_state(&state);
        let rects = compute_rects(&tree, three_zone_viewport());

        // Inner H-Split (3 children): 2 dividers → service x=68, w=1400-64-0-8=1328
        let service = rects.iter().find(|(id, _)| id.0 == "service:svc1").unwrap();
        let ai = rects.iter().find(|(id, _)| id.0 == "ai").unwrap();

        assert_eq!(service.1.x, 68.0);
        assert_eq!(service.1.width, 1328.0);
        assert_eq!(ai.1.width, 0.0);
    }

    #[test]
    fn vertical_split() {
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

        // V-Split 2 children: divider=4 → each h=(600-4)/2=298, bot y=298+4=302
        let t = rects.iter().find(|(id, _)| id.0 == "top").unwrap();
        let b = rects.iter().find(|(id, _)| id.0 == "bot").unwrap();
        assert_eq!(
            t.1,
            Rect {
                x: 0.0,
                y: 0.0,
                width: 800.0,
                height: 298.0
            }
        );
        assert_eq!(
            b.1,
            Rect {
                x: 0.0,
                y: 302.0,
                width: 800.0,
                height: 298.0
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

    fn make_leaf(id: &str, svc_id: &str) -> LayoutNode {
        LayoutNode::Leaf(Pane {
            id: PaneId(id.into()),
            kind: PaneKind::Service(svc_id.into()),
            webview_label: format!("service-{}", svc_id),
            visible: true,
        })
    }

    #[test]
    fn split_pane_creates_horizontal_split() {
        let tree = make_leaf("root", "svc1");
        let new_pane = Pane {
            id: PaneId("p2".into()),
            kind: PaneKind::Service("svc2".into()),
            webview_label: "service-svc2".into(),
            visible: true,
        };
        let result = split_pane_in_tree(
            &tree,
            &PaneId("root".into()),
            SplitDirection::Horizontal,
            new_pane,
        )
        .unwrap();
        let vp = Rect {
            x: 0.0,
            y: 0.0,
            width: 800.0,
            height: 600.0,
        };
        let rects = compute_rects(&result, vp);
        // H-Split 2 children: divider=4 → each w=(800-4)/2=398, r2 x=398+4=402
        let r1 = rects.iter().find(|(id, _)| id.0 == "root").unwrap();
        let r2 = rects.iter().find(|(id, _)| id.0 == "p2").unwrap();
        assert_eq!(
            r1.1,
            Rect {
                x: 0.0,
                y: 0.0,
                width: 398.0,
                height: 600.0
            }
        );
        assert_eq!(
            r2.1,
            Rect {
                x: 402.0,
                y: 0.0,
                width: 398.0,
                height: 600.0
            }
        );
    }

    #[test]
    fn split_pane_not_found_returns_err() {
        let tree = make_leaf("root", "svc1");
        let new_pane = Pane {
            id: PaneId("p2".into()),
            kind: PaneKind::Service("svc2".into()),
            webview_label: "service-svc2".into(),
            visible: true,
        };
        let result = split_pane_in_tree(
            &tree,
            &PaneId("nonexistent".into()),
            SplitDirection::Horizontal,
            new_pane,
        );
        assert!(result.is_err());
    }

    #[test]
    fn nested_split_rect() {
        // H of [V of [A, B], C] in 900×600
        let v = LayoutNode::Split {
            direction: SplitDirection::Vertical,
            sizes: vec![SplitSize::Flex(1.0), SplitSize::Flex(1.0)],
            children: vec![make_leaf("a", "a"), make_leaf("b", "b")],
        };
        let h = LayoutNode::Split {
            direction: SplitDirection::Horizontal,
            sizes: vec![SplitSize::Flex(1.0), SplitSize::Flex(1.0)],
            children: vec![v, make_leaf("c", "c")],
        };
        let vp = Rect {
            x: 0.0,
            y: 0.0,
            width: 900.0,
            height: 600.0,
        };
        let rects = compute_rects(&h, vp);
        // H-Split 2 children: divider=4 → each w=(900-4)/2=448, right x=452
        // V-Split 2 children: divider=4 → each h=(600-4)/2=298, bot y=302
        let ra = rects.iter().find(|(id, _)| id.0 == "a").unwrap();
        let rb = rects.iter().find(|(id, _)| id.0 == "b").unwrap();
        let rc = rects.iter().find(|(id, _)| id.0 == "c").unwrap();
        assert_eq!(
            ra.1,
            Rect {
                x: 0.0,
                y: 0.0,
                width: 448.0,
                height: 298.0
            }
        );
        assert_eq!(
            rb.1,
            Rect {
                x: 0.0,
                y: 302.0,
                width: 448.0,
                height: 298.0
            }
        );
        assert_eq!(
            rc.1,
            Rect {
                x: 452.0,
                y: 0.0,
                width: 448.0,
                height: 600.0
            }
        );
    }

    #[test]
    fn close_pane_collapses_to_sibling() {
        let tree = LayoutNode::Split {
            direction: SplitDirection::Horizontal,
            sizes: vec![SplitSize::Flex(1.0), SplitSize::Flex(1.0)],
            children: vec![make_leaf("root", "svc1"), make_leaf("p2", "svc2")],
        };
        let (new_tree, new_focus) = close_pane_in_tree(&tree, &PaneId("p2".into())).unwrap();
        assert!(matches!(&new_tree, LayoutNode::Leaf(p) if p.id == PaneId("root".into())));
        assert_eq!(new_focus, Some(PaneId("root".into())));
    }

    #[test]
    fn close_pane_last_pane_fails() {
        let tree = make_leaf("root", "svc1");
        let result = close_pane_in_tree(&tree, &PaneId("root".into()));
        assert!(result.is_err());
    }

    #[test]
    fn apply_split_sizes_updates_root() {
        let tree = LayoutNode::Split {
            direction: SplitDirection::Horizontal,
            sizes: vec![SplitSize::Flex(1.0), SplitSize::Flex(1.0)],
            children: vec![make_leaf("a", "svc1"), make_leaf("b", "svc2")],
        };
        let new_tree =
            apply_split_sizes(&tree, &[], vec![SplitSize::Flex(2.0), SplitSize::Flex(1.0)])
                .unwrap();
        let vp = Rect {
            x: 0.0,
            y: 0.0,
            width: 900.0,
            height: 600.0,
        };
        let rects = compute_rects(&new_tree, vp);
        // H-Split flex 2:1, divider=4 → remaining=896, a=2/3*896≈597.33, b=1/3*896≈298.67
        let ra = rects.iter().find(|(id, _)| id.0 == "a").unwrap();
        let rb = rects.iter().find(|(id, _)| id.0 == "b").unwrap();
        // remaining = 900 - 4 = 896; a = 2/3*896 = 597.33, b = 1/3*896 = 298.67
        assert!((ra.1.width - 597.33).abs() < 1.0);
        assert!((rb.1.width - 298.67).abs() < 1.0);
    }

    #[test]
    fn apply_split_sizes_bad_path_fails() {
        let tree = make_leaf("a", "svc1");
        let result = apply_split_sizes(&tree, &[0], vec![SplitSize::Flex(1.0)]);
        assert!(result.is_err());
    }

    #[test]
    fn collect_service_ids_works() {
        let tree = LayoutNode::Split {
            direction: SplitDirection::Horizontal,
            sizes: vec![SplitSize::Flex(1.0), SplitSize::Flex(1.0)],
            children: vec![make_leaf("a", "svc1"), make_leaf("b", "svc2")],
        };
        let ids = collect_service_ids(&tree);
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&"svc1".to_string()));
        assert!(ids.contains(&"svc2".to_string()));
    }

    #[test]
    fn compute_rects_accounts_for_divider() {
        let node = LayoutNode::Split {
            direction: SplitDirection::Horizontal,
            sizes: vec![SplitSize::Flex(1.0), SplitSize::Flex(1.0)],
            children: vec![make_leaf("l", "s1"), make_leaf("r", "s2")],
        };
        let vp = Rect {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 200.0,
        };
        let rects = compute_rects(&node, vp);
        let l = rects.iter().find(|(id, _)| id.0 == "l").unwrap();
        let r = rects.iter().find(|(id, _)| id.0 == "r").unwrap();
        // (100 - 4) / 2 = 48 each; right starts at 48 + 4 = 52
        assert!((l.1.width - 48.0).abs() < 0.1);
        assert_eq!(l.1.x, 0.0);
        assert!((r.1.x - 52.0).abs() < 0.1);
        assert!((r.1.width - 48.0).abs() < 0.1);
    }

    #[test]
    fn build_tree_for_preset_2x2_structure() {
        let tree = build_tree_for_preset(crate::state::LayoutPreset::TwoByTwo);
        if let LayoutNode::Split {
            direction,
            children,
            ..
        } = &tree
        {
            assert_eq!(*direction, SplitDirection::Horizontal);
            assert_eq!(children.len(), 2);
            assert!(matches!(
                &children[0],
                LayoutNode::Split {
                    direction: SplitDirection::Vertical,
                    ..
                }
            ));
            assert!(matches!(
                &children[1],
                LayoutNode::Split {
                    direction: SplitDirection::Vertical,
                    ..
                }
            ));
        } else {
            panic!("expected H-Split at root");
        }
        // All 4 leaves should be empty
        assert_eq!(collect_service_ids(&tree).len(), 0);
    }

    #[test]
    fn fill_preset_preserves_order() {
        let tree = build_tree_for_preset(crate::state::LayoutPreset::TwoH);
        let svc_ids = vec!["slack".to_string(), "gmail".to_string()];
        let filled = fill_preset_with_services(tree, &svc_ids);
        let ids = collect_service_ids(&filled);
        assert_eq!(ids, vec!["slack".to_string(), "gmail".to_string()]);
    }

    #[test]
    fn fill_preset_fewer_services_leaves_empties() {
        let tree = build_tree_for_preset(crate::state::LayoutPreset::TwoByTwo);
        let svc_ids = vec!["slack".to_string()];
        let filled = fill_preset_with_services(tree, &svc_ids);
        let ids = collect_service_ids(&filled);
        assert_eq!(ids.len(), 1);
        assert_eq!(ids[0], "slack");
    }

    #[test]
    fn collect_service_ids_excludes_empty() {
        let tree = LayoutNode::Leaf(Pane {
            id: PaneId("root".into()),
            kind: PaneKind::Service(String::new()),
            webview_label: String::new(),
            visible: true,
        });
        assert!(collect_service_ids(&tree).is_empty());
    }

    #[test]
    fn build_tree_three_h() {
        let tree = build_tree_for_preset(crate::state::LayoutPreset::ThreeH);
        if let LayoutNode::Split {
            direction,
            children,
            ..
        } = &tree
        {
            assert_eq!(*direction, SplitDirection::Horizontal);
            assert_eq!(children.len(), 3);
        } else {
            panic!("expected H-Split");
        }
        assert_eq!(collect_service_ids(&tree).len(), 0);
    }

    #[test]
    fn build_tree_four_h() {
        let tree = build_tree_for_preset(crate::state::LayoutPreset::FourH);
        if let LayoutNode::Split {
            direction,
            children,
            ..
        } = &tree
        {
            assert_eq!(*direction, SplitDirection::Horizontal);
            assert_eq!(children.len(), 4);
        } else {
            panic!("expected H-Split");
        }
        assert_eq!(collect_service_ids(&tree).len(), 0);
    }
}
