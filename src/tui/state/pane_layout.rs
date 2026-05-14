//! Pane layout tree — recursive binary splits.
//!
//! Layout is a binary tree of splits:
//!   - HSplit : left / right  (vim "vsplit")
//!   - VSplit : top  / bottom (vim "split")
//!
//! Constraints enforced:
//!   - Maximum 4 leaves reachable via HSplit chains (4 columns)
//!   - Maximum 2 leaves reachable via VSplit chains (2 rows)
//!   - Total pane count ≤ 8

use ratatui::layout::Rect;
use std::collections::HashMap;

use super::{cmdline::SearchDirection, dashboard::TableMode};

// ═══════════════════════════════════════════════════════════════════════════════
// Pane identity
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(PartialEq, Eq, Clone, Copy, Debug, Hash)]
pub struct PaneId(pub uuid::Uuid);

impl PaneId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Pane type
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaneType {
    TableList,
    TableView,
    SchemaView,
    QueryEditor,
}

impl std::fmt::Display for PaneType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PaneType::TableList => write!(f, "list"),
            PaneType::TableView => write!(f, "table"),
            PaneType::SchemaView => write!(f, "schema"),
            PaneType::QueryEditor => write!(f, "query"),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Search state (pane-local)
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone)]
pub struct SearchState {
    pub query: String,
    pub direction: SearchDirection,
    pub matches: Vec<usize>,
    pub current_idx: usize,
}

// ═══════════════════════════════════════════════════════════════════════════════
// Pane state
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone)]
pub struct Pane {
    pub id: PaneId,
    pub kind: PaneType,

    // ── Display ID ────────────────────────────────────────────────────────────
    /// Human-readable number shown in the border title (1, 2, 3…).
    pub display_id: usize,

    // ── Bound table ───────────────────────────────────────────────────────────
    /// Which table this pane is associated with (set when converting to
    /// TableView / SchemaView / QueryEditor).
    pub bound_table: Option<String>,

    // ── TableList state ─────────────────────────────────────────────────────
    pub nav_cursor: usize,
    pub nav_offset: usize,

    // ── TableView / SchemaView state ────────────────────────────────────────
    pub row_cursor: usize,
    pub row_offset: usize,
    pub cursor_col: usize,
    pub col_offset: usize,
    pub mode: TableMode,

    // ── Search state (pane-local) ───────────────────────────────────────────
    pub last_search: Option<SearchState>,

    // ── Visual selection anchor ─────────────────────────────────────────────
    /// When in VisualRow / VisualColumn, this is the row where the selection
    /// started. All rows between anchor and cursor are highlighted.
    pub visual_anchor: Option<usize>,

    // ── Staged cell edits (pane-local) ──────────────────────────────────────
    /// Each entry is (row_idx, col_idx, new_value).
    pub pending_updates: Vec<(usize, usize, String)>,

    // ── Staged row deletes (pane-local) ─────────────────────────────────────
    /// PK values of rows marked for deletion.
    pub pending_deletes: Vec<String>,

    // ── Filter / sort (pane-local) ──────────────────────────────────────────
    pub filter: Option<String>,
    pub sort_col: Option<String>,
    pub sort_desc: bool,

    // ── Cached render area (updated every frame) ────────────────────────────
    pub area: Option<Rect>,
}

impl Pane {
    pub fn new(id: PaneId, kind: PaneType, display_id: usize) -> Self {
        Self {
            id,
            kind,
            display_id,
            bound_table: None,
            nav_cursor: 0,
            nav_offset: 0,
            row_cursor: 0,
            row_offset: 0,
            cursor_col: 0,
            col_offset: 0,
            mode: TableMode::Normal,
            last_search: None,
            visual_anchor: None,
            pending_updates: Vec::new(),
            pending_deletes: Vec::new(),
            filter: None,
            sort_col: None,
            sort_desc: false,
            area: None,
        }
    }

    // ── Helpers ─────────────────────────────────────────────────────────────

    pub fn reset_to_list(&mut self) {
        self.kind = PaneType::TableList;
        self.bound_table = None;
        self.nav_cursor = 0;
        self.nav_offset = 0;
        self.row_cursor = 0;
        self.row_offset = 0;
        self.cursor_col = 0;
        self.col_offset = 0;
        self.mode = TableMode::Normal;
        self.pending_updates.clear();
        self.pending_deletes.clear();
        self.filter = None;
        self.sort_col = None;
        self.sort_desc = false;
    }

