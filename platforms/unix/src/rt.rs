// The runtime: the Rust side of the entry stub, the arguments and environment from the initial stack, the memory routines the compiler
// expects a C library to supply, and the panic handler.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

use core::ffi::c_void;
use core::ptr;

use crate::os::system;

/// The command-line arguments after the program's own name, as byte strings.
pub struct Args {
	next: *const *const u8,
}

impl Iterator for Args {
	type Item = &'static [u8];

	fn next(&mut self) -> Option<&'static [u8]> {
		// SAFETY: `next` walks the kernel's NUL-terminated argv array and stops at its null pointer.
		unsafe {
			let text = *self.next;
			if text.is_null() {
				return None;
			}
			self.next = self.next.add(1);
			Some(c_bytes(text))
		}
	}
}

/// The bytes of a NUL-terminated string.
///
/// # Safety
/// `text` must point to a NUL-terminated string that lives for the whole run.
pub unsafe fn c_bytes(text: *const u8) -> &'static [u8] {
	let mut length = 0;
	// SAFETY: the caller guarantees the terminator, so every read is inside the string.
	unsafe {
		// Volatile, so the loop stays a loop and is not turned into a call to `strlen`, which does not exist here.
		while ptr::read_volatile(text.add(length)) != 0 {
			length += 1;
		}
		core::slice::from_raw_parts(text, length)
	}
}

/// Called by the entry stub with the stack pointer the kernel set: `argc`, then `argv`, a null, then `envp`, a null.
pub extern "C" fn start(stack: *const usize) -> ! {
	// SAFETY: the kernel lays the stack out as described above.
	let (args, environment) = unsafe {
		let argc = *stack;
		let argv = stack.add(1).cast::<*const u8>();
		(Args { next: argv.add(1) }, argv.add(argc + 1))
	};
	let code = crate::os::system::main(args, environment);
	system::exit(code)
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo<'_>) -> ! {
	system::exit(afterfetch_core::run::EXIT_CRASH)
}

// The compiler emits calls to these for copies and fills, and the target has no C library to provide them. Volatile access keeps the
// optimiser from turning the loops back into calls to themselves.

/// # Safety
/// The regions must be valid and must not overlap.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn memcpy(destination: *mut c_void, source: *const c_void, count: usize) -> *mut c_void {
	let (destination_bytes, source_bytes) = (destination.cast::<u8>(), source.cast::<u8>());
	let mut at = 0;
	// SAFETY: the caller guarantees both regions are valid for `count` bytes.
	unsafe {
		while at < count {
			ptr::write_volatile(destination_bytes.add(at), ptr::read_volatile(source_bytes.add(at)));
			at += 1;
		}
	}
	destination
}

/// # Safety
/// The regions must be valid; they may overlap.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn memmove(destination: *mut c_void, source: *const c_void, count: usize) -> *mut c_void {
	let (destination_bytes, source_bytes) = (destination.cast::<u8>(), source.cast::<u8>());
	// SAFETY: the caller guarantees both regions are valid for `count` bytes.
	unsafe {
		if destination_bytes.cast_const() <= source_bytes || destination_bytes.cast_const() >= source_bytes.add(count) {
			return memcpy(destination, source, count);
		}
		let mut at = count;
		while at > 0 {
			at -= 1;
			ptr::write_volatile(destination_bytes.add(at), ptr::read_volatile(source_bytes.add(at)));
		}
	}
	destination
}

/// # Safety
/// The region must be valid for `count` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn memset(destination: *mut c_void, value: i32, count: usize) -> *mut c_void {
	let destination_bytes = destination.cast::<u8>();
	let mut at = 0;
	// SAFETY: the caller guarantees the region is valid for `count` bytes.
	unsafe {
		while at < count {
			ptr::write_volatile(destination_bytes.add(at), value as u8);
			at += 1;
		}
	}
	destination
}

/// # Safety
/// Both regions must be valid for `count` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn memcmp(first: *const c_void, second: *const c_void, count: usize) -> i32 {
	let (first, second) = (first.cast::<u8>(), second.cast::<u8>());
	let mut at = 0;
	// SAFETY: the caller guarantees both regions are valid for `count` bytes.
	unsafe {
		while at < count {
			let (left, right) = (ptr::read_volatile(first.add(at)), ptr::read_volatile(second.add(at)));
			if left != right {
				return i32::from(left) - i32::from(right);
			}
			at += 1;
		}
	}
	0
}

/// # Safety
/// Both regions must be valid for `count` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bcmp(first: *const c_void, second: *const c_void, count: usize) -> i32 {
	// SAFETY: the same contract as `memcmp`.
	unsafe { memcmp(first, second, count) }
}
