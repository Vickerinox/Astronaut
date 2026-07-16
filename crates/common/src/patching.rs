// SPDX-FileCopyrightText: 2026 Viktor Karlsson <viktor@koda.re>
// SPDX-License-Identifier: MIT

use crate::bootstrap::TWLHeader;

const LAUNCHER_ARM9_PATCH: VPatch<'static> = VPatch {
    blocks: &[
        VBlock {
            original_len: 0x08,
            patch_len: 0x02,
            offset: 0x0c,
        },
        VBlock {
            original_len: 0x05,
            patch_len: 0x02,
            offset: 0x0a,
        },
        VBlock {
            original_len: 0x05,
            patch_len: 0x01,
            offset: 0x08,
        },
        VBlock {
            original_len: 0x07,
            patch_len: 0x01,
            offset: 0x0c,
        },
        VBlock {
            original_len: 0x07,
            patch_len: 0x01,
            offset: 0x0c,
        },
        VBlock {
            original_len: 0x06,
            patch_len: 0x02,
            offset: 0x0c,
        },
        VBlock {
            original_len: 0x0c,
            patch_len: 0x02,
            offset: 0x00,
        },
        VBlock {
            original_len: 0x08,
            patch_len: 0x02,
            offset: 0xfffcu16 as i16,
        },
        VBlock {
            original_len: 0x0c,
            patch_len: 0x02,
            offset: 0x10,
        },
        VBlock {
            original_len: 0x13,
            patch_len: 0x02,
            offset: 0x22,
        },
        VBlock {
            original_len: 0x0d,
            patch_len: 0x02,
            offset: 0x08,
        },
        VBlock {
            original_len: 0x07,
            patch_len: 0x02,
            offset: 0x00,
        },
        VBlock {
            original_len: 0x10,
            patch_len: 0x02,
            offset: 0x00,
        },
        VBlock {
            original_len: 0x04,
            patch_len: 0x01,
            offset: 0x04,
        },
        VBlock {
            original_len: 0x07,
            patch_len: 0x06,
            offset: 0x0e,
        },
        VBlock {
            original_len: 0x0a,
            patch_len: 0x02,
            offset: 0x14,
        },
    ],
    originals: &[
        0x1000, 0xe3a0, 0xa4, 0xe59f, 0x2b01, 0xe3a0, 0xaaaa, 0xeb00, 0x4b0e, 0xa800, 0xa903,
        0x1c2a, 0x3380, 0x4b09, 0xa812, 0xa90d, 0x1c32, 0x47a0, 0x2201, 0x1c11, 0x4081, 0x9802,
        0x4208, 0xd100, 0x2200, 0x2101, 0x1c0a, 0x4082, 0x1c38, 0x4210, 0xd100, 0x2100, 0x223e,
        0x192, 0xa800, 0xa903, 0x18b2, 0x1c23, 0xb5f0, 0xb085, 0x1c04, 0xaaaa, 0xaaaa, 0x8920,
        0x700, 0xf40, 0x2803, 0xd10d, 0x6820, 0x6861, 0x2800, 0xd105, 0x2017, 0xaaaa, 0xaaaa,
        0xb028, 0x2000, 0xbd08, 0x201e, 0xaaaa, 0xaaaa, 0x2000, 0xbd08, 0xf7ff, 0xaaaa, 0xbd08,
        0xf7ff, 0xaaaa, 0xbd08, 0x2020, 0x2a03, 0xaaaa, 0xf7ff, 0xaaaa, 0xaaaa, 0x7f49, 0x789,
        0xfc9, 0xd104, 0x201e, 0xaaaa, 0xaaaa, 0x2000, 0xaaaa, 0xf7ff, 0xaaaa, 0xaaaa, 0xf7ff,
        0xaaaa, 0x2009, 0xaaaa, 0xaaaa, 0xbd10, 0xaaaa, 0xaaaa, 0x2800, 0xd103, 0x200b, 0xaaaa,
        0xaaaa, 0xbd10, 0x8920, 0xb570, 0xb0d4, 0x1c05, 0x1c0c, 0xa823, 0x21e9, 0x1c16, 0xb5f0,
        0xb0ff, 0xb0ff, 0xb0ff, 0xb0ff, 0xb0ff, 0xb08e, 0x1c05, 0x4847, 0x1c0f, 0x4468, 0x6944,
        0x4846, 0x8a6, 0x9202, 0x9303, 0xdf27, 0x4770, 0xdf28, 0x4770, 0x2079, 0x80, 0x4285,
        0xd302, 0xaaaa, 0x4285, 0xd905, 0x2201, 0x312, 0xaaaa, 0xaaaa, 0x4809, 0x4b0c, 0x6802,
        0x480a, 0xa900, 0x1810,
    ],
    patches: &[
        0x00, 0xe1a0, 0x46c0, 0x2001, 0x46c0, 0x46c0, 0x46c0, 0x46c0, 0x2001, 0x2001, 0x4770,
        0x46c0, 0x2001, 0x46c0, 0x2001, 0x46c0, 0x2001, 0x46c0, 0x2001, 0x2000, 0x4770, 0x2000,
        0x4770, 0x2001, 0x2582, 0xad, 0x46c0, 0x46c0, 0x46c0, 0x46c0, 0x46c0, 0x2001,
    ],
};
#[derive(Debug)]
pub struct VPatch<'a> {
    blocks: &'a [VBlock],
    originals: &'a [u16],
    patches: &'a [u16],
}
#[derive(Debug)]
pub struct VBlock {
    original_len: u16,
    patch_len: u16,
    offset: i16,
}
#[derive(Debug, PartialEq)]
pub enum VPatchResult {
    Ok,
    BinaryRanOut,
    BadPatch,
    MatchRanOut,
    PatchRanOut,
    MalformedPatch,
}
fn app_vlaunch_patch(l_words: &mut [u16], patch: &VPatch) -> VPatchResult {
    let VPatch {
        blocks,
        mut originals,
        mut patches,
    } = patch;
    let mut l_cursor = 0;
    for block in blocks.iter() {
        let Some((orig, remainder)) = originals.split_at_checked(block.original_len as usize)
        else {
            return VPatchResult::MatchRanOut;
        };
        originals = remainder;
        let Some((patch, remainder)) = patches.split_at_checked(block.patch_len as usize) else {
            return VPatchResult::PatchRanOut;
        };
        patches = remainder;

        loop {
            loop {
                let Some(word) = l_words.get(l_cursor) else {
                    return VPatchResult::BinaryRanOut;
                };
                let Some(word2) = orig.get(0) else {
                    return VPatchResult::BadPatch;
                };
                if word == word2 {
                    break;
                } else {
                    l_cursor += 1;
                }
            }
            let match_length = l_words[l_cursor..]
                .iter()
                .zip(orig.iter())
                .filter(|(a, b)| (**b == 0xAAAA) || (**b == **a))
                .count();
            if match_length == orig.len() {
                break;
            } else {
                l_cursor += match_length
            }
        }
        let patch_cursor = l_cursor.wrapping_add_signed((block.offset as isize) / 2);
        for (src, dst) in patch.iter().zip(&mut l_words[patch_cursor..]) {
            *dst = *src
        }
    }
    if originals.is_empty() && patches.is_empty() {
        VPatchResult::Ok
    } else {
        VPatchResult::MalformedPatch
    }
}
pub unsafe fn look_for_launcher_patch(header: &TWLHeader) {
    if header.title_id & !0xFF == 0x00030017_484E4100 {
        let binary = core::slice::from_raw_parts_mut(
            header.head.arm9_load as *mut u16,
            header.head.arm9_size as usize / 2,
        );
        if app_vlaunch_patch(binary, &LAUNCHER_ARM9_PATCH) != VPatchResult::Ok {};
    }
}