    pub fn set_table_view(&mut self, table_name: String) {
        self.kind = PaneType::TableView;
        self.bound_table = Some(table_name);
        self.row_cursor = 0;
        self.row_offset = 0;
        self.cursor_col = 0;
        self.col_offset = 0;
        self.mode = TableMode::Normal;
        self.last_search = None; // clear search highlight when leaving list
        self.visual_anchor = None;
        self.pending_updates.clear();
        self.pending_deletes.clear();
        self.filter = None;
        self.sort_col = None;
        self.sort_desc = false;
    }

    pub fn set_schema_view(&mut self, table_name: String) {
        self.kind = PaneType::SchemaView;
        self.bound_table = Some(table_name);
    }

    pub fn set_query_editor(&mut self) {
        self.kind = PaneType::QueryEditor;
    }

    // ── Row navigation ──────────────────────────────────────────────────────
    pub fn row_next(&mut self, max: usize) {
        if max > 0 {
            self.row_cursor = (self.row_cursor + 1).min(max.saturating_sub(1));
        }
    }
    pub fn row_prev(&mut self) {
        self.row_cursor = self.row_cursor.saturating_sub(1);
    }
    pub fn row_top(&mut self) {
        self.row_cursor = 0;
    }
    pub fn row_bottom(&mut self, max: usize) {
        if max > 0 {
            self.row_cursor = max.saturating_sub(1);
        }
    }
    pub fn sync_row_offset(&mut self, viewport: usize) {
        if self.row_cursor < self.row_offset {
            self.row_offset = self.row_cursor;
        } else if self.row_cursor >= self.row_offset + viewport {
            self.row_offset = self.row_cursor + 1 - viewport;
        }
    }

    // ── Column navigation ───────────────────────────────────────────────────
    pub fn col_right(&mut self, max: usize) {
        if max > 0 {
            self.cursor_col = (self.cursor_col + 1).min(max.saturating_sub(1));
        }
    }
    pub fn col_left(&mut self) {
        self.cursor_col = self.cursor_col.saturating_sub(1);
    }
    pub fn col_first(&mut self) {
        self.cursor_col = 0;
    }
    pub fn col_last(&mut self, max: usize) {
        if max > 0 {
            self.cursor_col = max.saturating_sub(1);
        }
    }
    pub fn sync_col_offset(&mut self, viewport: usize) {
        if self.cursor_col < self.col_offset {
            self.col_offset = self.cursor_col;
        } else if self.cursor_col >= self.col_offset + viewport {
            self.col_offset = self.cursor_col + 1 - viewport;
        }
    }

    // ── Nav list navigation ─────────────────────────────────────────────────
    pub fn nav_next(&mut self, max: usize) {
        if max > 0 {
            self.nav_cursor = (self.nav_cursor + 1).min(max.saturating_sub(1));
        }
    }
    pub fn nav_prev(&mut self) {
        self.nav_cursor = self.nav_cursor.saturating_sub(1);
    }
    pub fn nav_top(&mut self) {
        self.nav_cursor = 0;
    }
    pub fn nav_bottom(&mut self, max: usize) {
        if max > 0 {
            self.nav_cursor = max.saturating_sub(1);
        }
    }
    pub fn sync_nav_offset(&mut self, viewport: usize) {
        if self.nav_cursor < self.nav_offset {
            self.nav_offset = self.nav_cursor;
        } else if self.nav_cursor >= self.nav_offset + viewport {
            self.nav_offset = self.nav_cursor + 1 - viewport;
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Layout tree node
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone)]
pub enum LayoutNode {
    Leaf(PaneId),
    HSplit {
        ratio: f32,        // 0.0..=1.0, fixed at 0.5
        left: Box<LayoutNode>,
        right: Box<LayoutNode>,
    },
    VSplit {
        ratio: f32,        // 0.0..=1.0, fixed at 0.5
        top: Box<LayoutNode>,
        bottom: Box<LayoutNode>,
    },
}

impl LayoutNode {
    pub fn leaf(id: PaneId) -> Self {
        LayoutNode::Leaf(id)
    }

