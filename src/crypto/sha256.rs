const K: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

#[inline]
fn rotr(x: u32, n: u32) -> u32 {
    x.rotate_right(n)
}

#[inline]
fn ch(x: u32, y: u32, z: u32) -> u32 {
    (x & y) ^ (!x & z)
}

#[inline]
fn maj(x: u32, y: u32, z: u32) -> u32 {
    (x & y) ^ (x & z) ^ (y & z)
}

#[inline]
fn ep0(x: u32) -> u32 {
    rotr(x, 2) ^ rotr(x, 13) ^ rotr(x, 22)
}

#[inline]
fn ep1(x: u32) -> u32 {
    rotr(x, 6) ^ rotr(x, 11) ^ rotr(x, 25)
}

#[inline]
fn sig0(x: u32) -> u32 {
    rotr(x, 7) ^ rotr(x, 18) ^ (x >> 3)
}

#[inline]
fn sig1(x: u32) -> u32 {
    rotr(x, 17) ^ rotr(x, 19) ^ (x >> 10)
}

pub struct Sha256 {
    state: [u32; 8],
    data: [u8; 64],
    datalen: usize,
    bitlen: u64,
}

impl Default for Sha256 {
    fn default() -> Self {
        Self {
            state: [
                0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
                0x5be0cd19,
            ],
            data: [0u8; 64],
            datalen: 0,
            bitlen: 0,
        }
    }
}

impl Sha256 {
    pub fn new() -> Self {
        Self::default()
    }

    fn transform(&mut self) {
        let mut m: [u32; 64] = [0; 64];

        for i in 0..16 {
            let j = i * 4;
            m[i] = u32::from_be_bytes([
                self.data[j],
                self.data[j + 1],
                self.data[j + 2],
                self.data[j + 3],
            ]);
        }

        for i in 16..64 {
            m[i] = sig1(m[i - 2])
                .wrapping_add(m[i - 7])
                .wrapping_add(sig0(m[i - 15]))
                .wrapping_add(m[i - 16]);
        }

        let mut a = self.state[0];
        let mut b = self.state[1];
        let mut c = self.state[2];
        let mut d = self.state[3];
        let mut e = self.state[4];
        let mut f = self.state[5];
        let mut g = self.state[6];
        let mut h = self.state[7];

        for i in 0..64 {
            let t1 = h
                .wrapping_add(ep1(e))
                .wrapping_add(ch(e, f, g))
                .wrapping_add(K[i])
                .wrapping_add(m[i]);
            let t2 = ep0(a).wrapping_add(maj(a, b, c));
            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }

        self.state[0] = self.state[0].wrapping_add(a);
        self.state[1] = self.state[1].wrapping_add(b);
        self.state[2] = self.state[2].wrapping_add(c);
        self.state[3] = self.state[3].wrapping_add(d);
        self.state[4] = self.state[4].wrapping_add(e);
        self.state[5] = self.state[5].wrapping_add(f);
        self.state[6] = self.state[6].wrapping_add(g);
        self.state[7] = self.state[7].wrapping_add(h);
    }

    pub fn update(&mut self, data: &[u8]) {
        for &byte in data {
            self.data[self.datalen] = byte;
            self.datalen += 1;
            if self.datalen == 64 {
                self.transform();
                self.bitlen += 512;
                self.datalen = 0;
            }
        }
    }

    pub fn finalize(&mut self) -> [u8; 32] {
        let i = self.datalen;

        if self.datalen < 56 {
            self.data[i] = 0x80;
            for j in (i + 1)..56 {
                self.data[j] = 0;
            }
        } else {
            self.data[i] = 0x80;
            for j in (i + 1)..64 {
                self.data[j] = 0;
            }
            self.transform();
            for j in 0..56 {
                self.data[j] = 0;
            }
        }

        self.bitlen += (self.datalen as u64) * 8;
        self.data[56] = (self.bitlen >> 56) as u8;
        self.data[57] = (self.bitlen >> 48) as u8;
        self.data[58] = (self.bitlen >> 40) as u8;
        self.data[59] = (self.bitlen >> 32) as u8;
        self.data[60] = (self.bitlen >> 24) as u8;
        self.data[61] = (self.bitlen >> 16) as u8;
        self.data[62] = (self.bitlen >> 8) as u8;
        self.data[63] = self.bitlen as u8;

        self.transform();

        let mut hash = [0u8; 32];
        for i in 0..8 {
            let bytes = self.state[i].to_be_bytes();
            hash[i * 4] = bytes[0];
            hash[i * 4 + 1] = bytes[1];
            hash[i * 4 + 2] = bytes[2];
            hash[i * 4 + 3] = bytes[3];
        }
        hash
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

pub fn double_sha256(data: &[u8]) -> [u8; 32] {
    let mut ctx = Sha256::new();
    ctx.update(data);
    let first = ctx.finalize();

    ctx.reset();
    ctx.update(&first);
    ctx.finalize()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sha256_empty() {
        let mut ctx = Sha256::new();
        ctx.update(b"");
        let hash = ctx.finalize();
        let expected: [u8; 32] = [
            0xe3, 0xb0, 0xc4, 0x42, 0x98, 0xfc, 0x1c, 0x14, 0x9a, 0xfb, 0xf4, 0xc8, 0x99, 0x6f,
            0xb9, 0x24, 0x27, 0xae, 0x41, 0xe4, 0x64, 0x9b, 0x93, 0x4c, 0xa4, 0x95, 0x99, 0x1b,
            0x78, 0x52, 0xb8, 0x55,
        ];
        assert_eq!(hash, expected);
    }

    #[test]
    fn test_sha256_abc() {
        let mut ctx = Sha256::new();
        ctx.update(b"abc");
        let hash = ctx.finalize();
        let expected: [u8; 32] = [
            0xba, 0x78, 0x16, 0xbf, 0x8f, 0x01, 0xcf, 0xea, 0x41, 0x41, 0x40, 0xde, 0x5d, 0xae,
            0x22, 0x23, 0xb0, 0x03, 0x61, 0xa3, 0x96, 0x17, 0x7a, 0x9c, 0xb4, 0x10, 0xff, 0x61,
            0xf2, 0x00, 0x15, 0xad,
        ];
        assert_eq!(hash, expected);
    }

    #[test]
    fn test_double_sha256() {
        let hash = double_sha256(b"hello");
        assert_eq!(hash.len(), 32);
    }
}
