// SPDX-FileCopyrightText: 2026 Viktor Karlsson <viktor@koda.re>
// SPDX-License-Identifier: MIT

//! This module attempts to add various type checks that otherwise can't be enforced without unneccesary overhead. 
//! This is very similar to how bytemucks "must cast" system works, except a little less stringent and meant to be used with
//! components of reboot_lib like the SWI functions.

pub(crate) struct TypeCheck<A,B>(A,B);
impl<A, B> TypeCheck<A, B> {
  pub(crate) const ASSERT_ALIGN_GREATER_THAN_EQUAL: () =
    assert!(core::mem::align_of::<A>() >= core::mem::align_of::<B>());
  pub(crate) const ASSERT_SIZE_MULTIPLE_OF: () = 
    assert!(core::mem::size_of::<A>() % core::mem::size_of::<B>() == 0);
}
