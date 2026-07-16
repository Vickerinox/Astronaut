// SPDX-FileCopyrightText: 2026 Viktor Karlsson <viktor@koda.re>
// SPDX-License-Identifier: GPL-3.0-or-later

pub fn _crc16(mut value: u16, buffer: &[u8]) -> u16 {
    /*
    val[0..7] = C0C1h,C181h,C301h,C601h,CC01h,D801h,F001h,A001h
    for i=start to end
        crc=crc xor byte[i]
        for j=0 to 7
        crc=crc shr 1:if carry then crc=crc xor (val[j] shl (7-j))
        next j
    next i
    */
    let vals = [
        0xC0C1, 0xC181, 0xC301, 0xC601, 0xCC01, 0xD801, 0xF001, 0xA001,
    ];
    for byte in buffer {
        value ^= *byte as u16;
        for i in 0..8 {
            value >>= 1;
            if value & 0x1 != 0 {
                value = value ^ (vals[i] << (7 - i))
            };
        }
    }
    value
}
