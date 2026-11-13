use alloc::{format, string::String, vec::Vec};

use core::{cell::RefCell, fmt::Write};

use uefi::{Status, boot, helpers, proto::console::text::Input, system};

use crate::{
    boot::{BootConfig, BootManager, BootTarget, StorageManager},
    firmware,
    graphics::{Color, FrameBuffer, Rect, Renderer},
    input::{InputEvent, read_event},
    ui::{
        Button, Dimension, ElementId, Label, LayoutEngine, LayoutResult, List, PaintError, Panel,
        UiTree, paint, paint_region,
    },
};

const UI_CAPACITY: usize = 32;

const BACKGROUND_COLOR: Color = Color::rgb(12, 9, 25);

const NAVIGATION_TEXT: &str = "BOOT CORE READY - ARROWS: NAVIGATE - ENTER: SELECT";

struct BootUi {
    tree: UiTree<UI_CAPACITY>,

    buttons: Vec<ElementId>,

    footer: ElementId,

    manager: BootManager,

    _storage: StorageManager,
}

fn report_error(message: &str, status: Status) {
    system::with_stdout(|stdout| {
        let _ = stdout.clear();

        let _ = writeln!(stdout, "SIOS Bootloader");
        let _ = writeln!(stdout, "");
        let _ = writeln!(stdout, "{}", message);
        let _ = writeln!(stdout, "Status: {:?}", status);
    });
}

fn clear_text_console() {
    system::with_stdout(|stdout| {
        let _ = stdout.clear();
    });
}

fn create_boot_button(
    tree: &mut UiTree<UI_CAPACITY>,
    parent: ElementId,
    text: &str,
) -> Option<ElementId> {
    let mut button = Button::new(text).ok()?;

    let style = *button.style();

    button.set_style(style.with_height(Dimension::Pixels(56)).with_font_scale(2));

    tree.create_element(parent, button).ok()
}

fn build_boot_ui(manager: BootManager, storage: StorageManager) -> Option<BootUi> {
    if manager.entries().len() + 6 > UI_CAPACITY {
        return None;
    }

    let mut tree = UiTree::<UI_CAPACITY>::new();

    let root = tree.create_root().ok()?;

    let mut panel_element = Panel::new();

    let panel_style = *panel_element.style();

    panel_element.set_style(panel_style.with_padding(24).with_gap(14));

    let panel = tree.create_element(root, panel_element).ok()?;

    let mut heading = Label::new("SIOS Bootloader").ok()?;

    let heading_style = *heading.style();

    heading.set_style(
        heading_style
            .with_height(Dimension::Pixels(34))
            .with_font_scale(3)
            .with_foreground(Color::WHITE),
    );

    tree.create_element(panel, heading).ok()?;

    let subtitle_text = format!(
        "UEFI Storage: {} volumes / {} devices / {} GPT",
        storage.volume_count(),
        storage.block_device_count(),
        storage.gpt_disk_count(),
    );

    let mut subtitle = Label::new(&subtitle_text).ok()?;

    let subtitle_style = *subtitle.style();

    subtitle.set_style(
        subtitle_style
            .with_height(Dimension::Pixels(24))
            .with_foreground(Color::rgb(170, 155, 195)),
    );

    tree.create_element(panel, subtitle).ok()?;

    let list = tree.create_element(panel, List::new()).ok()?;

    let mut buttons = Vec::with_capacity(manager.entries().len());

    for entry in manager.entries() {
        let button = create_boot_button(&mut tree, list, entry.name())?;

        buttons.push(button);
    }

    let mut footer = Label::new(NAVIGATION_TEXT).ok()?;

    let footer_style = *footer.style();

    footer.set_style(
        footer_style
            .with_height(Dimension::Pixels(18))
            .with_foreground(Color::rgb(137, 124, 158)),
    );

    let footer_id = tree.create_element(panel, footer).ok()?;

    let mut ui = BootUi {
        tree,
        buttons,
        footer: footer_id,
        manager,
        _storage: storage,
    };

    apply_button_styles(&mut ui).ok()?;

    Some(ui)
}

