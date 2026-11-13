use core::convert::TryFrom;

use crate::graphics::Rect;

use super::{
    element::ElementId,
    style::{Dimension, LayoutDirection},
    tree::{TreeError, UiTree},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutError {
    NoRoot,
    Tree(TreeError),
    TreeTooDeep,
    CapacityExceeded,
}

#[derive(Clone, Copy)]
struct LayoutSlot {
    generation: u32,
    rect: Option<Rect>,
}

impl LayoutSlot {
    const EMPTY: Self = Self {
        generation: 0,
        rect: None,
    };
}

pub struct LayoutResult<const MAX_NODES: usize> {
    slots: [LayoutSlot; MAX_NODES],
}

impl<const MAX_NODES: usize> LayoutResult<MAX_NODES> {
    fn new() -> Self {
        Self {
            slots: [LayoutSlot::EMPTY; MAX_NODES],
        }
    }

    fn set(&mut self, id: ElementId, rect: Rect) -> Result<(), LayoutError> {
        let slot = self
            .slots
            .get_mut(id.index() as usize)
            .ok_or(LayoutError::CapacityExceeded)?;

        slot.generation = id.generation();
        slot.rect = Some(rect);

        Ok(())
    }

    pub fn rect(&self, id: ElementId) -> Option<Rect> {
        let slot = self.slots.get(id.index() as usize)?;

        if slot.generation != id.generation() {
            return None;
        }

        slot.rect
    }
}

pub struct LayoutEngine;

impl LayoutEngine {
    pub fn compute<const MAX_NODES: usize>(
        tree: &UiTree<MAX_NODES>,
        viewport: Rect,
    ) -> Result<LayoutResult<MAX_NODES>, LayoutError> {
        let root = tree.root().ok_or(LayoutError::NoRoot)?;

        let mut result = LayoutResult::new();

        layout_node(tree, root, viewport, &mut result, 0)?;

        Ok(result)
    }
}

fn layout_node<const MAX_NODES: usize>(
    tree: &UiTree<MAX_NODES>,
    id: ElementId,
    rect: Rect,
    result: &mut LayoutResult<MAX_NODES>,
    depth: usize,
) -> Result<(), LayoutError> {
    if depth >= MAX_NODES {
        return Err(LayoutError::TreeTooDeep);
    }

    let style = *tree.element(id).map_err(LayoutError::Tree)?.style();

    result.set(id, rect)?;

    let content = inset_rect(rect, style.padding());

    match style.direction() {
        LayoutDirection::Overlay => {
            layout_overlay(tree, id, content, result, depth)?;
        }

        LayoutDirection::Vertical => {
            layout_stack(tree, id, content, result, depth, true, style.gap())?;
        }

        LayoutDirection::Horizontal => {
            layout_stack(tree, id, content, result, depth, false, style.gap())?;
        }
    }

    Ok(())
}

fn layout_overlay<const MAX_NODES: usize>(
    tree: &UiTree<MAX_NODES>,
    parent: ElementId,
    content: Rect,
    result: &mut LayoutResult<MAX_NODES>,
    depth: usize,
) -> Result<(), LayoutError> {
    for child in tree.children(parent).map_err(LayoutError::Tree)? {
        let style = *tree.element(child).map_err(LayoutError::Tree)?.style();

        let child_rect = Rect {
            x: content.x,
            y: content.y,

            width: resolve_dimension(style.width(), content.width),

            height: resolve_dimension(style.height(), content.height),
        };

        layout_node(tree, child, child_rect, result, depth + 1)?;
    }

    Ok(())
}

fn layout_stack<const MAX_NODES: usize>(
    tree: &UiTree<MAX_NODES>,
    parent: ElementId,
    content: Rect,
    result: &mut LayoutResult<MAX_NODES>,
    depth: usize,
    vertical: bool,
    gap: u32,
) -> Result<(), LayoutError> {
    let available_main = if vertical {
        content.height
    } else {
        content.width
    };

    let available_cross = if vertical {
        content.width
    } else {
        content.height
    };

    let mut child_count = 0usize;
    let mut fixed_main = 0u32;
    let mut fill_count = 0usize;

    for child in tree.children(parent).map_err(LayoutError::Tree)? {
        child_count += 1;

        if child_count > MAX_NODES {
            return Err(LayoutError::TreeTooDeep);
        }

        let child_style = tree.element(child).map_err(LayoutError::Tree)?.style();

        let dimension = if vertical {
            child_style.height()
        } else {
            child_style.width()
        };

        match dimension {
            Dimension::Pixels(size) => {
                fixed_main = fixed_main.saturating_add(size);
            }

            Dimension::Fill => {
                fill_count += 1;
            }
        }
    }

    if child_count == 0 {
        return Ok(());
    }

    let gap_count = u32::try_from(child_count - 1).unwrap_or(u32::MAX);

    let total_gap = gap.saturating_mul(gap_count);

    let occupied = fixed_main.saturating_add(total_gap);

    let remaining = available_main.saturating_sub(occupied);

    let fill_count_u32 = u32::try_from(fill_count).unwrap_or(u32::MAX);

    let fill_size = if fill_count_u32 > 0 {
        remaining / fill_count_u32
    } else {
        0
    };

    let fill_remainder = if fill_count_u32 > 0 {
        remaining % fill_count_u32
    } else {
        0
    };

    let mut cursor = 0u32;
    let mut fill_index = 0usize;

    for child in tree.children(parent).map_err(LayoutError::Tree)? {
        let child_style = *tree.element(child).map_err(LayoutError::Tree)?.style();

        let (main_dimension, cross_dimension) = if vertical {
            (child_style.height(), child_style.width())
        } else {
            (child_style.width(), child_style.height())
        };

        let requested_main = match main_dimension {
            Dimension::Pixels(size) => size,

            Dimension::Fill => {
                let extra = if fill_index < fill_remainder as usize {
                    1
                } else {
                    0
                };

                fill_index += 1;

                fill_size.saturating_add(extra)
            }
        };

        let remaining_main = available_main.saturating_sub(cursor);

        let main_size = requested_main.min(remaining_main);

        let cross_size = resolve_dimension(cross_dimension, available_cross);

        let child_rect = if vertical {
            Rect {
                x: content.x,
                y: add_offset(content.y, cursor),
                width: cross_size,
                height: main_size,
            }
        } else {
            Rect {
                x: add_offset(content.x, cursor),
                y: content.y,
                width: main_size,
                height: cross_size,
            }
        };

        layout_node(tree, child, child_rect, result, depth + 1)?;

        cursor = cursor.saturating_add(main_size).saturating_add(gap);
    }

    Ok(())
}

fn resolve_dimension(dimension: Dimension, available: u32) -> u32 {
    match dimension {
        Dimension::Fill => available,
        Dimension::Pixels(size) => size.min(available),
    }
}

fn inset_rect(rect: Rect, padding: u32) -> Rect {
    let horizontal = padding.min(rect.width / 2);
    let vertical = padding.min(rect.height / 2);

    Rect {
        x: add_offset(rect.x, horizontal),
        y: add_offset(rect.y, vertical),

        width: rect.width.saturating_sub(horizontal.saturating_mul(2)),

        height: rect.height.saturating_sub(vertical.saturating_mul(2)),
    }
}

fn add_offset(position: i32, offset: u32) -> i32 {
    let offset = offset.min(i32::MAX as u32) as i32;

    position.saturating_add(offset)
}
