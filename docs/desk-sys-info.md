# A `desk_sys_info` call for Desk

A proposal for the Desk project (AF-D26). It is written here, in afterfetch, because afterfetch is the first module that wants it, and is meant to be copied into Desk's SDK documents or rejected there. Nothing in this repository depends on it being accepted.

## Why

A Desk module may not touch hardware, so `AFETCH.DMD` can print only what its build knows: `Desk SDK 1 (native, i486)`. A system-information program needs the rest: how long the machine has been up, how much memory there is, what the processor is, and what the display is doing. Desk owns all of that, and can answer without giving a module a port or an interrupt.

## The call

One service, imported like `desk_log` through a slot in `.desk.imp`:

```rust
desk_guest::import!(pub fn desk_sys_info(out: *mut SysInfo, size: u32) -> i32);
```

`out` points at a `SysInfo` the module owns and `size` is its size in bytes, so a later Desk can add fields at the end and an older module still gets the prefix it knows. The result is 0 when Desk filled the structure and negative on an error. Desk fills at most `size` bytes and sets `written` to the count.

```rust
#[repr(C)]
pub struct SysInfo {
	/// Bytes of this structure Desk filled in; a field past it is not valid.
	pub written: u32,
	/// The SDK version Desk speaks, as `desk_sdk_version` would.
	pub sdk: u16,
	/// 0 for the native kernel, 1 for the hosted track.
	pub track: u16,
	/// Milliseconds since Desk started.
	pub uptime_ms: u64,
	/// Usable and free RAM, in bytes.
	pub memory_total: u64,
	pub memory_free: u64,
	/// The processor's brand string from `CPUID` where it has one (NUL-padded), else empty.
	pub cpu_brand: [u8; 48],
	/// 486, 586 or 686 for the family, 0 when unknown.
	pub cpu_family: u16,
	/// The display mode: width, height and bits per pixel; 0 when Desk has no graphics mode.
	pub display_width: u16,
	pub display_height: u16,
	pub display_bits: u8,
}
```

## What afterfetch would do with it

Fill the Uptime, CPU, Memory and Display rows (SPEC section 4) from the structure, leave a row out when its field is 0 or empty (AF-D20), and take the track from `track` instead of a build feature, so one `AFETCH.DMD` serves both tracks.

## What it costs Desk

One import in the service table, and a function in each loader's core (native and hosted) that copies its own numbers. No new capability: the module already may log, allocate and panic through the core.
