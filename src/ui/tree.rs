use core::convert::TryFrom;
use core::fmt;

use super::element::{Element, ElementId, ElementKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TreeError {
    CapacityExceeded,
    RootAlreadyExists,
    RootCannotBeRemoved,
    RootCannotBeChild,
    InvalidElementId,
    CorruptTree,
}

#[derive(Clone, Copy)]
struct Node {
    element: Element,

    parent: Option<ElementId>,

    first_child: Option<ElementId>,
    last_child: Option<ElementId>,

    next_sibling: Option<ElementId>,
}

#[derive(Clone, Copy)]
struct Slot {
    generation: u32,
    node: Option<Node>,
}

impl Slot {
    const EMPTY: Self = Self {
        generation: 1,
        node: None,
    };
}

pub struct UiTree<const MAX_NODES: usize> {
    slots: [Slot; MAX_NODES],

    root: Option<ElementId>,

    len: usize,
}

impl<const MAX_NODES: usize> UiTree<MAX_NODES> {
    pub const fn new() -> Self {
        Self {
            slots: [Slot::EMPTY; MAX_NODES],
            root: None,
            len: 0,
        }
    }

    pub const fn len(&self) -> usize {
        self.len
    }

    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub const fn root(&self) -> Option<ElementId> {
        self.root
    }

    pub fn contains(&self, id: ElementId) -> bool {
        self.node(id).is_some()
    }

    pub fn create_root(&mut self) -> Result<ElementId, TreeError> {
        if self.root.is_some() {
            return Err(TreeError::RootAlreadyExists);
        }

        let id = self.insert_node(Element::root(), None)?;

        self.root = Some(id);

        Ok(id)
    }

    pub fn create_element(
        &mut self,
        parent: ElementId,
        element: Element,
    ) -> Result<ElementId, TreeError> {
        if element.kind() == ElementKind::Root {
            return Err(TreeError::RootCannotBeChild);
        }

        if self.node(parent).is_none() {
            return Err(TreeError::InvalidElementId);
        }

        self.insert_node(element, Some(parent))
    }

    fn insert_node(
        &mut self,
        element: Element,
        parent: Option<ElementId>,
    ) -> Result<ElementId, TreeError> {
        let previous_last = if let Some(parent_id) = parent {
            let parent_node = self.node(parent_id).ok_or(TreeError::InvalidElementId)?;

            match (parent_node.first_child, parent_node.last_child) {
                (None, None) => None,

                (Some(_), Some(last_id)) => {
                    let last_node = self.node(last_id).ok_or(TreeError::CorruptTree)?;

                    if last_node.parent != Some(parent_id) || last_node.next_sibling.is_some() {
                        return Err(TreeError::CorruptTree);
                    }

                    Some(last_id)
                }

                _ => return Err(TreeError::CorruptTree),
            }
        } else {
            None
        };

        let index = self
            .slots
            .iter()
            .position(|slot| slot.node.is_none())
            .ok_or(TreeError::CapacityExceeded)?;

        let index_u32 = u32::try_from(index).map_err(|_| TreeError::CapacityExceeded)?;

        let generation = self.slots[index].generation;

        let id = ElementId::new(index_u32, generation);

        self.slots[index].node = Some(Node {
            element,
            parent,
            first_child: None,
            last_child: None,
            next_sibling: None,
        });

        self.len += 1;

        if let Some(parent_id) = parent {
            if let Some(previous_id) = previous_last {
                self.node_mut(previous_id)
                    .ok_or(TreeError::CorruptTree)?
                    .next_sibling = Some(id);
            } else {
                self.node_mut(parent_id)
                    .ok_or(TreeError::CorruptTree)?
                    .first_child = Some(id);
            }

            self.node_mut(parent_id)
                .ok_or(TreeError::CorruptTree)?
                .last_child = Some(id);
        }

        Ok(id)
    }

    pub fn element(&self, id: ElementId) -> Result<&Element, TreeError> {
        let node = self.node(id).ok_or(TreeError::InvalidElementId)?;

        Ok(&node.element)
    }

    pub fn element_mut(&mut self, id: ElementId) -> Result<&mut Element, TreeError> {
        let node = self.node_mut(id).ok_or(TreeError::InvalidElementId)?;

        Ok(&mut node.element)
    }

    pub fn parent(&self, id: ElementId) -> Result<Option<ElementId>, TreeError> {
        let node = self.node(id).ok_or(TreeError::InvalidElementId)?;

        Ok(node.parent)
    }

    pub fn children(&self, parent: ElementId) -> Result<Children<'_, MAX_NODES>, TreeError> {
        let node = self.node(parent).ok_or(TreeError::InvalidElementId)?;

        Ok(Children {
            tree: self,
            next: node.first_child,
        })
    }

    pub fn depth_first(&self) -> Option<DepthFirst<'_, MAX_NODES>> {
        let root = self.root?;

        Some(DepthFirst {
            tree: self,
            root,
            next: Some(root),
        })
    }

    pub fn remove(&mut self, id: ElementId) -> Result<(), TreeError> {
        if self.root == Some(id) {
            return Err(TreeError::RootCannotBeRemoved);
        }

        let parent = self
            .node(id)
            .ok_or(TreeError::InvalidElementId)?
            .parent
            .ok_or(TreeError::CorruptTree)?;

        let mut pending: [Option<ElementId>; MAX_NODES] = [None; MAX_NODES];

        let mut order: [Option<ElementId>; MAX_NODES] = [None; MAX_NODES];

        let mut pending_len = 1;
        let mut order_len = 0;

        pending[0] = Some(id);

        while pending_len > 0 {
            pending_len -= 1;

            let current = pending[pending_len].take().ok_or(TreeError::CorruptTree)?;

            if order_len >= MAX_NODES {
                return Err(TreeError::CorruptTree);
            }

            order[order_len] = Some(current);
            order_len += 1;

            let mut child = self
                .node(current)
                .ok_or(TreeError::CorruptTree)?
                .first_child;

            while let Some(child_id) = child {
                if pending_len >= MAX_NODES {
                    return Err(TreeError::CorruptTree);
                }

                pending[pending_len] = Some(child_id);
                pending_len += 1;

                child = self
                    .node(child_id)
                    .ok_or(TreeError::CorruptTree)?
                    .next_sibling;
            }
        }

        self.detach_child(parent, id)?;

        for index in (0..order_len).rev() {
            let current = order[index].ok_or(TreeError::CorruptTree)?;

            let slot = self
                .slots
                .get_mut(current.index() as usize)
                .ok_or(TreeError::CorruptTree)?;

            if slot.generation != current.generation() || slot.node.is_none() {
                return Err(TreeError::CorruptTree);
            }

            slot.node = None;

            slot.generation = slot.generation.wrapping_add(1);

            if slot.generation == 0 {
                slot.generation = 1;
            }

            self.len -= 1;
        }

        Ok(())
    }

    fn detach_child(&mut self, parent_id: ElementId, child_id: ElementId) -> Result<(), TreeError> {
        let mut current = self
            .node(parent_id)
            .ok_or(TreeError::CorruptTree)?
            .first_child;

        let mut previous = None;

        while let Some(current_id) = current {
            if current_id == child_id {
                let next = self
                    .node(current_id)
                    .ok_or(TreeError::CorruptTree)?
                    .next_sibling;

                if let Some(previous_id) = previous {
                    self.node_mut(previous_id)
                        .ok_or(TreeError::CorruptTree)?
                        .next_sibling = next;
                } else {
                    self.node_mut(parent_id)
                        .ok_or(TreeError::CorruptTree)?
                        .first_child = next;
                }

                if self
                    .node(parent_id)
                    .ok_or(TreeError::CorruptTree)?
                    .last_child
                    == Some(child_id)
                {
                    self.node_mut(parent_id)
                        .ok_or(TreeError::CorruptTree)?
                        .last_child = previous;
                }

                let child = self.node_mut(child_id).ok_or(TreeError::CorruptTree)?;

                child.parent = None;
                child.next_sibling = None;

                return Ok(());
            }

            previous = Some(current_id);

            current = self
                .node(current_id)
                .ok_or(TreeError::CorruptTree)?
                .next_sibling;
        }

        Err(TreeError::CorruptTree)
    }

    fn node(&self, id: ElementId) -> Option<&Node> {
        let slot = self.slots.get(id.index() as usize)?;

        if slot.generation != id.generation() {
            return None;
        }

        slot.node.as_ref()
    }

    fn node_mut(&mut self, id: ElementId) -> Option<&mut Node> {
        let slot = self.slots.get_mut(id.index() as usize)?;

        if slot.generation != id.generation() {
            return None;
        }

        slot.node.as_mut()
    }
}

