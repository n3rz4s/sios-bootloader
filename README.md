# SIOS Bootloader

<p align="center">
  <strong>A modern UEFI bootloader written entirely in Rust.</strong>
  <br>
  <em>Built from the ground up with a custom graphics renderer, declarative UI engine, storage subsystem, EFI image loader, and native Linux boot infrastructure.</em>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Language-Rust-orange?style=for-the-badge&logo=rust" alt="Rust">
  <img src="https://img.shields.io/badge/Firmware-UEFI-blue?style=for-the-badge" alt="UEFI">
  <img src="https://img.shields.io/badge/Architecture-x86__64-informational?style=for-the-badge" alt="x86_64">
  <img src="https://img.shields.io/badge/Environment-no__std-purple?style=for-the-badge" alt="no_std">
  <img src="https://img.shields.io/badge/Status-Active%20Development-yellow?style=for-the-badge" alt="Active Development">
</p>

---

## Overview

**SIOS Bootloader** is an independent UEFI bootloader project developed as part of the SIOS ecosystem.

Unlike traditional bootloaders that rely on external graphical interfaces or existing boot management frameworks, SIOS Bootloader is being developed around its own internal architecture.

The project provides a custom graphical rendering pipeline, a lightweight UI engine, firmware interaction abstractions, storage discovery, EFI executable loading, and architecture-specific Linux boot infrastructure.

The primary objective is to build a modular and extensible boot environment capable of supporting both EFI chainloading and native Linux kernel booting.

The entire core is written in Rust using a `no_std` environment.

## Features

### UEFI Runtime

- Native UEFI application entry point.
- Firmware service initialization.
- UEFI protocol discovery and interaction.
- Firmware console integration for error reporting.
- Boot-time resource management.

### Storage Subsystem

A dedicated storage subsystem provides access to firmware-exposed storage devices.

- UEFI Block I/O discovery.
- Simple File System protocol discovery.
- Block device abstraction.
- Filesystem volume enumeration.
- File metadata inspection.
- Device Path parsing.
- GPT partition table discovery and parsing.
- GPT partition metadata extraction.
- Storage error tracking and reporting.
- Volume-to-GPT partition association.

### EFI Image Loader

SIOS Bootloader includes an EFI executable loading subsystem.

- EFI image loading through UEFI Boot Services.
- File Device Path construction.
- EFI image startup.
- Load options support.
- EFI chainloading.
- Structured loading errors with execution stages.

### Linux Kernel Image Parser

SIOS Bootloader includes an architecture-specific Linux kernel image parsing subsystem.

The implementation is located under `boot/x86_64/linux/`, separating Linux-specific functionality from the general boot infrastructure.

- x86_64-specific Linux boot subsystem.
- Linux kernel image representation.
- Linux kernel header parsing.
- Structured parsing errors.
- Kernel image metadata extraction.
- Foundation for native Linux kernel loading.

The parser is implemented and serves as the first completed component of the native Linux boot pipeline.

### Linux Boot Protocol

SIOS Bootloader includes an initial implementation of the Linux Boot Protocol infrastructure for x86_64.

This subsystem provides the structures and interfaces required to prepare Linux kernel boot parameters.

- Linux Boot Protocol structures.
- Linux `boot_params` representation.
- Linux boot header integration.
- Kernel command-line infrastructure.
- Architecture-specific address abstractions.
- Boot protocol module organization.
- Structured boot-related error handling.

This subsystem builds upon the Linux kernel image parser and establishes the foundation for subsequent native kernel loading stages.

### Kernel Physical Placement

SIOS Bootloader includes a kernel placement subsystem for preparing a parsed x86_64 Linux `bzImage` for loading into physical memory.

The implementation is located in `boot/x86_64/linux/kernel_placement.rs` and uses the physical page allocator to select and reserve a suitable destination range.

Current capabilities:

