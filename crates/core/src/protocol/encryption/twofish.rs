/*
    Copyright (c) 2017 Alexander Krotov

    Permission is hereby granted, free of charge, to any
    person obtaining a copy of this software and associated
    documentation files (the "Software"), to deal in the
    Software without restriction, including without
    limitation the rights to use, copy, modify, merge,
    publish, distribute, sublicense, and/or sell copies of
    the Software, and to permit persons to whom the Software
    is furnished to do so, subject to the following
    conditions:

    The above copyright notice and this permission notice
    shall be included in all copies or substantial portions
    of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
    ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
    TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
    PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
    SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
    CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
    OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
    IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
    DEALINGS IN THE SOFTWARE.
*/

pub const QORD: [[usize; 5]; 4] = [
    [1, 1, 0, 0, 1],
    [0, 1, 1, 0, 0],
    [0, 0, 0, 1, 1],
    [1, 0, 1, 1, 0],
];

#[rustfmt::skip]
pub const QBOX: [[[u8; 16]; 4]; 2] = [
    [
        [
            0x8, 0x1, 0x7, 0xD, 0x6, 0xF, 0x3, 0x2,
            0x0, 0xB, 0x5, 0x9, 0xE, 0xC, 0xA, 0x4,
        ], [
            0xE, 0xC, 0xB, 0x8, 0x1, 0x2, 0x3, 0x5,
            0xF, 0x4, 0xA, 0x6, 0x7, 0x0, 0x9, 0xD,
        ], [
            0xB, 0xA, 0x5, 0xE, 0x6, 0xD, 0x9, 0x0,
            0xC, 0x8, 0xF, 0x3, 0x2, 0x4, 0x7, 0x1,
        ], [
            0xD, 0x7, 0xF, 0x4, 0x1, 0x2, 0x6, 0xE,
            0x9, 0xB, 0x3, 0x0, 0x8, 0x5, 0xC, 0xA,
        ],
    ], [
        [
            0x2, 0x8, 0xB, 0xD, 0xF, 0x7, 0x6, 0xE,
            0x3, 0x1, 0x9, 0x4, 0x0, 0xA, 0xC, 0x5,
        ], [
            0x1, 0xE, 0x2, 0xB, 0x4, 0xC, 0x3, 0x7,
            0x6, 0xD, 0xA, 0x5, 0xF, 0x9, 0x0, 0x8,
        ], [
            0x4, 0xC, 0x7, 0x5, 0x1, 0x6, 0x9, 0xA,
            0x0, 0xE, 0xD, 0x8, 0x2, 0xB, 0x3, 0xF,
        ], [
            0xB, 0x9, 0x5, 0x1, 0xC, 0x3, 0xD, 0xE,
            0x6, 0x4, 0x7, 0xF, 0x2, 0x0, 0x8, 0xA,
        ],
    ]
];

pub const RS: [[u8; 8]; 4] = [
    [0x01, 0xa4, 0x55, 0x87, 0x5a, 0x58, 0xdb, 0x9e],
    [0xa4, 0x56, 0x82, 0xf3, 0x1e, 0xc6, 0x68, 0xe5],
    [0x02, 0xa1, 0xfc, 0xc1, 0x47, 0xae, 0x3d, 0x19],
    [0xa4, 0x55, 0x87, 0x5a, 0x58, 0xdb, 0x9e, 0x03],
];

// 0x169 (x⁸ + x⁶ + x⁵ + x³ + 1)
pub const MDS_POLY: u8 = 0x69;
// 0x14d (x⁸ + x⁶ + x³ + x² + 1)
pub const RS_POLY: u8 = 0x4d;

/// Twofish block cipher
#[derive(Clone)]
pub struct Twofish {
    s: [u8; 16],  // S-box key
    k: [u32; 40], // Subkeys
    start: usize,
}

fn gf_mult(mut a: u8, mut b: u8, p: u8) -> u8 {
    let mut result = 0;
    while a > 0 {
        if a & 1 == 1 {
            result ^= b;
        }
        a >>= 1;
        if b & 0x80 == 0x80 {
            b = (b << 1) ^ p;
        } else {
            b <<= 1;
        }
    }
    result
}

