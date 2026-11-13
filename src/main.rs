#![no_main]
#![no_std]

use uefi::{Status, entry};

use sios_bootloader::runtime;

#[entry]
fn main() -> Status {
    runtime::run()
}