    pub fn hsplit(left: LayoutNode, right: LayoutNode) -> Self {
        LayoutNode::HSplit {
            ratio: 0.5,
            left: Box::new(left),
            right: Box::new(right),
        }
    }

    pub fn vsplit(top: LayoutNode, bottom: LayoutNode) -> Self {
        LayoutNode::VSplit {
            ratio: 0.5,
            top: Box::new(top),
            bottom: Box::new(bottom),
        }
    }

    /// Count total leaves in this subtree.
    pub fn count_leaves(&self) -> usize {
        match self {
            LayoutNode::Leaf(_) => 1,
            LayoutNode::HSplit { left, right, .. } => left.count_leaves() + right.count_leaves(),
            LayoutNode::VSplit { top, bottom, .. } => top.count_leaves() + bottom.count_leaves(),
        }
    }

    /// True if this subtree contains a VSplit anywhere.
    pub fn has_vsplit(&self) -> bool {
        match self {
            LayoutNode::Leaf(_) => false,
            LayoutNode::VSplit { .. } => true,
            LayoutNode::HSplit { left, right, .. } => left.has_vsplit() || right.has_vsplit(),
        }
    }

    /// True if this subtree contains the target leaf.
    pub fn contains(&self, target: PaneId) -> bool {
        match self {
            LayoutNode::Leaf(id) => *id == target,
            LayoutNode::HSplit { left, right, .. } => {
                left.contains(target) || right.contains(target)
            }
            LayoutNode::VSplit { top, bottom, .. } => {
                top.contains(target) || bottom.contains(target)
            }
        }
    }

    /// Find the innermost split that directly contains `target` as a leaf.
    /// Returns `(is_hsplit, is_first_child, &mut ratio)`.
    pub fn find_split_for(&mut self, target: PaneId) -> Option<(bool, bool, &mut f32)> {
        match self {
            LayoutNode::Leaf(_) => None,
            LayoutNode::HSplit { ratio, left, right } => {
                if left.contains(target) {
                    if let LayoutNode::Leaf(_) = **left {
                        Some((true, true, ratio))
                    } else {
                        left.find_split_for(target)
                    }
                } else if right.contains(target) {
                    if let LayoutNode::Leaf(_) = **right {
                        Some((true, false, ratio))
                    } else {
                        right.find_split_for(target)
                    }
                } else {
                    None
                }
            }
            LayoutNode::VSplit { ratio, top, bottom } => {
                if top.contains(target) {
                    if let LayoutNode::Leaf(_) = **top {
                        Some((false, true, ratio))
                    } else {
                        top.find_split_for(target)
                    }
                } else if bottom.contains(target) {
                    if let LayoutNode::Leaf(_) = **bottom {
                        Some((false, false, ratio))
                    } else {
                        bottom.find_split_for(target)
                    }
                } else {
                    None
                }
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Pane tree manager
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone)]
pub struct PaneTree {
    pub root: LayoutNode,
    pub panes: HashMap<PaneId, Pane>,
    pub active_pane: PaneId,
    /// Next display ID to assign (recycled IDs are used first).
    next_display_id: usize,
    /// IDs that were freed when panes were closed and can be reused.
    recycled_ids: Vec<usize>,
}

impl PaneTree {
    pub fn new(initial: PaneType) -> Self {
        let id = PaneId::new();
        let mut panes = HashMap::new();
        panes.insert(id, Pane::new(id, initial, 1));
        Self {
            root: LayoutNode::leaf(id),
            panes,
            active_pane: id,
            next_display_id: 2,
            recycled_ids: Vec::new(),
        }
    }

    fn alloc_display_id(&mut self) -> usize {
        if let Some(id) = self.recycled_ids.pop() {
            id
        } else {
            let id = self.next_display_id;
            self.next_display_id += 1;
            id
        }
    }

    /// Get mutable reference to the active pane.
    pub fn active_mut(&mut self) -> Option<&mut Pane> {
        self.panes.get_mut(&self.active_pane)
    }

    /// Get reference to the active pane.
    pub fn active(&self) -> Option<&Pane> {
        self.panes.get(&self.active_pane)
    }

    /// Total number of panes.
    pub fn pane_count(&self) -> usize {
        self.panes.len()
    }

    // ── Resizing ─────────────────────────────────────────────────────────────

    const MIN_RATIO: f32 = 0.1;
    const MAX_RATIO: f32 = 0.9;

    /// Resize the active pane by `delta` percentage points.
    /// Positive delta grows the pane; negative shrinks it.
    pub fn resize_active(&mut self, delta: i32) -> Result<(), &'static str> {
        if self.pane_count() <= 1 {
            return Err("cannot resize single pane");
        }
        let delta_f = delta as f32 / 100.0;
        let target = self.active_pane;
        match self.root.find_split_for(target) {
            Some((_, is_first_child, ratio)) => {
                if is_first_child {
                    *ratio = (*ratio + delta_f).clamp(Self::MIN_RATIO, Self::MAX_RATIO);
                } else {
                    *ratio = (*ratio - delta_f).clamp(Self::MIN_RATIO, Self::MAX_RATIO);
                }
                Ok(())
            }
            None => Err("no split found for active pane"),
        }
    }

    // ── Splitting ────────────────────────────────────────────────────────────

    /// Split the active pane vertically (new pane to the right).
    pub fn split_active_v(&mut self, new_kind: PaneType) -> Result<PaneId, &'static str> {
        if self.pane_count() >= 8 {
            return Err("maximum pane count (8) reached");
        }
        let new_id = PaneId::new();
        let display_id = self.alloc_display_id();
        self.panes.insert(new_id, Pane::new(new_id, new_kind, display_id));
        self.replace_leaf_with_split(self.active_pane, true, new_id);
        self.active_pane = new_id;
        Ok(new_id)
    }

