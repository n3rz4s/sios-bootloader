# SIOS Bootloader

\<p align="center">
&#x20; \<strong>A modern UEFI bootloader written entirely in Rust.\</strong>
&#x20; \<br>
&#x20; \<em>Built from the ground up with a custom graphics renderer, declarative UI engine, storage subsystem, EFI image loader, and native Linux boot infrastructure.\</em>
\</p>

\<p align="center">
&#x20; \<img src="[https://img.shields.io/badge/Language-Rust-orange?style=for-the-badge&logo=rust](https://img.shields.io/badge/Language-Rust-orange?style=for-the-badge\&logo=rust)" alt="Rust">
&#x20; \<img src="[https://img.shields.io/badge/Firmware-UEFI-blue?style=for-the-badge](https://img.shields.io/badge/Firmware-UEFI-blue?style=for-the-badge)" alt="UEFI">
&#x20; \<img src="[https://img.shields.io/badge/Architecture-x86__64-informational?style=for-the-badge](https://img.shields.io/badge/Architecture-x86__64-informational?style=for-the-badge)" alt="x86_64">
&#x20; \<img src="[https://img.shields.io/badge/Environment-no__std-purple?style=for-the-badge](https://img.shields.io/badge/Environment-no__std-purple?style=for-the-badge)" alt="no_std">
&#x20; \<img src="[https://img.shields.io/badge/Status-Active%20Development-yellow?style=for-the-badge](https://img.shields.io/badge/Status-Active%20Development-yellow?style=for-the-badge)" alt="Active Development">
\</p>

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

### Linux Boot Protocol

SIOS Bootloader includes the initial implementation of the Linux Boot Protocol infrastructure for x86_64.

This subsystem provides the structures and interfaces required to prepare Linux kernel boot parameters.

- Linux Boot Protocol structures.
- `boot_params` representation.
- Linux boot header integration.
- Kernel command-line support.
- Boot protocol module organization.
- Architecture-specific boot infrastructure.

This subsystem builds upon the Linux kernel image parser and establishes the foundation for subsequent native kernel loading stages.

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
│       ├── mod.rs
│       └── linux/
│           ├── boot_params.rs
│           ├── command_line.rs
│           ├── error.rs
│           ├── header.rs
│           ├── image.rs
│           ├── mod.rs
│           └── protocol.rs
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

Contains functionality specific to the x86_64 architecture.

**`boot/x86_64/linux/`**

Contains the Linux-specific boot implementation for x86_64.

The Linux subsystem includes separate modules for kernel image parsing, Linux Boot Protocol structures, boot parameters, and command-line handling.

This separation allows architecture-specific boot functionality to evolve independently without introducing unnecessary dependencies into the general boot management infrastructure.

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

This allows platform-dependent functionality, such as Linux kernel loading, to be implemented without coupling it to unrelated boot subsystems.

---

## Technology Stack

| Component            | Technology                                      |
| -------------------- | ----------------------------------------------- |
| Programming language | Rust                                            |
| Rust edition         | 2024                                            |
| Firmware interface   | UEFI                                            |
| UEFI crate           | `uefi` 0.35                                     |
| Runtime              | `no_std`                                        |
| Graphics             | Custom software renderer                        |
| UI                   | Custom UI engine                                |
| Storage              | UEFI protocols                                  |
| Partition scheme     | GPT                                             |
| Input                | UEFI keyboard                                   |
| Target architecture  | x86_64                                          |
| Linux support        | Kernel parsing and Boot Protocol infrastructure |

## Development Status

SIOS Bootloader is under active development.

The following table distinguishes existing implementations from planned functionality.

| Component                          | Status      |
| ---------------------------------- | ----------- |
| UEFI application entry point       | Implemented |
| UEFI runtime initialization        | Implemented |
| Storage discovery                  | Implemented |
| Block I/O abstraction              | Implemented |
| Filesystem volume discovery        | Implemented |
| GPT parsing                        | Implemented |
| Device Path parsing                | Implemented |
| EFI image loading                  | Implemented |
| EFI chainloading                   | Implemented |
| Linux kernel image parser (x86_64) | Implemented |
| Linux Boot Protocol infrastructure | Implemented |
| Linux `boot_params` structures     | Implemented |
| Kernel command-line infrastructure | Implemented |
| Custom framebuffer renderer        | Implemented |
| Graphics and text rendering        | Implemented |
| UI tree                            | Implemented |
| Layout engine                      | Implemented |
| Widget system                      | Implemented |
| Keyboard navigation                | Implemented |
| Dirty-region rendering             | Implemented |
| Physical memory allocator          | Planned     |
| Kernel physical memory placement   | Planned     |
| Initrd loading and placement       | Planned     |
| UEFI Memory Map processing         | Planned     |
| E820 memory map generation         | Planned     |
| Page table construction            | Planned     |
| UEFI ExitBootServices integration  | Planned     |
| Native Linux kernel handoff        | Planned     |

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
- [ ] Physical page allocator
- [ ] Kernel physical memory placement
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

\<p align="center">   \<strong>SIOS Bootloader\</strong>   \<br>   \<em>Built from the firmware up.\</em>   \<br>\<br>   Copyright 2026 SIOS Corporation \</p>