fn apply_button_styles(ui: &mut BootUi) -> Result<(), Status> {
    for index in 0..ui.buttons.len() {
        let button_id = ui.buttons[index];

        let current_style = *ui
            .tree
            .element(button_id)
            .map_err(|_| Status::ABORTED)?
            .style();

        let style = if index == ui.manager.selected_index() {
            current_style
                .with_background(Color::rgba(91, 61, 135, 255))
                .with_opacity(255)
                .with_border(2, 12, Color::rgba(190, 150, 255, 255))
        } else {
            current_style
                .with_background(Color::rgba(48, 36, 67, 255))
                .with_opacity(145)
                .with_border(1, 10, Color::rgba(130, 160, 210, 190))
        };

        ui.tree
            .element_mut(button_id)
            .map_err(|_| Status::ABORTED)?
            .set_style(style);
    }

    Ok(())
}

fn set_footer_text(ui: &mut BootUi, text: &str) -> Result<(), Status> {
    let footer = ui
        .tree
        .element_mut(ui.footer)
        .map_err(|_| Status::ABORTED)?;

    footer.set_text(text).map_err(|_| Status::ABORTED)?;

    Ok(())
}

fn render_ui(
    renderer: &mut Renderer<'_, '_>,
    ui: &BootUi,
    layout: &LayoutResult<UI_CAPACITY>,
) -> Result<(), PaintError> {
    renderer.clear(BACKGROUND_COLOR);

    paint(renderer, &ui.tree, layout)
}

fn render_dirty_region(
    renderer: &mut Renderer<'_, '_>,
    ui: &BootUi,
    layout: &LayoutResult<UI_CAPACITY>,
    rect: Rect,
) -> Result<(), Status> {
    paint_region(renderer, &ui.tree, layout, rect, BACKGROUND_COLOR).map_err(|_| Status::ABORTED)
}

fn update_footer_text(ui: &mut BootUi, text: &str) -> Result<bool, Status> {
    let unchanged = ui
        .tree
        .element(ui.footer)
        .map_err(|_| Status::ABORTED)?
        .text()
        == text;

    if unchanged {
        return Ok(false);
    }

    ui.tree
        .element_mut(ui.footer)
        .map_err(|_| Status::ABORTED)?
        .set_text(text)
        .map_err(|_| Status::ABORTED)?;

    Ok(true)
}

fn handle_input_event(
    event: InputEvent,
    ui: &mut BootUi,
    renderer: &mut Renderer<'_, '_>,
    layout: &LayoutResult<UI_CAPACITY>,
) -> Result<(), Status> {
    match event {
        InputEvent::Up | InputEvent::Down => {
            let old_index = ui.manager.selected_index();

            let old_rect = layout.rect(ui.buttons[old_index]).ok_or(Status::ABORTED)?;

            match event {
                InputEvent::Up => {
                    ui.manager.select_previous();
                }

                InputEvent::Down => {
                    ui.manager.select_next();
                }

                _ => {}
            }

            let new_index = ui.manager.selected_index();

            if new_index != old_index {
                apply_button_styles(ui)?;

                let new_rect = layout.rect(ui.buttons[new_index]).ok_or(Status::ABORTED)?;

                render_dirty_region(renderer, ui, layout, old_rect)?;

                render_dirty_region(renderer, ui, layout, new_rect)?;
            }

            if update_footer_text(ui, NAVIGATION_TEXT)? {
                let footer_rect = layout.rect(ui.footer).ok_or(Status::ABORTED)?;

                render_dirty_region(renderer, ui, layout, footer_rect)?;
            }
        }

        InputEvent::Confirm => {
            let selected_name = String::from(ui.manager.selected_entry().name());

            let selected_target = ui.manager.selected_entry().target().clone();

            let message = match selected_target {
                BootTarget::LinuxNative(_) => {
                    format!(
                        "{}: native Linux loader is not implemented yet.",
                        selected_name,
                    )
                }

                BootTarget::LinuxEfiStub(target) | BootTarget::EfiChainload(target) => {
                    clear_text_console();

                    renderer.clear(Color::rgb(0, 0, 0));

                    let result = crate::boot::efi::chainload(
                        &mut ui._storage,
                        &target,
                    );

                    clear_text_console();

                    match result {
                        Ok(()) => {
                            format!(
                                "{} returned to SIOS.",
                                selected_name,
                            )
                        }

                        Err(error) => {
                            format!(
                                "EFI chainload failed: {:?} ({:?})",
                                error.stage(),
                                error.status(),
                            )
                        }
                    }
                }
            };

            update_footer_text(ui, &message)?;

            render_ui(
                renderer,
                ui,
                layout,
            )
            .map_err(|_| Status::ABORTED)?;
        }

        InputEvent::Cancel => {
            if update_footer_text(ui, NAVIGATION_TEXT)? {
                let footer_rect = layout.rect(ui.footer).ok_or(Status::ABORTED)?;

                render_dirty_region(renderer, ui, layout, footer_rect)?;
            }
        }
    }

    Ok(())
}