// q_i sbox
fn sbox(i: usize, x: u8) -> u8 {
    let (a0, b0) = (x >> 4 & 15, x & 15);
    let a1 = a0 ^ b0;
    let b1 = (a0 ^ ((b0 << 3) | (b0 >> 1)) ^ (a0 << 3)) & 15;
    let (a2, b2) = (QBOX[i][0][a1 as usize], QBOX[i][1][b1 as usize]);
    let a3 = a2 ^ b2;
    let b3 = (a2 ^ ((b2 << 3) | (b2 >> 1)) ^ (a2 << 3)) & 15;
    let (a4, b4) = (QBOX[i][2][a3 as usize], QBOX[i][3][b3 as usize]);
    (b4 << 4) + a4
}

fn mds_column_mult(x: u8, column: usize) -> u32 {
    let x5b = gf_mult(x, 0x5b, MDS_POLY);
    let xef = gf_mult(x, 0xef, MDS_POLY);

    let v = match column {
        0 => [x, x5b, xef, xef],
        1 => [xef, xef, x5b, x],
        2 => [x5b, xef, x, xef],
        3 => [x5b, x, xef, x5b],
        _ => unreachable!(),
    };
    u32::from_le_bytes(v)
}

fn mds_mult(y: [u8; 4]) -> u32 {
    let mut z = 0;
    for i in 0..4 {
        z ^= mds_column_mult(y[i], i);
    }
    z
}

fn rs_mult(m: &[u8], out: &mut [u8]) {
    for i in 0..4 {
        out[i] = 0;
        for j in 0..8 {
            out[i] ^= gf_mult(m[j], RS[i][j], RS_POLY);
        }
    }
}

#[allow(clippy::many_single_char_names)]
fn h(x: u32, m: &[u8], k: usize, offset: usize) -> u32 {
    let mut y = x.to_le_bytes();

    if k == 4 {
        y[0] = sbox(1, y[0]) ^ m[4 * (6 + offset)];
        y[1] = sbox(0, y[1]) ^ m[4 * (6 + offset) + 1];
        y[2] = sbox(0, y[2]) ^ m[4 * (6 + offset) + 2];
        y[3] = sbox(1, y[3]) ^ m[4 * (6 + offset) + 3];
    }

    if k >= 3 {
        y[0] = sbox(1, y[0]) ^ m[4 * (4 + offset)];
        y[1] = sbox(1, y[1]) ^ m[4 * (4 + offset) + 1];
        y[2] = sbox(0, y[2]) ^ m[4 * (4 + offset) + 2];
        y[3] = sbox(0, y[3]) ^ m[4 * (4 + offset) + 3];
    }

    let a = 4 * (2 + offset);
    let b = 4 * offset;
    y[0] = sbox(1, sbox(0, sbox(0, y[0]) ^ m[a]) ^ m[b]);
    y[1] = sbox(0, sbox(0, sbox(1, y[1]) ^ m[a + 1]) ^ m[b + 1]);
    y[2] = sbox(1, sbox(1, sbox(0, y[2]) ^ m[a + 2]) ^ m[b + 2]);
    y[3] = sbox(0, sbox(1, sbox(1, y[3]) ^ m[a + 3]) ^ m[b + 3]);

    mds_mult(y)
}

impl Twofish {
    pub fn new() -> Twofish {
        Self {
            s: [0u8; 16],
            k: [0u32; 40],
            start: 0,
        }
    }

    fn g_func(&self, x: u32) -> u32 {
        let mut result: u32 = 0;
        for y in 0..4 {
            let mut g = sbox(QORD[y][self.start], (x >> (8 * y)) as u8);

            for z in self.start + 1..5 {
                g ^= self.s[4 * (z - self.start - 1) + y];
                g = sbox(QORD[y][z], g);
            }

            result ^= mds_column_mult(g, y);
        }
        result
    }

    pub fn key_schedule(&mut self, key: &[u8]) {
        let k = key.len() / 8;

        let rho: u32 = 0x1010101;

        for x in 0..20 {
            let a = h(rho * (2 * x), key, k, 0);
            let b = h(rho * (2 * x + 1), key, k, 1).rotate_left(8);
            let v = a.wrapping_add(b);
            self.k[(2 * x) as usize] = v;
            self.k[(2 * x + 1) as usize] = (v.wrapping_add(b)).rotate_left(9);
        }
        self.start = match k {
            4 => 0,
            3 => 1,
            2 => 2,
            _ => unreachable!(),
        };

        // Compute S_i.
        for i in 0..k {
            rs_mult(&key[i * 8..i * 8 + 8], &mut self.s[i * 4..(i + 1) * 4]);
        }
    }