    /// Split the active pane horizontally (new pane below).
    pub fn split_active_h(&mut self, new_kind: PaneType) -> Result<PaneId, &'static str> {
        if self.pane_count() >= 8 {
            return Err("maximum pane count (8) reached");
        }
        let new_id = PaneId::new();
        let display_id = self.alloc_display_id();
        self.panes.insert(new_id, Pane::new(new_id, new_kind, display_id));
        self.replace_leaf_with_split(self.active_pane, false, new_id);
        self.active_pane = new_id;
        Ok(new_id)
    }

    fn replace_leaf_with_split(&mut self, target: PaneId, is_hsplit: bool, new_id: PaneId) {
        self.root = Self::replace_leaf_recursive(
            std::mem::replace(&mut self.root, LayoutNode::leaf(new_id)),
            target,
            is_hsplit,
            new_id,
        );
    }

    fn replace_leaf_recursive(node: LayoutNode, target: PaneId, is_hsplit: bool, new_id: PaneId) -> LayoutNode {
        match node {
            LayoutNode::Leaf(id) if id == target => {
                if is_hsplit {
                    LayoutNode::hsplit(LayoutNode::Leaf(id), LayoutNode::Leaf(new_id))
                } else {
                    LayoutNode::vsplit(LayoutNode::Leaf(id), LayoutNode::Leaf(new_id))
                }
            }
            LayoutNode::HSplit { ratio, left, right } => {
                let new_left = Self::replace_leaf_recursive(*left, target, is_hsplit, new_id);
                let new_right = Self::replace_leaf_recursive(*right, target, is_hsplit, new_id);
                LayoutNode::HSplit {
                    ratio,
                    left: Box::new(new_left),
                    right: Box::new(new_right),
                }
            }
            LayoutNode::VSplit { ratio, top, bottom } => {
                let new_top = Self::replace_leaf_recursive(*top, target, is_hsplit, new_id);
                let new_bottom = Self::replace_leaf_recursive(*bottom, target, is_hsplit, new_id);
                LayoutNode::VSplit {
                    ratio,
                    top: Box::new(new_top),
                    bottom: Box::new(new_bottom),
                }
            }
            other => other,
        }
    }

    // ── Closing ──────────────────────────────────────────────────────────────

    /// Close the active pane. Returns true if the tree became empty.
    pub fn close_active(&mut self) -> bool {
        if self.pane_count() <= 1 {
            self.panes.clear();
            return true;
        }

        let target = self.active_pane;
        if let Some(pane) = self.panes.remove(&target) {
            self.recycled_ids.push(pane.display_id);
        }

        let (new_root, sibling) = Self::remove_leaf_recursive(self.root.clone(), target);
        self.root = new_root.unwrap_or_else(|| LayoutNode::Leaf(target));

        self.active_pane = sibling
            .or_else(|| self.panes.keys().next().copied())
            .unwrap_or(target);

        false
    }