impl<const MAX_NODES: usize> Default for UiTree<MAX_NODES> {
    fn default() -> Self {
        Self::new()
    }
}

pub struct Children<'a, const MAX_NODES: usize> {
    tree: &'a UiTree<MAX_NODES>,
    next: Option<ElementId>,
}

impl<const MAX_NODES: usize> Iterator for Children<'_, MAX_NODES> {
    type Item = ElementId;

    fn next(&mut self) -> Option<Self::Item> {
        let current = self.next?;

        self.next = self.tree.node(current).and_then(|node| node.next_sibling);

        Some(current)
    }
}

pub struct DepthFirst<'a, const MAX_NODES: usize> {
    tree: &'a UiTree<MAX_NODES>,
    root: ElementId,
    next: Option<ElementId>,
}

impl<const MAX_NODES: usize> DepthFirst<'_, MAX_NODES> {
    fn successor(&self, current: ElementId) -> Option<ElementId> {
        let node = self.tree.node(current)?;

        if let Some(child) = node.first_child {
            return Some(child);
        }

        let mut cursor = current;

        loop {
            if cursor == self.root {
                return None;
            }

            let node = self.tree.node(cursor)?;

            if let Some(sibling) = node.next_sibling {
                return Some(sibling);
            }

            cursor = node.parent?;
        }
    }
}

impl<const MAX_NODES: usize> Iterator for DepthFirst<'_, MAX_NODES> {
    type Item = ElementId;

    fn next(&mut self) -> Option<Self::Item> {
        let current = self.next?;

        self.next = self.successor(current);

        Some(current)
    }
}

impl fmt::Display for TreeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CapacityExceeded => f.write_str("UI tree capacity exceeded"),
            Self::RootAlreadyExists => f.write_str("UI root already exists"),
            Self::RootCannotBeRemoved => f.write_str("UI root cannot be removed"),
            Self::RootCannotBeChild => f.write_str("Root cannot be created as a child"),
            Self::InvalidElementId => f.write_str("Invalid element ID"),
            Self::CorruptTree => f.write_str("UI tree is corrupted"),
        }
    }
}
