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
    },
}

impl Layout {
    pub const MAX_DEPTH: usize = 4;

    pub fn new(id: PaneId) -> Self {
        Layout::Leaf(id)
    }

    pub fn depth(&self) -> usize {
        match self {
            Layout::Leaf(_) => 0,
            Layout::Split { left, right, .. } =>
                1 + left.depth().max(right.depth()),
        }
    }

    pub fn pane_ids(&self) -> Vec<PaneId> {
        match self {
            Layout::Leaf(id) => vec![*id],
            Layout::Split { left, right, .. } => {
                let mut ids = left.pane_ids();
                ids.extend(right.pane_ids());
                ids
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
            }),
            Layout::Leaf(_) => None,
            Layout::Split { direction, left, right } => {
                if let Some(nl) = left.split_inner(target, dir.clone(), new_id) {
                    return Some(Layout::Split {
                        direction: direction.clone(),
                        left:  Box::new(nl),
                        right: right.clone(),
                    });
                }
                right.split_inner(target, dir, new_id).map(|nr| Layout::Split {
                    direction: direction.clone(),
                    left:  left.clone(),
                    right: Box::new(nr),
                })
            }
        }
    }

    pub fn remove(&self, target: PaneId) -> Option<Self> {
        match self {
            Layout::Leaf(id) if *id == target => None,
            Layout::Leaf(_) => Some(self.clone()),
            Layout::Split { direction, left, right } => {
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
                        })
                    }
                }
            }
        }
    }
}