- Validation of the supplied setup header against the kernel image.
- Validation of the Linux image type, boot protocol version, and 64-bit support.
- Support for relocatable and non-relocatable kernels.
- Preferred physical load address handling through `pref_address`.
- Kernel alignment handling through `kernel_alignment` and `min_alignment`.
- Fallback placement at or above the preferred address for relocatable kernels.
- Exact-address allocation for non-relocatable kernels.
- Kernel memory sizing using `init_size`.
- Protected kernel image size handling using `syssize`.
- Page-aligned physical allocation through `PhysicalPageAllocator`.
- Above-4-GiB placement when supported by the kernel header.
- Below-4-GiB address restriction when above-4-GiB loading is unsupported.
- Kernel entry point calculation.
- Copying the protected kernel image into a destination buffer and zero-filling the remaining allocation.
- Explicit release of the placement allocation.
- Structured placement and memory allocation errors.

The `KernelPlacement` result retains the allocation and exposes the selected load address, runtime start, preferred address, memory size, alignment, page count, and allocation size.

It also provides access to the calculated entry point and a method for copying the kernel image into a caller-provided destination buffer.

This subsystem prepares the kernel's physical memory placement. It does not yet perform the complete native Linux handoff or initrd placement.

### Physical Memory Management

SIOS Bootloader includes an architecture-specific physical memory management subsystem for x86_64.

The subsystem is located under `boot/x86_64/memory/` and provides the foundation for managing physical memory during native Linux kernel loading.

Current capabilities:

- Physical memory region representation.
- Physical memory range abstraction.
- Memory region classification.
- Physical page allocator.
- Free physical memory range tracking.
- Allocation record management.
- 4 KiB physical page allocation.
- Configurable allocation alignment.
- Minimum and maximum physical address constraints.
- Lowest-address allocation strategy.
- Highest-address allocation strategy.
- Physical memory range reservation.
- Physical page deallocation.
- Adjacent free range merging.
- Allocation ownership validation.
- Structured memory management errors.
- Allocation statistics and free memory tracking.

The `PhysicalPageAllocator` operates on explicitly supplied memory regions and considers only regions classified as `Usable` available for allocation.

It maintains separate collections for free physical ranges and active allocations.

The allocator supports configurable allocation requests through `AllocationRequest`, allowing the caller to specify page count, alignment, address boundaries, and allocation strategy.

This implementation establishes the foundation for kernel physical placement and initrd memory allocation.

### Graphics Engine

The graphics subsystem is developed independently rather than relying on an external UI framework.

Components include:

- Framebuffer abstraction.
- Custom software renderer.
- Rectangle and primitive rendering.
- Color management.
- RGBA support.
- Bitmap font rendering.
- Text measurement and drawing.
- Image representation.
- Compositing system.
- Region-based rendering.

### Declarative UI Engine

SIOS Bootloader contains its own lightweight UI engine designed specifically for the boot environment.

The engine separates UI structure, layout calculation, styling, and rendering.

Current capabilities:

- Element tree management.
- Fixed-capacity UI tree.
- Parent-child relationships.
- Element identification.
- Layout engine.
- Declarative styling API.
- Dimension abstractions.
- Layout direction support.
- Widget-based UI construction.
- Custom painting pipeline.
- Dirty-region rendering.
- Element opacity control using an 8-bit value (`0–255`).

Built-in widgets:

- `Panel`
- `Label`
- `Button`
- `List`

### Keyboard Input

- UEFI keyboard input.
- Navigation events.
- Selection confirmation.
- Cancellation events.
- Keyboard-driven boot menu navigation.

---

## Architecture

The project is organized into independent subsystems to keep firmware interaction, graphics, UI logic, storage, and architecture-specific boot functionality separated.

### Source Structure

