//
// Copyright 2026, Slime OS contributors
//
// SPDX-License-Identifier: BSD-2-Clause
//

use core::ptr;

use sel4_config::sel4_cfg_bool;

use crate::{arch::reset_cntvoff, plat::Plat};

// UART0 on the Novatek NT98690 (NS02201) H1V1, the port BL31 and U-Boot print
// through and the one `src/plat/ns02201/overlay-ns02201-h1v1.dts` names in
// `seL4,elfloader-devices`. The node is `/uart@2f0130000`, `compatible =
// "ns16550a"` with `reg-shift = <2>` and `reg-io-width = <4>`, so the 16550
// transmit-holding and line-status registers sit at byte offsets 0x00 and 0x14
// and are accessed as 32-bit words. Firmware leaves it configured for 115200
// 8N1; Slime's P6.A probe printed through exactly these two registers on the
// named board, which is why no driver crate and no initialisation are needed.
const SERIAL_DEVICE_BASE_ADDR: usize = 0x2_f013_0000;
const TRANSMIT_HOLDING: usize = 0x00;
const LINE_STATUS: usize = 0x14;
const TRANSMIT_HOLDING_EMPTY: u32 = 1 << 5;

fn put_char_polled(c: u8) {
    let lsr = (SERIAL_DEVICE_BASE_ADDR + LINE_STATUS) as *const u32;
    let thr = (SERIAL_DEVICE_BASE_ADDR + TRANSMIT_HOLDING) as *mut u32;
    // SAFETY: both addresses lie in the UART0 register page the platform's
    // device tree declares, which the loader's identity map covers as device
    // memory; the reads and writes are the 32-bit accesses the hardware
    // requires and have no effect beyond the peripheral.
    unsafe {
        while ptr::read_volatile(lsr) & TRANSMIT_HOLDING_EMPTY == 0 {
            core::hint::spin_loop();
        }
        ptr::write_volatile(thr, u32::from(c));
    }
}

pub(crate) enum PlatImpl {}

impl Plat for PlatImpl {
    fn init() {}

    fn init_per_core() {
        if sel4_cfg_bool!(ARM_HYPERVISOR_SUPPORT) {
            unsafe {
                reset_cntvoff();
            }
        }
    }

    fn put_char(c: u8) {
        put_char_polled(c);
    }

    // A polled 16550 has no state a lock would protect: each call waits for
    // the holding register to drain and writes one byte, so the unsynchronised
    // path is the same path.
    fn put_char_without_synchronization(c: u8) {
        put_char_polled(c);
    }

    fn start_secondary_core(core_id: usize, sp: usize) {
        // TF-A 2.2 implements PSCI 0.2 CPU_ON for this SoC; single-node
        // kernels never reach here.
        crate::arch::drivers::psci::start_secondary_core(core_id, sp)
    }
}