fn run_input_loop(
    input: &mut Input,
    ui: &mut BootUi,
    renderer: &mut Renderer<'_, '_>,
    layout: &LayoutResult<UI_CAPACITY>,
) -> Status {
    loop {
        match read_event(input) {
            Ok(Some(event)) => {
                if let Err(status) = handle_input_event(event, ui, renderer, layout) {
                    return status;
                }
            }

            Ok(None) => {
                boot::stall(30_000);
            }

            Err(error) => {
                return error.status();
            }
        }
    }
}

pub fn run() -> Status {
    if let Err(error) = helpers::init() {
        return error.status();
    }

    let boot_manager = match BootConfig::development().and_then(BootManager::new) {
        Ok(manager) => manager,

        Err(_) => {
            let status = Status::ABORTED;

            report_error("Failed to initialize boot catalog.", status);

            return status;
        }
    };

    let mut storage = match StorageManager::discover() {
        Ok(storage) => storage,

        Err(error) => {
            let status = error.status();

            report_error("Failed to initialize UEFI storage.", status);

            return status;
        }
    };

    for volume in storage.volumes_mut() {
        match volume.read_file("\\storage-test.txt", 4096) {
            Ok(contents) => {
                if contents.as_slice() != b"SIOS_STORAGE_TEST" {
                    report_error("Storage test: invalid file contents.", Status::DEVICE_ERROR);

                    return Status::DEVICE_ERROR;
                }

                break;
            }

            Err(error) if error.status() == Status::NOT_FOUND => {
                continue;
            }

            Err(error) => {
                let status = error.status();

                report_error("Storage test: file read failed.", status);

                return status;
            }
        }
    }

    let mut gop = match firmware::open_graphics_output() {
        Ok(gop) => gop,

        Err(error) => {
            let status = error.status();

            report_error("Failed to open Graphics Output Protocol.", status);

            return status;
        }
    };

    let mut framebuffer = match FrameBuffer::new(&mut gop) {
        Ok(framebuffer) => framebuffer,

        Err(error) => {
            let status = error.status();

            report_error("Failed to initialize framebuffer.", status);

            return status;
        }
    };

    let width = framebuffer.width().min(i32::MAX as usize) as u32;

    let height = framebuffer.height().min(i32::MAX as usize) as u32;

    let margin_x = width / 8;
    let margin_y = height / 6;

    let panel_width = width.saturating_sub(margin_x.saturating_mul(2));

    let panel_height = height.saturating_sub(margin_y.saturating_mul(2));

    let viewport = Rect::new(margin_x as i32, margin_y as i32, panel_width, panel_height);

    let ui = match build_boot_ui(boot_manager, storage) {
        Some(ui) => ui,

        None => {
            let status = Status::OUT_OF_RESOURCES;

            report_error("Failed to construct UI.", status);

            return status;
        }
    };

    let layout = match LayoutEngine::compute(&ui.tree, viewport) {
        Ok(layout) => layout,

        Err(_) => {
            let status = Status::ABORTED;

            report_error("Failed to calculate UI layout.", status);

            return status;
        }
    };

    let mut renderer = Renderer::new(&mut framebuffer);

    if render_ui(&mut renderer, &ui, &layout).is_err() {
        drop(renderer);

        let status = Status::ABORTED;

        report_error("Failed to render initial UI.", status);

        return status;
    }

    let runtime_state = RefCell::new((ui, renderer));

    let input_status = system::with_stdin(|stdin| {
        let mut state = runtime_state.borrow_mut();

        let (ui, renderer) = &mut *state;

        run_input_loop(stdin, ui, renderer, &layout)
    });

    drop(runtime_state);

    if input_status != Status::SUCCESS {
        report_error("Keyboard input failed.", input_status);
    }

    input_status
}