```text
src/
├── bin/
│   └── chainload_test.rs
│
├── boot/
│   ├── config.rs
│   ├── entry.rs
│   ├── error.rs
│   ├── manager.rs
│   ├── mod.rs
│   │
│   ├── efi/
│   │   ├── chainload.rs
│   │   ├── error.rs
│   │   ├── image.rs
│   │   ├── mod.rs
│   │   └── path.rs
│   │
│   ├── storage/
│   │   ├── device.rs
│   │   ├── device_path.rs
│   │   ├── error.rs
│   │   ├── gpt.rs
│   │   ├── manager.rs
│   │   ├── mod.rs
│   │   ├── protocols.rs
│   │   └── volume.rs
│   │
│   └── x86_64/
│       ├── address.rs
│       ├── mod.rs
│       │
│       ├── linux/
│       │   ├── boot_params.rs
│       │   ├── command_line.rs
│       │   ├── error.rs
│       │   ├── header.rs
│       │   ├── image.rs
│       │   ├── kernel_placement.rs
│       │   ├── mod.rs
│       │   └── protocol.rs
│       │
│       └── memory/
│           ├── allocator.rs
│           ├── error.rs
│           ├── mod.rs
│           └── region.rs
│
├── firmware/
│   ├── mod.rs
│   └── protocols.rs
│
├── graphics/
│   ├── color.rs
│   ├── compositor.rs
│   ├── font.rs
│   ├── framebuffer.rs
│   ├── image.rs
│   ├── mod.rs
│   ├── renderer.rs
│   └── text.rs
│
├── input/
│   ├── keyboard.rs
│   └── mod.rs
│
├── runtime/
│   └── mod.rs
│
├── ui/
│   ├── element.rs
│   ├── layout.rs
│   ├── mod.rs
│   ├── paint.rs
│   ├── style.rs
│   ├── tree.rs
│   │
│   └── widgets/
│       ├── button.rs
│       ├── label.rs
│       ├── list.rs
│       ├── mod.rs
│       └── panel.rs
│
├── lib.rs
└── main.rs
```

### Boot Architecture

The boot subsystem is divided into independent functional and architecture-specific modules.

**`boot/efi/`**

Responsible for EFI executable loading and chainloading through UEFI Boot Services.

**`boot/storage/`**

Responsible for storage discovery, block device access, filesystem volumes, GPT parsing, and Device Path handling.

**`boot/x86_64/`**

Contains functionality specific to the x86_64 architecture, including address-related abstractions, physical memory management, and Linux boot infrastructure.

**`boot/x86_64/linux/`**

Contains the Linux-specific boot implementation for x86_64.

The Linux subsystem includes separate modules for:

- Kernel image parsing.
- Linux kernel header handling.
- Linux Boot Protocol structures.
- Boot parameter representation.
- Kernel command-line handling.
- Kernel physical placement.
- Relocatable and fixed-address kernel allocation.
- Kernel image copying into the selected destination.
- Protocol-related error handling.

**`boot/x86_64/linux/kernel_placement.rs`**

Implements physical placement for x86_64 Linux `bzImage` kernels.

The placement subsystem validates kernel metadata, determines the required memory size, and obtains a suitable physical allocation from `PhysicalPageAllocator`.

Relocatable kernels may be placed at or above their preferred address while respecting the alignment constraints in the setup header. Non-relocatable kernels must be allocated at their exact preferred address.

The resulting `KernelPlacement` provides the selected load address, runtime start, entry point, allocation information, image-copy operation, and explicit release operation.

**`boot/x86_64/memory/`**

Contains architecture-specific physical memory management functionality.

The memory subsystem is divided into the following modules:

- `allocator.rs` — Physical page allocation, deallocation, reservation, and free range management.
- `region.rs` — Physical memory region and range representations, including memory region classification.
- `error.rs` — Structured errors for physical memory operations.
- `mod.rs` — Memory subsystem module organization and public interfaces.

The subsystem exposes the following primary types:

| Type | Responsibility |
|---|---|
| `PhysicalPageAllocator` | Manages physical page allocation and deallocation. |
| `AllocationRequest` | Defines allocation requirements and constraints. |
| `AllocationStrategy` | Controls lowest-address or highest-address allocation. |
| `PageAllocation` | Represents an allocated physical memory region. |
| `MemoryRegion` | Represents a physical memory region with a specific type. |
| `MemoryRegionKind` | Classifies physical memory regions. |
| `PhysicalRange` | Represents a physical memory address range. |
| `MemoryError` | Provides structured memory management errors. |

