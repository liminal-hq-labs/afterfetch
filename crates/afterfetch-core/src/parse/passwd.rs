// `/etc/passwd`: `name:password:uid:gid:gecos:home:shell`. Used to name the user when `$USER` and `$LOGNAME` are not set.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

use super::leading_number;

/// The user name on `line` when its uid is `uid`.
pub fn name_for_uid(line: &[u8], uid: u32) -> Option<&[u8]> {
	let mut fields = line.split(|&byte| byte == b':');
	let name = fields.next().filter(|name| !name.is_empty() && name[0] != b'#')?;
	let _password = fields.next()?;
	let field = fields.next()?;
	let number = leading_number(field)?;
	(number == u64::from(uid) && field.iter().all(u8::is_ascii_digit)).then_some(name)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn a_matching_uid_names_the_user() {
		assert_eq!(
			name_for_uid(b"scott:x:1000:1000:Scott:/home/scott:/bin/zsh", 1000),
			Some(&b"scott"[..])
		);
		assert_eq!(name_for_uid(b"root:x:0:0:root:/root:/bin/bash", 0), Some(&b"root"[..]));
	}

	#[test]
	fn other_uids_comments_and_short_lines_do_not_match() {
		assert_eq!(name_for_uid(b"scott:x:1000:1000::/home/scott:/bin/zsh", 1001), None);
		assert_eq!(name_for_uid(b"# scott:x:1000:1000", 1000), None);
		assert_eq!(name_for_uid(b"scott:x", 1000), None);
		assert_eq!(name_for_uid(b"odd:x:10a:1::/:/bin/sh", 10), None);
		assert_eq!(name_for_uid(b":x:5:5", 5), None);
	}
}