    pub fn encrypt(&mut self, b: &mut [u8]) {
        let mut p = [
            u32::from_le_bytes(b[0..4].try_into().unwrap()),
            u32::from_le_bytes(b[4..8].try_into().unwrap()),
            u32::from_le_bytes(b[8..12].try_into().unwrap()),
            u32::from_le_bytes(b[12..16].try_into().unwrap()),
        ];

        // Input whitening
        for i in 0..4 {
            p[i] ^= self.k[i];
        }

        for r in 0..8 {
            let k = 4 * r + 8;

            let t1 = self.g_func(p[1].rotate_left(8));
            let t0 = self.g_func(p[0]).wrapping_add(t1);
            p[2] = (p[2] ^ (t0.wrapping_add(self.k[k]))).rotate_right(1);
            let t2 = t1.wrapping_add(t0).wrapping_add(self.k[k + 1]);
            p[3] = p[3].rotate_left(1) ^ t2;

            let t1 = self.g_func(p[3].rotate_left(8));
            let t0 = self.g_func(p[2]).wrapping_add(t1);
            p[0] = (p[0] ^ (t0.wrapping_add(self.k[k + 2]))).rotate_right(1);
            let t2 = t1.wrapping_add(t0).wrapping_add(self.k[k + 3]);
            p[1] = (p[1].rotate_left(1)) ^ t2;
        }

        // Undo last swap and output whitening
        p[2] ^= self.k[4];
        p[3] ^= self.k[5];
        p[0] ^= self.k[6];
        p[1] ^= self.k[7];

        b[0..4].copy_from_slice(&p[2].to_le_bytes());
        b[4..8].copy_from_slice(&p[3].to_le_bytes());
        b[8..12].copy_from_slice(&p[0].to_le_bytes());
        b[12..16].copy_from_slice(&p[1].to_le_bytes());
    }
}

/*
impl KeyInit for Twofish {
    #[inline]
    fn new(key: &Key<Self>) -> Self {
        Self::new_from_slice(key).unwrap()
    }


}
*/

/*
cipher::impl_simple_block_encdec!(
    Twofish, U16, cipher, block,
    encrypt: {

    }
    decrypt: {
        let b = block.get_in();
        let mut c = [
            u32::from_le_bytes(b[8..12].try_into().unwrap()) ^ cipher.k[6],
            u32::from_le_bytes(b[12..16].try_into().unwrap()) ^ cipher.k[7],
            u32::from_le_bytes(b[0..4].try_into().unwrap()) ^ cipher.k[4],
            u32::from_le_bytes(b[4..8].try_into().unwrap()) ^ cipher.k[5],
        ];

        for r in (0..8).rev() {
            let k = 4 * r + 8;

            let t1 = cipher.g_func(c[3].rotate_left(8));
            let t0 = cipher.g_func(c[2]).wrapping_add(t1);
            c[0] = c[0].rotate_left(1) ^ (t0.wrapping_add(cipher.k[k + 2]));
            let t2 = t1.wrapping_add(t0).wrapping_add(cipher.k[k + 3]);
            c[1] = (c[1] ^ t2).rotate_right(1);

            let t1 = cipher.g_func(c[1].rotate_left(8));
            let t0 = cipher.g_func(c[0]).wrapping_add(t1);
            c[2] = c[2].rotate_left(1) ^ (t0.wrapping_add(cipher.k[k]));
            let t2 = t1.wrapping_add(t0).wrapping_add(cipher.k[k + 1]);
            c[3] = (c[3] ^ t2).rotate_right(1);
        }

        for i in 0..4 {
            c[i] ^= cipher.k[i];
        }

        let block = block.get_out();
        block[0..4].copy_from_slice(&c[0].to_le_bytes());
        block[4..8].copy_from_slice(&c[1].to_le_bytes());
        block[8..12].copy_from_slice(&c[2].to_le_bytes());
        block[12..16].copy_from_slice(&c[3].to_le_bytes());
    }
);
 */