The Linux kernel placement subsystem exposes:

| Type | Responsibility |
|---|---|
| `KernelPlacement` | Represents a kernel's selected physical memory placement and owns its allocation. |
| `KernelPlacementError` | Provides structured errors for kernel placement and image-copy operations. |

The allocator supports:

- Allocation of contiguous physical pages.
- Configurable power-of-two alignment.
- Minimum physical address constraints.
- Exclusive maximum physical address constraints.
- Lowest-address and highest-address allocation strategies.
- Reservation of physical address ranges.
- Deallocation of previously allocated pages.
- Automatic merging of adjacent free ranges.

The default physical page size is 4096 bytes (4 KiB).

Kernel placement uses this allocator to reserve a page-aligned range large enough for the kernel's `init_size`.

For relocatable kernels, the placement logic attempts the preferred address first and can fall back to a suitable address above it while reducing alignment down to the minimum alignment declared by the kernel.

For non-relocatable kernels, the preferred address must be available.

The placement logic also checks the kernel header's above-4-GiB capability before allowing an allocation beyond the 4-GiB boundary.

The protected kernel image is copied from the image's payload area; the rest of the allocated destination range is zero-filled.

This separation allows architecture-specific memory management to evolve independently from the general boot infrastructure.

The architecture is designed to accommodate additional platform-specific implementations in the future.

---

## Design Philosophy

### 1. Independent Components

Each subsystem has its own responsibilities and public interfaces.

The graphics renderer does not depend on the UI tree, the UI tree does not directly interact with UEFI protocols, and boot management does not need to know how individual widgets are rendered.

Architecture-specific boot implementations are also isolated from the general boot infrastructure.

### 2. No Standard Library

The bootloader operates in a `no_std` environment.

This means the project cannot rely on the usual operating system facilities and must explicitly manage its firmware-facing operations and runtime requirements.

### 3. Minimal Dependencies

The project uses Rust and the UEFI ecosystem while implementing its own higher-level infrastructure.

External dependencies are intentionally kept limited.

### 4. Rendering Efficiency

The rendering pipeline supports region-based painting, allowing individual UI elements to be updated without repainting the entire framebuffer.

This is particularly important in firmware environments where rendering performance and unnecessary framebuffer operations matter.

### 5. Architecture Isolation

Architecture-specific implementations are kept separate from the general boot infrastructure.

This allows platform-dependent functionality, such as Linux kernel loading and physical memory management, to be implemented without coupling it to unrelated boot subsystems.

### 6. Explicit Physical Memory Management

Physical memory allocation is implemented as an independent subsystem rather than relying on implicit allocation behavior.

The physical page allocator tracks available memory ranges and active allocations explicitly.

Allocation requests provide control over alignment, address boundaries, and allocation strategies.

This design provides a foundation for deterministic physical memory placement during the native Linux boot process.

---

## Technology Stack

| Component | Technology |
|---|---|
| Programming language | Rust |
| Rust edition | 2024 |
| Firmware interface | UEFI |
| UEFI crate | `uefi` 0.35 |
| Runtime | `no_std` |
| Graphics | Custom software renderer |
| UI | Custom UI engine |
| Storage | UEFI protocols |
| Partition scheme | GPT |
| Input | UEFI keyboard |
| Memory management | Custom physical page allocator |
| Physical page size | 4 KiB |
| Target architecture | x86_64 |
| Linux support | Kernel parsing, Boot Protocol infrastructure, and physical kernel placement |

## Development Status

SIOS Bootloader is under active development.

The following table distinguishes existing implementations from planned functionality.

