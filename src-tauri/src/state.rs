use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Space {
    pub id: String,
    pub name: String,
    pub tree: LayoutNode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Service {
    pub id: String,
    pub name: String,
    pub url: String,
    pub icon: String,
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub favicon_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiService {
    pub id: String,
    pub name: String,
    pub url: String,
    #[serde(rename = "isDefault")]
    pub is_default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowBounds {
    pub width: u32,
    pub height: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub x: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub y: Option<i32>,
}

impl Default for WindowBounds {
    fn default() -> Self {
        Self {
            width: 1400,
            height: 900,
            x: None,
            y: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppState {
    pub services: Vec<Service>,
    pub ai_services: Vec<AiService>,
    pub active_service_id: String,
    pub active_ai_service_id: String,
    pub show_ai_companion: bool,
    pub ai_width: u32,
    pub window_bounds: WindowBounds,
    pub spaces: Vec<Space>,
    pub active_space_id: String,
    #[serde(skip)]
    pub badge_counts: HashMap<String, u32>,
    #[serde(skip)]
    pub created_webview_labels: Vec<String>,
    #[serde(skip)]
    pub ai_webview_created: bool,
    /// Runtime cache of the active space's tree. Sync to spaces on every mutation.
    #[serde(skip)]
    pub service_tree: Arc<LayoutNode>,
    #[serde(skip)]
    pub focused_pane_id: Option<PaneId>,
}

impl AppState {
    /// Sync the runtime service_tree back into the active space's tree.
    pub fn sync_tree_to_active_space(&mut self) {
        let tree = self.service_tree.as_ref().clone();
        let id = self.active_space_id.clone();
        if let Some(space) = self.spaces.iter_mut().find(|s| s.id == id) {
            space.tree = tree;
        }
    }

    /// Load the active space's tree into service_tree.
    pub fn load_active_space_tree(&mut self) {
        let id = self.active_space_id.clone();
        if let Some(space) = self.spaces.iter().find(|s| s.id == id) {
            self.service_tree = Arc::new(space.tree.clone());
            self.focused_pane_id = crate::layout::first_pane_id(&space.tree);
        }
    }
}

// ========================================
// Layout tree types (Phase 2 foundation)
// ========================================

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PaneId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaneKind {
    Chrome,
    Service(String),
    AiCompanion,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pane {
    pub id: PaneId,
    pub kind: PaneKind,
    pub webview_label: String,
    pub visible: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SplitDirection {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum SplitSize {
    Fixed(f32),
    Flex(f32),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LayoutPreset {
    Single,
    TwoH,
    TwoV,
    TwoByTwo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LayoutNode {
    Leaf(Pane),
    Split {
        direction: SplitDirection,
        sizes: Vec<SplitSize>,
        children: Vec<LayoutNode>,
    },
}

pub type LayoutTree = Arc<LayoutNode>;

impl Default for LayoutNode {
    fn default() -> Self {
        LayoutNode::Leaf(Pane {
            id: PaneId("root".into()),
            kind: PaneKind::Service(String::new()),
            webview_label: String::new(),
            visible: true,
        })
    }
}

impl Default for AppState {
    fn default() -> Self {
        let root_id = PaneId("root".into());
        let default_tree = LayoutNode::Leaf(Pane {
            id: root_id.clone(),
            kind: PaneKind::Service(String::new()),
            webview_label: String::new(),
            visible: true,
        });
        let default_space = Space {
            id: "space-1".to_string(),
            name: "スペース 1".to_string(),
            tree: default_tree.clone(),
        };
        Self {
            services: Vec::new(),
            ai_services: vec![
                AiService {
                    id: "gemini".to_string(),
                    name: "Gemini".to_string(),
                    url: "https://gemini.google.com".to_string(),
                    is_default: true,
                },
                AiService {
                    id: "chatgpt".to_string(),
                    name: "ChatGPT".to_string(),
                    url: "https://chat.openai.com".to_string(),
                    is_default: true,
                },
                AiService {
                    id: "claude".to_string(),
                    name: "Claude".to_string(),
                    url: "https://claude.ai".to_string(),
                    is_default: true,
                },
            ],
            active_service_id: "".to_string(),
            active_ai_service_id: "gemini".to_string(),
            show_ai_companion: true,
            ai_width: 400,
            window_bounds: WindowBounds::default(),
            spaces: vec![default_space],
            active_space_id: "space-1".to_string(),
            badge_counts: HashMap::new(),
            created_webview_labels: Vec::new(),
            ai_webview_created: false,
            service_tree: Arc::new(default_tree),
            focused_pane_id: Some(root_id),
        }
    }
}
