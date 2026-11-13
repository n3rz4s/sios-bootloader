use crate::graphics::{Color, FONT_5X7, Point, Rect, Renderer, draw_text};

use super::{
    element::ElementId,
    layout::LayoutResult,
    tree::{TreeError, UiTree},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaintError {
    Tree(TreeError),
    MissingLayout,
    CorruptTree,
}

pub fn paint<const MAX_NODES: usize>(
    renderer: &mut Renderer<'_, '_>,
    tree: &UiTree<MAX_NODES>,
    layout: &LayoutResult<MAX_NODES>,
) -> Result<(), PaintError> {
    renderer.reset_clip();

    paint_elements(renderer, tree, layout, None)
}

pub fn paint_region<const MAX_NODES: usize>(
    renderer: &mut Renderer<'_, '_>,
    tree: &UiTree<MAX_NODES>,
    layout: &LayoutResult<MAX_NODES>,
    dirty: Rect,
    background: Color,
) -> Result<(), PaintError> {
    let screen = Rect::new(0, 0, renderer.width(), renderer.height());

    let Some(dirty) = dirty.intersection(screen) else {
        return Ok(());
    };

    renderer.set_clip(dirty);

    renderer.fill_rect(dirty, background);

    let result = paint_elements(renderer, tree, layout, Some(dirty));

    renderer.reset_clip();

    result
}

fn paint_elements<const MAX_NODES: usize>(
    renderer: &mut Renderer<'_, '_>,
    tree: &UiTree<MAX_NODES>,
    layout: &LayoutResult<MAX_NODES>,
    dirty: Option<Rect>,
) -> Result<(), PaintError> {
    let Some(elements) = tree.depth_first() else {
        return Ok(());
    };

    for id in elements {
        if !is_effectively_visible(tree, id)? {
            continue;
        }

        let element = tree.element(id).map_err(PaintError::Tree)?;

        let rect = layout.rect(id).ok_or(PaintError::MissingLayout)?;

        if let Some(dirty) = dirty {
            if rect.intersection(dirty).is_none() {
                continue;
            }
        }

        let style = *element.style();

        let opacity = style.opacity();

        let background = apply_opacity(style.background(), opacity);

        draw_background(renderer, rect, background, style.border_radius());

        let border_color = apply_opacity(style.border_color(), opacity);

        draw_border(
            renderer,
            rect,
            style.border_width(),
            style.border_radius(),
            border_color,
        );

        if element.has_text() {
            let padding = style.padding().min(i32::MAX as u32) as i32;

            let origin = Point {
                x: rect.x.saturating_add(padding),
                y: rect.y.saturating_add(padding),
            };

            let foreground = apply_opacity(style.foreground(), opacity);

            draw_text(
                renderer,
                &FONT_5X7,
                origin,
                element.text(),
                foreground,
                style.font_scale(),
            );
        }
    }

    Ok(())
}

fn apply_opacity(color: Color, opacity: u8) -> Color {
    let alpha = (u16::from(color.a) * u16::from(opacity) + 127) / 255;

    Color::rgba(color.r, color.g, color.b, alpha as u8)
}

fn draw_background(renderer: &mut Renderer<'_, '_>, rect: Rect, color: Color, radius: u32) {
    if color.a == 0 {
        return;
    }

    if radius > 0 {
        renderer.fill_rounded_rect(rect, radius, color);
    } else {
        renderer.fill_rect(rect, color);
    }
}

fn draw_border(renderer: &mut Renderer<'_, '_>, rect: Rect, width: u32, radius: u32, color: Color) {
    if width == 0 || color.a == 0 || rect.is_empty() {
        return;
    }

    let min_dimension = rect.width.min(rect.height);

    let max_layers = min_dimension.saturating_add(1) / 2;

    let layers = width.min(max_layers);

    for inset in 0..layers {
        let offset = inset.min(i32::MAX as u32) as i32;

        let border_rect = Rect::new(
            rect.x.saturating_add(offset),
            rect.y.saturating_add(offset),
            rect.width.saturating_sub(inset.saturating_mul(2)),
            rect.height.saturating_sub(inset.saturating_mul(2)),
        );

        if border_rect.is_empty() {
            break;
        }

        let current_radius = radius.saturating_sub(inset);

        if radius > 0 {
            renderer.draw_rounded_rect(border_rect, current_radius, color);
        } else {
            renderer.draw_rect(border_rect, color);
        }
    }
}

fn is_effectively_visible<const MAX_NODES: usize>(
    tree: &UiTree<MAX_NODES>,
    id: ElementId,
) -> Result<bool, PaintError> {
    let mut current = Some(id);

    let mut visited = 0usize;

    while let Some(current_id) = current {
        visited += 1;

        if visited > tree.len() {
            return Err(PaintError::CorruptTree);
        }

        let element = tree.element(current_id).map_err(PaintError::Tree)?;

        if !element.is_visible() {
            return Ok(false);
        }

        current = tree.parent(current_id).map_err(PaintError::Tree)?;
    }

    Ok(true)
}