| Component | Status |
|---|---|
| UEFI application entry point | Implemented |
| UEFI runtime initialization | Implemented |
| Storage discovery | Implemented |
| Block I/O abstraction | Implemented |
| Filesystem volume discovery | Implemented |
| GPT parsing | Implemented |
| Device Path parsing | Implemented |
| EFI image loading | Implemented |
| EFI chainloading | Implemented |
| Linux kernel image parser (x86_64) | Implemented |
| Linux Boot Protocol infrastructure | Implemented |
| Linux `boot_params` structures | Implemented |
| Kernel command-line infrastructure | Implemented |
| x86_64 address abstractions | Implemented |
| Physical memory region representation | Implemented |
| Physical memory range abstraction | Implemented |
| Physical page allocator | Implemented |
| Configurable physical page allocation | Implemented |
| Physical memory range reservation | Implemented |
| Physical page deallocation | Implemented |
| Custom framebuffer renderer | Implemented |
| Graphics and text rendering | Implemented |
| UI tree | Implemented |
| Layout engine | Implemented |
| Widget system | Implemented |
| Keyboard navigation | Implemented |
| Dirty-region rendering | Implemented |
| Kernel physical memory placement | Implemented |
| Initrd loading and placement | Planned |
| UEFI Memory Map processing | Planned |
| E820 memory map generation | Planned |
| Page table construction | Planned |
| UEFI ExitBootServices integration | Planned |
| Native Linux kernel handoff | Planned |

*Implementation status refers to the current development snapshot and does not imply production readiness.*

---

## Roadmap

The long-term goal is to turn SIOS Bootloader into a complete native boot environment.

### Phase 1 - Firmware Infrastructure

- [x] UEFI runtime initialization
- [x] Storage device discovery
- [x] Filesystem discovery
- [x] GPT partition parsing
- [x] Device Path handling
- [x] EFI executable loading
- [x] EFI chainloading

### Phase 2 - Graphics and UI

- [x] Framebuffer abstraction
- [x] Custom renderer
- [x] Bitmap font rendering
- [x] Text rendering
- [x] UI element tree
- [x] Layout engine
- [x] Styling system
- [x] Widget system
- [x] Keyboard navigation
- [x] Partial framebuffer updates

### Phase 3 - Native Linux Boot (x86_64)

- [x] Linux kernel image parsing
- [x] Linux Boot Protocol infrastructure
- [x] Linux `boot_params` structures
- [x] Kernel command-line infrastructure
- [x] x86_64 address abstractions
- [x] Physical memory region representation
- [x] Physical page allocator
- [x] Physical memory range reservation
- [x] Physical page deallocation
- [x] Kernel physical memory placement
- [ ] Initrd loading and placement
- [ ] UEFI Memory Map processing
- [ ] E820 memory map generation
- [ ] x86_64 page table construction
- [ ] UEFI ExitBootServices integration
- [ ] Native Linux kernel handoff

### Phase 4 - Advanced Boot Environment

- [ ] Persistent boot configuration
- [ ] Boot entry autodetection
- [ ] Advanced boot options
- [ ] Recovery environment
- [ ] Improved diagnostics
- [ ] Extended hardware support

---

## Building

The project uses Cargo.

A compatible Rust toolchain and the appropriate UEFI target are required.

Update Rust:

```bash
rustup update
```

Install the UEFI target:

```bash
rustup target add x86_64-unknown-uefi
```

Build the UEFI binary:

```bash
cargo build --release --target x86_64-unknown-uefi
```

The resulting EFI executable can be found under the corresponding Cargo target directory.

**Note:** The exact build configuration, target features, and firmware setup depend on the project's current Cargo configuration.

## Testing

The bootloader is intended to be tested in a UEFI environment.

Recommended testing environments:

- QEMU with OVMF firmware.
- UEFI-capable virtual machines.
- Physical x86_64 hardware with UEFI firmware.

Always test experimental bootloader changes in a virtual machine before using them on physical hardware.

---

## Part of the SIOS Ecosystem

SIOS Bootloader is developed as part of **SIOS**, an independent operating system project.

The broader SIOS ecosystem aims to provide a custom Linux-based operating system with independently developed system components and desktop infrastructure.

SIOS Bootloader is intended to become the foundation of its boot infrastructure.

---

## License

License information will be added when the project license is finalized.

---

<p align="center">
  <strong>SIOS Bootloader</strong>
  <br>
  <em>Built from the firmware up.</em>
  <br><br>
  Copyright 2026 SIOS Corporation
</p>