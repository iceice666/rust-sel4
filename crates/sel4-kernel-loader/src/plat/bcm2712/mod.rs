//
// Copyright 2023, Colias Group, LLC
//
// SPDX-License-Identifier: BSD-2-Clause
//

use embedded_hal_nb::nb;
use embedded_hal_nb::serial::Write;
use spin::lock_api::Mutex;

use sel4_config::sel4_cfg_bool;
use sel4_pl011_driver::Driver as Pl011Driver;

use crate::{arch::reset_cntvoff, plat::Plat};

// UART10 on the Raspberry Pi 5's dedicated debug header, which
// `src/plat/bcm2712/overlay-rpi5.dts` designates as this platform's
// `seL4,elfloader-devices` console via the `serial10` alias. The node is
// `/soc@107c000000/serial@7d001000`, `compatible = "arm,pl011"`, whose
// firmware-final physical address is the SoC base plus the node offset.
const SERIAL_DEVICE_BASE_ADDR: usize = 0x10_7d00_1000;

static SERIAL_DRIVER: Mutex<Pl011Driver> = Mutex::new(get_serial_driver());

const fn get_serial_driver() -> Pl011Driver {
    unsafe { Pl011Driver::new_uninit(SERIAL_DEVICE_BASE_ADDR as *mut _) }
}

pub(crate) enum PlatImpl {}

impl Plat for PlatImpl {
    fn init() {
        SERIAL_DRIVER.lock().init();
    }

    fn init_per_core() {
        if sel4_cfg_bool!(ARM_HYPERVISOR_SUPPORT) {
            unsafe {
                reset_cntvoff();
            }
        }
    }

    fn put_char(c: u8) {
        nb::block!(SERIAL_DRIVER.lock().write(c)).unwrap_or_else(|err| match err {});
    }

    fn put_char_without_synchronization(c: u8) {
        nb::block!(get_serial_driver().write(c)).unwrap_or_else(|err| match err {});
    }

    fn start_secondary_core(core_id: usize, sp: usize) {
        // The BCM2712 boots secondary cores through PSCI, exposed by the
        // firmware and named in this platform's `seL4,elfloader-devices`
        // alongside the console. That is the RPi4's spin-table replacement:
        // `tools/dts/rpi5b.dts` declares a `/psci` node with method
        // `smc`, and carries no spin table for the loader to poke.
        crate::arch::drivers::psci::start_secondary_core(core_id, sp)
    }
}
