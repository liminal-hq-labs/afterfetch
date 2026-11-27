// Links AFETCH.DMD as a module: it starts at `app_start` and has no `app_event` or `app_stop`.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

include!("../vendor/link.rs");

fn main() {
	emit_link_arguments("app_start", &[]);
}