    /// Close a pane by its display ID.
    pub fn close_by_display_id(&mut self, display_id: usize) -> bool {
        let target = self.panes.iter()
            .find(|(_, p)| p.display_id == display_id)
            .map(|(id, _)| *id);

        let Some(target) = target else { return false };

        if self.pane_count() <= 1 {
            self.panes.clear();
            return true;
        }

        if let Some(pane) = self.panes.remove(&target) {
            self.recycled_ids.push(pane.display_id);
        }

        let (new_root, sibling) = Self::remove_leaf_recursive(self.root.clone(), target);
        self.root = new_root.unwrap_or_else(|| LayoutNode::Leaf(target));

        self.active_pane = sibling
            .or_else(|| self.panes.keys().next().copied())
            .unwrap_or(target);

        false
    }

    /// Recursively remove a leaf from the tree and return the promoted sibling.
    fn remove_leaf_recursive(
        node: LayoutNode,
        target: PaneId,
    ) -> (Option<LayoutNode>, Option<PaneId>) {
        match node {
            LayoutNode::Leaf(id) if id == target => {
                (None, None)
            }
            leaf @ LayoutNode::Leaf(_) => {
                (Some(leaf), None)
            }
            LayoutNode::HSplit {
                ratio,
                left,
                right,
            } => {
                let right_sibling = first_leaf_id(&right);
                let left_sibling = first_leaf_id(&left);

                let (new_left, s1) = Self::remove_leaf_recursive(*left, target);
                if new_left.is_none() {
                    return (Some(*right), right_sibling);
                }

                let (new_right, s2) = Self::remove_leaf_recursive(*right, target);
                if new_right.is_none() {
                    return (new_left, left_sibling);
                }

                (
                    Some(LayoutNode::HSplit {
                        ratio,
                        left: Box::new(new_left.unwrap()),
                        right: Box::new(new_right.unwrap()),
                    }),
                    s1.or(s2),
                )
            }
            LayoutNode::VSplit {
                ratio,
                top,
                bottom,
            } => {
                let bottom_sibling = first_leaf_id(&bottom);
                let top_sibling = first_leaf_id(&top);

                let (new_top, s1) = Self::remove_leaf_recursive(*top, target);
                if new_top.is_none() {
                    return (Some(*bottom), bottom_sibling);
                }

                let (new_bottom, s2) = Self::remove_leaf_recursive(*bottom, target);
                if new_bottom.is_none() {
                    return (new_top, top_sibling);
                }

                (
                    Some(LayoutNode::VSplit {
                        ratio,
                        top: Box::new(new_top.unwrap()),
                        bottom: Box::new(new_bottom.unwrap()),
                    }),
                    s1.or(s2),
                )
            }
        }
    }

    // ── Navigation ───────────────────────────────────────────────────────────

    /// Move focus to the pane in the given direction.
    pub fn navigate(&mut self, direction: PaneDirection) {
        let Some(active) = self.active() else { return };
        let Some(active_area) = active.area else { return };

        let mut best: Option<(PaneId, u16)> = None;

        for (id, pane) in &self.panes {
            if *id == self.active_pane {
                continue;
            }
            let Some(area) = pane.area else { continue };

            let (is_adjacent, distance) = match direction {
                PaneDirection::Left => {
                    let other_right = area.x.saturating_add(area.width);
                    let active_left = active_area.x;
                    if other_right <= active_left {
                        let overlap_top = area.y.max(active_area.y);
                        let overlap_bottom = (area.y + area.height).min(active_area.y + active_area.height);
                        if overlap_bottom > overlap_top {
                            (true, active_left - other_right)
                        } else {
                            (false, 0)
                        }
                    } else {
                        (false, 0)
                    }
                }
                PaneDirection::Right => {
                    let other_left = area.x;
                    let active_right = active_area.x.saturating_add(active_area.width);
                    if other_left >= active_right {
                        let overlap_top = area.y.max(active_area.y);
                        let overlap_bottom = (area.y + area.height).min(active_area.y + active_area.height);
                        if overlap_bottom > overlap_top {
                            (true, other_left - active_right)
                        } else {
                            (false, 0)
                        }
                    } else {
                        (false, 0)
                    }
                }
                PaneDirection::Up => {
                    let other_bottom = area.y.saturating_add(area.height);
                    let active_top = active_area.y;
                    if other_bottom <= active_top {
                        let overlap_left = area.x.max(active_area.x);
                        let overlap_right = (area.x + area.width).min(active_area.x + active_area.width);
                        if overlap_right > overlap_left {
                            (true, active_top - other_bottom)
                        } else {
                            (false, 0)
                        }
                    } else {
                        (false, 0)
                    }
                }
                PaneDirection::Down => {
                    let other_top = area.y;
                    let active_bottom = active_area.y.saturating_add(active_area.height);
                    if other_top >= active_bottom {
                        let overlap_left = area.x.max(active_area.x);
                        let overlap_right = (area.x + area.width).min(active_area.x + active_area.width);
                        if overlap_right > overlap_left {
                            (true, other_top - active_bottom)
                        } else {
                            (false, 0)
                        }
                    } else {
                        (false, 0)
                    }
                }
            };

            if is_adjacent {
                if best.map_or(true, |(_, d)| distance < d) {
                    best = Some((*id, distance));
                }
            }
        }

        if let Some((id, _)) = best {
            self.active_pane = id;
        }
    }

