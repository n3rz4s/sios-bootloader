#![no_main]
#![no_std]

use core::fmt::Write;
use uefi::{Status, boot, entry, system};


fn log(message: &str) {
    system::with_stdout(|stdout| {
        writeln!(stdout, "{}", message).unwrap();
    });
}

#[entry]
fn main() -> Status {
    log("SIOS Bootloader v0.1.0");
    log("");
    log("-----------------------------------");

    boot::stall(300_000);

    log("");
    log("[ OK ] Booting...");

    bot::stall(1_000_000);

    log("");
    log("");
    log("-----------------------------------");
    log("BOOT STATUS: SUCCESS.");
    log("");
    log("Welcome to SIOS.");
    log("");
    log("Control transfered to EFI application.");
    log("SIOS Bootloader is no longer in control.");
    log("");
    log("Thank you for using SIOS.");

    log("");
    log("");
    log("-----------------------------------");

    boot::stall(5_000_000);

    log("Hading over control...");
    boot::stall(600_000);
    log("Goodbye.");
    boot::stall(1_000_000);
    
    Status::SUCCESS
}
