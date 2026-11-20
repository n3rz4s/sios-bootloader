use alloc::{format, string::String, vec::Vec};

use core::{cell::RefCell, fmt::Write};

use uefi::{Status, boot, helpers, proto::console::text::Input, system};

use crate::{
    boot::{
        BootConfig, BootManager, BootParams, BootParamsBuilder, BootTarget, KernelCommandLine,
        LinuxKernelImage, LinuxNativeTarget, StorageManager,
    },
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

fn inspect_native_kernel(storage: &mut StorageManager, target: &LinuxNativeTarget) -> String {
    const MAX_KERNEL_SIZE: usize = 128 * 1024 * 1024;

    let path = target.kernel_path().as_str();

    let mut matching_volume = None;
    let mut scan_error = None;

    for (index, volume) in storage.volumes_mut().iter_mut().enumerate() {
        match volume.metadata(path) {
            Ok(metadata) => {
                if metadata.is_directory() {
                    continue;
                }

                if metadata.size() == 0 {
                    return String::from("Kernel image is empty.");
                }

                if matching_volume.is_some() {
                    return String::from("Kernel path is ambiguous.");
                }

                matching_volume = Some(index);
            }

            Err(error) if error.status() == Status::NOT_FOUND => {
                continue;
            }

            Err(error) => {
                scan_error.get_or_insert(error.status());
            }
        }
    }

    if let Some(status) = scan_error {
        return format!("Kernel discovery failed: {:?}", status,);
    }

    let Some(volume_index) = matching_volume else {
        return format!("Kernel not found: {}", path,);
    };

    let Some(volume) = storage.volume_mut(volume_index) else {
        return String::from("Invalid storage volume.");
    };

    let kernel_bytes = match volume.read_file(path, MAX_KERNEL_SIZE) {
        Ok(bytes) => bytes,

        Err(error) => {
            return format!("Kernel read failed: {:?}", error.status(),);
        }
    };

    inspect_native_kernel_image(&kernel_bytes, target.command_line())
}

fn inspect_native_kernel_image(kernel_bytes: &[u8], command_line_value: &str) -> String {
    let image = match LinuxKernelImage::parse(kernel_bytes) {
        Ok(image) => image,

        Err(error) => {
            return format!("Kernel parse failed: {:?}", error,);
        }
    };

    if let Err(error) = image.validate_native_x86_64() {
        return format!("Unsupported Linux Kernel: {:?}", error,);
    }

    let header = image.header();
    let version = header.protocol_version();

    let command_line = match KernelCommandLine::new(command_line_value, header) {
        Ok(command_line) => command_line,

        Err(error) => {
            return format!("Command line validation failed: {:?}", error,);
        }
    };

    let boot_params = match BootParamsBuilder::new(kernel_bytes, header) {
        Ok(builder) => builder.build(),

        Err(error) => {
            return format!("Boot Params initialization failed: {:?}", error,);
        }
    };

    if boot_params.sentinel() != 0 {
        return String::from("Boot Params sentinel is invalid.");
    }

    if boot_params.boot_signature() != 0xAA55 {
        return String::from("Boot Params header signature mismatch.");
    }

    if boot_params.protocol_version() != version.raw() {
        return String::from("Boot Params protocol version mismatch.");
    }

    format!(
        "KERNEL OK: BZIMAGE X64 P{}.{} {:?} | BP={} CMD={} E820=PENDING",
        version.major(),
        version.minor(),
        image.payload_format(),
        BootParams::SIZE,
        command_line.len(),
    )
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
                BootTarget::LinuxNative(target) => inspect_native_kernel(&mut ui._storage, &target),

                BootTarget::LinuxEfiStub(target) | BootTarget::EfiChainload(target) => {
                    clear_text_console();

                    renderer.clear(Color::rgb(0, 0, 0));

                    let result = crate::boot::efi::chainload(&mut ui._storage, &target);

                    clear_text_console();

                    match result {
                        Ok(()) => {
                            format!("{} returned to SIOS.", selected_name,)
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

            render_ui(renderer, ui, layout).map_err(|_| Status::ABORTED)?;
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

#[derive(Debug)]
enum InputLoopFailure {
    Keyboard(Status),
    Ui { event: InputEvent, status: Status },
}

fn run_input_loop(
    input: &mut Input,
    ui: &mut BootUi,
    renderer: &mut Renderer<'_, '_>,
    layout: &LayoutResult<UI_CAPACITY>,
) -> Result<(), InputLoopFailure> {
    loop {
        match read_event(input) {
            Ok(Some(event)) => {
                if let Err(status) = handle_input_event(event, ui, renderer, layout) {
                    return Err(InputLoopFailure::Ui { event, status });
                }
            }

            Ok(None) => {
                boot::stall(30_000);
            }

            Err(error) => {
                return Err(InputLoopFailure::Keyboard(error.status()));
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

    match input_status {
        Ok(()) => Status::SUCCESS,

        Err(InputLoopFailure::Keyboard(status)) => {
            report_error("UEFI keyboard protocol failed.", status);

            status
        }

        Err(InputLoopFailure::Ui { event, status }) => {
            let message = format!("Boot UI failed while handling {:?}.", event,);

            report_error(&message, status);

            status
        }
    }
}

#[cfg(test)]
mod tests {
    use alloc::vec;
    use alloc::vec::Vec;

    use super::*;

    use crate::boot::{
        KernelPlacement, MemoryRegion, MemoryRegionKind, PhysicalAddress, PhysicalPageAllocator,
    };

    fn put_u16(image: &mut [u8], offset: usize, value: u16) {
        image[offset..offset + 2].copy_from_slice(&value.to_le_bytes())
    }

    fn put_u32(image: &mut [u8], offset: usize, value: u32) {
        image[offset..offset + 4].copy_from_slice(&value.to_le_bytes())
    }

    fn put_u64(image: &mut [u8], offset: usize, value: u64) {
        image[offset..offset + 8].copy_from_slice(&value.to_le_bytes())
    }

    fn make_kernel(command_line_size: u32) -> Vec<u8> {
        let mut image = vec![0u8; 4096];

        image[0x1F1] = 1;

        put_u32(&mut image, 0x1F4, 64);
        put_u16(&mut image, 0x1FE, 0xAA55);

        image[0x200] = 0xEB;
        image[0x201] = 0x6A;

        image[0x202..0x206].copy_from_slice(b"HdrS");

        put_u16(&mut image, 0x206, 0x020F);

        image[0x211] = 1;

        put_u32(&mut image, 0x22C, u32::MAX);
        put_u32(&mut image, 0x230, 0x200000);

        image[0x234] = 1;
        image[0x235] = 12;

        put_u16(&mut image, 0x236, 0x0003);

        put_u32(&mut image, 0x238, command_line_size);
        put_u32(&mut image, 0x248, 0);
        put_u32(&mut image, 0x24C, 0);

        put_u64(&mut image, 0x258, 0x2000000);

        put_u32(&mut image, 0x260, 0x400000);

        image
    }

    #[test]
    fn runtime_accepts_valid_native_kernel() {
        let image = make_kernel(2047);

        let result = inspect_native_kernel_image(&image, "quiet");

        assert!(result.starts_with("KERNEL OK: BZIMAGE X64 P2.15"),);

        assert!(result.contains("BP=4096"),);

        assert!(result.contains("CMD=5"),);
    }

    #[test]
    fn runtime_rejects_invalid_kernel_image() {
        let image = vec![0u8; 4096];

        let result = inspect_native_kernel_image(&image, "quiet");

        assert!(result.starts_with("Kernel parse failed:"),);
    }

    #[test]
    fn runtime_rejects_command_line_exceeding_kernel_limit() {
        let image = make_kernel(4);

        let result = inspect_native_kernel_image(&image, "quiet");

        assert_eq!(result, "Command line validation failed: TooLong",);
    }

    #[test]
    fn runtime_kernel_placement_integrates_allocator_and_zero_page() {
        let image = make_kernel(2047);

        let parsed_image = LinuxKernelImage::parse(&image).unwrap();

        parsed_image.validate_native_x86_64().unwrap();

        let header = parsed_image.header();

        let regions = [MemoryRegion::new(
            PhysicalAddress::new(0x1000000),
            0x4000000,
            MemoryRegionKind::Usable,
        )
        .unwrap()];

        let mut allocator = PhysicalPageAllocator::new(&regions).unwrap();

        let placement = KernelPlacement::allocate(&image, &header, &mut allocator).unwrap();

        assert_eq!(placement.load_address().as_u64(), 0x2000000,);

        let mut builder = BootParamsBuilder::new(&image, &header).unwrap();

        builder
            .set_kernel_placement(placement.load_address(), placement.alignment() as u32)
            .unwrap();

        let boot_params = builder.build();

        assert_eq!(boot_params.code32_start(), placement.load_address().low32(),);

        assert_eq!(boot_params.kernel_alignment(), placement.alignment() as u32,);

        let mut destination = vec![0xAA; placement.allocation_size() as usize];

        placement.copy_into(&image, &mut destination).unwrap();

        assert_eq!(&destination[..1024], &image[1024..2048],);

        assert!(destination[1024..].iter().all(|byte| *byte == 0),);

        assert_eq!(allocator.allocation_count(), 1,);
    }
}