    /// Collect all leaf PaneIds in render order (left→right, top→bottom).
    pub fn collect_leaves(&self) -> Vec<PaneId> {
        let mut out = Vec::new();
        Self::collect_leaves_recursive(&self.root, &mut out);
        out
    }

    fn collect_leaves_recursive(node: &LayoutNode, out: &mut Vec<PaneId>) {
        match node {
            LayoutNode::Leaf(id) => out.push(*id),
            LayoutNode::HSplit { left, right, .. } => {
                Self::collect_leaves_recursive(left, out);
                Self::collect_leaves_recursive(right, out);
            }
            LayoutNode::VSplit { top, bottom, .. } => {
                Self::collect_leaves_recursive(top, out);
                Self::collect_leaves_recursive(bottom, out);
            }
        }
    }

    /// Update cached `area` for every pane by recursively computing layout.
    pub fn compute_areas(&mut self, area: Rect) {
        Self::compute_areas_recursive(&self.root, area, &mut self.panes);
    }

    fn compute_areas_recursive(node: &LayoutNode, area: Rect, panes: &mut HashMap<PaneId, Pane>) {
        match node {
            LayoutNode::Leaf(id) => {
                if let Some(pane) = panes.get_mut(id) {
                    pane.area = Some(area);
                }
            }
            LayoutNode::HSplit { ratio, left, right, .. } => {
                let split_x = area.x + (area.width as f32 * ratio.max(0.01).min(0.99)) as u16;
                let left_area = Rect {
                    x: area.x,
                    y: area.y,
                    width: split_x.saturating_sub(area.x),
                    height: area.height,
                };
                let right_area = Rect {
                    x: split_x,
                    y: area.y,
                    width: (area.x + area.width).saturating_sub(split_x),
                    height: area.height,
                };
                Self::compute_areas_recursive(left, left_area, panes);
                Self::compute_areas_recursive(right, right_area, panes);
            }
            LayoutNode::VSplit { ratio, top, bottom, .. } => {
                let split_y = area.y + (area.height as f32 * ratio.max(0.01).min(0.99)) as u16;
                let top_area = Rect {
                    x: area.x,
                    y: area.y,
                    width: area.width,
                    height: split_y.saturating_sub(area.y),
                };
                let bottom_area = Rect {
                    x: area.x,
                    y: split_y,
                    width: area.width,
                    height: (area.y + area.height).saturating_sub(split_y),
                };
                Self::compute_areas_recursive(top, top_area, panes);
                Self::compute_areas_recursive(bottom, bottom_area, panes);
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Direction helpers
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaneDirection {
    Left,
    Down,
    Up,
    Right,
}

/// Return the first PaneId found in a subtree (top-left-most leaf).
fn first_leaf_id(node: &LayoutNode) -> Option<PaneId> {
    match node {
        LayoutNode::Leaf(id) => Some(*id),
        LayoutNode::HSplit { left, .. } => first_leaf_id(left),
        LayoutNode::VSplit { top, .. } => first_leaf_id(top),
    }
}
