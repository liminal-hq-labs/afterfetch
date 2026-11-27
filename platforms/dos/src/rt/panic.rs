// The panic handler: exit with the code a shell uses when it crashes. A second panic cannot happen, since nothing here panics.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

use core::panic::PanicInfo;

#[panic_handler]
fn panic(_info: &PanicInfo<'_>) -> ! {
	crate::rt::dos::exit(afterfetch_core::run::EXIT_CRASH)
}
