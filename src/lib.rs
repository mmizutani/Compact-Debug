//! `{:#?}` formatting, and the `dbg!()` macro, sound nice on paper. But once you try using them...
//!
//! ```text
//! Goto(
//!     Address(
//!         30016,
//!     ),
//! ),
//! Label(
//!     Address(
//!         29990,
//!     ),
//! ),
//! Expr(
//!     Expr(
//!         Expr(
//!             [
//!                 Var(
//!                     0,
//!                 ),
//!                 Const(
//!                     0,
//!                 ),
//!                 Op(
//!                     Ne,
//!                 ),
//!             ],
//!         ),
//!     ),
//!     Address(
//!         30016,
//!     ),
//! ),
//! ```
//!
//! Your dreams of nice and readable output are shattered by a chunk of output more porous than cotton
//! candy, with approximately two tokens of content on each line. Screenful upon screenful of vacuous
//! output for even a moderately complex type. Upset, you reluctantly replace your derived `Debug`
//! implementation with a manual one that eschews `DebugTuple` in favor of `write_str`. However, this
//! results in a catastrophic amount of boilerplate code, and doesn't affect types outside of your
//! control, like the ubiquitous `Option`.
//!
//! That's where this crate comes in. It monkey-patches the pretty-printing machinery so that
//! `DebugTuple` is printed on a single line regardless of `#` flag. The above snippet is printed as:
//!
//! ```text
//! Goto(Address(30016)),
//! Label(Address(29990)),
//! Expr(Expr(Expr([
//!     Var(0),
//!     Const(0),
//!     Op(Ne),
//! ])), Address(30016)),
//! ```
//!
//! This crate currently only supports x86_64 and aarch64 architectures.

#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
compile_error!("only supported on x86_64 and aarch64");

#[cfg(target_arch = "x86_64")]
const ORIGINAL: [u8; 2] = [0x75, 0x3E]; // jne 0x40
#[cfg(target_arch = "x86_64")]
const PATCHED: [u8; 2] = [0x66, 0x90]; // nop

#[cfg(target_arch = "aarch64")]
// const ORIGINAL: [u8; 4] = [0x54, 0x00, 0x00, 0x00]; // Example B.NE instruction
// const ORIGINAL: [u8; 4] = [0x76, 0xcb, 0x75, 0x04]; // B.NE instruction
const ORIGINAL: [u8; 4] = [0x00, 0xD0, 0x08, 0x05]; // 0, 208, 8, 5
#[cfg(target_arch = "aarch64")]
const PATCHED: [u8; 4] = [0x1F, 0x20, 0x03, 0xD5]; // NOP instruction (0xD503201F)

/// Enables or disables the patch.
///
/// # Panics
/// Panics if the function does not look like expected, which is most likely to happen if `std`
/// changes something internally, or if the compiler finds a better way to optimize it.
///
/// # Safety
/// Aside from the whole concept being inherently unsafe, this will probably have unexpected
/// consequences if called in multi-threaded contexts.
pub unsafe fn enable(on: bool) {
	unsafe {
		let function = std::fmt::DebugTuple::field as *const () as *const u8;
		#[cfg(target_arch = "x86_64")]
		let ptr = function.offset(0x46) as *mut [u8; 2];
		#[cfg(target_arch = "aarch64")]
		let ptr = function.offset(0x46) as *mut [u8; 4];
		if !matches!(*ptr, ORIGINAL | PATCHED) {
			panic!("DebugTuple::field is not as expected")
		}
		let size = std::mem::size_of_val(&ORIGINAL);
		let _prot =
			region::protect_with_handle(ptr, size, region::Protection::READ_WRITE_EXECUTE).unwrap();
		ptr.write(if on { PATCHED } else { ORIGINAL });
	}
}

#[test]
fn test() {
	#[derive(Debug)]
	struct A(u32, u32);

	#[allow(dead_code)]
	#[derive(Debug)]
	struct B {
		x: u32,
		y: u32,
	}

	let a = A(8, 32);
	let b = B { x: 8, y: 32 };

	assert_eq!(format!("{a:?}"), "A(8, 32)");
	assert_eq!(format!("{a:#?}"), "A(\n    8,\n    32,\n)");
	assert_eq!(format!("{b:?}"), "B { x: 8, y: 32 }");
	assert_eq!(format!("{b:#?}"), "B {\n    x: 8,\n    y: 32,\n}");

	unsafe { enable(true) };

	assert_eq!(format!("{a:?}"), "A(8, 32)");
	assert_eq!(format!("{a:#?}"), "A(8, 32)");
	assert_eq!(format!("{b:?}"), "B { x: 8, y: 32 }");
	assert_eq!(format!("{b:#?}"), "B {\n    x: 8,\n    y: 32,\n}");

	unsafe { enable(false) };

	assert_eq!(format!("{a:?}"), "A(8, 32)");
	assert_eq!(format!("{a:#?}"), "A(\n    8,\n    32,\n)");
	assert_eq!(format!("{b:?}"), "B { x: 8, y: 32 }");
	assert_eq!(format!("{b:#?}"), "B {\n    x: 8,\n    y: 32,\n}");
}
