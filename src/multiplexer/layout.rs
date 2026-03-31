use std::collections::HashMap;

pub type PaneId = usize;

#[derive(Debug, Clone, PartialEq)]
pub enum SplitDirection {
    Horizontal, // top / bottom
    Vertical,   // left / right
}

#[derive(Debug, Clone)]
pub enum Layout {
    Leaf(PaneId),
    Split {
        direction: SplitDirection,
        left:  Box<Layout>,
        right: Box<Layout>,
        /// Fraction of space allocated to the left/top child (0.1 … 0.9).
        ratio: f32,
    },
}

impl Layout {
    pub const MAX_DEPTH: usize = 4;
    pub const SEP_PX:    f32   = 4.0; // visual separator thickness in pixels

    pub fn new(id: PaneId) -> Self {
        Layout::Leaf(id)
    }

    pub fn depth(&self) -> usize {
        match self {
            Layout::Leaf(_)                    => 0,
            Layout::Split { left, right, .. } =>
                1 + left.depth().max(right.depth()),
        }
    }

    pub fn pane_ids(&self) -> Vec<PaneId> {
        match self {
            Layout::Leaf(id)                    => vec![*id],
            Layout::Split { left, right, .. } => {
                let mut ids = left.pane_ids();
                ids.extend(right.pane_ids());
                ids
            }
        }
    }

    /// The left-most pane in the subtree — stable identity for a split node.
    pub fn first_pane(&self) -> PaneId {
        match self {
            Layout::Leaf(id)           => *id,
            Layout::Split { left, .. } => left.first_pane(),
        }
    }

    /// Returns the ratio of the split whose left child's first pane == divider_id.
    pub fn get_ratio(&self, divider_id: PaneId) -> Option<f32> {
        match self {
            Layout::Leaf(_) => None,
            Layout::Split { left, right, ratio, .. } => {
                if left.first_pane() == divider_id {
                    Some(*ratio)
                } else {
                    left.get_ratio(divider_id)
                        .or_else(|| right.get_ratio(divider_id))
                }
            }
        }
    }

    /// Updates the ratio of the split identified by its left child's first pane.
    pub fn set_ratio(&mut self, divider_id: PaneId, new_ratio: f32) {
        match self {
            Layout::Leaf(_) => {}
            Layout::Split { left, right, ratio, .. } => {
                if left.first_pane() == divider_id {
                    *ratio = new_ratio.clamp(0.1, 0.9);
                } else {
                    left.set_ratio(divider_id, new_ratio);
                    right.set_ratio(divider_id, new_ratio);
                }
            }
        }
    }

    /// Computes pixel (width, height) for each pane given the total available area.
    /// Accounts for the separator between splits.
    pub fn pane_pixel_sizes(&self, w: f32, h: f32) -> HashMap<PaneId, (f32, f32)> {
        match self {
            Layout::Leaf(id) => {
                let mut m = HashMap::new();
                m.insert(*id, (w, h));
                m
            }
            Layout::Split { direction, left, right, ratio } => {
                let content = match direction {
                    SplitDirection::Vertical   => w - Self::SEP_PX,
                    SplitDirection::Horizontal => h - Self::SEP_PX,
                };
                let (lw, lh, rw, rh) = match direction {
                    SplitDirection::Vertical => {
                        let lw = (content * ratio).max(1.0);
                        let rw = (content * (1.0 - ratio)).max(1.0);
                        (lw, h, rw, h)
                    }
                    SplitDirection::Horizontal => {
                        let lh = (content * ratio).max(1.0);
                        let rh = (content * (1.0 - ratio)).max(1.0);
                        (w, lh, w, rh)
                    }
                };
                let mut m = left.pane_pixel_sizes(lw, lh);
                m.extend(right.pane_pixel_sizes(rw, rh));
                m
            }
        }
    }

    pub fn split(&self, target: PaneId, dir: SplitDirection, new_id: PaneId) -> Option<Self> {
        if self.depth() >= Self::MAX_DEPTH {
            return None;
        }
        self.split_inner(target, dir, new_id)
    }

    fn split_inner(&self, target: PaneId, dir: SplitDirection, new_id: PaneId) -> Option<Self> {
        match self {
            Layout::Leaf(id) if *id == target => Some(Layout::Split {
                direction: dir,
                left:  Box::new(Layout::Leaf(target)),
                right: Box::new(Layout::Leaf(new_id)),
                ratio: 0.5,
            }),
            Layout::Leaf(_) => None,
            Layout::Split { direction, left, right, ratio } => {
                if let Some(nl) = left.split_inner(target, dir.clone(), new_id) {
                    return Some(Layout::Split {
                        direction: direction.clone(),
                        left:  Box::new(nl),
                        right: right.clone(),
                        ratio: *ratio,
                    });
                }
                right.split_inner(target, dir, new_id).map(|nr| Layout::Split {
                    direction: direction.clone(),
                    left:  left.clone(),
                    right: Box::new(nr),
                    ratio: *ratio,
                })
            }
        }
    }

    pub fn remove(&self, target: PaneId) -> Option<Self> {
        match self {
            Layout::Leaf(id) if *id == target => None,
            Layout::Leaf(_) => Some(self.clone()),
            Layout::Split { direction, left, right, ratio } => {
                match (left.as_ref(), right.as_ref()) {
                    (Layout::Leaf(l), _) if *l == target => Some(*right.clone()),
                    (_, Layout::Leaf(r)) if *r == target => Some(*left.clone()),
                    _ => {
                        let nl = left.remove(target).unwrap_or_else(|| *left.clone());
                        let nr = right.remove(target).unwrap_or_else(|| *right.clone());
                        Some(Layout::Split {
                            direction: direction.clone(),
                            left:  Box::new(nl),
                            right: Box::new(nr),
                            ratio: *ratio,
                        })
                    }
                }
            }
        }
    }
}
