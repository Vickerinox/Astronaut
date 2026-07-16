// SPDX-FileCopyrightText: 2026 Viktor Karlsson <viktor@koda.re>
// SPDX-License-Identifier: MIT

pub struct File {
    path: alloc::string::String,
    contents: Option<alloc::vec::Vec<u8>>,
    metadata: Option<alloc::vec::Vec<u8>>,
}
