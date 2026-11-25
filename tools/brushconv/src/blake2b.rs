//! BLAKE2b-512, as the app names a brush's own head and texture: the digest of the picture's bytes,
//! in hex. Only this one size is needed, so the parameter block is fixed rather than general.

/// The SHA-512 initialisation vector BLAKE2b starts from.
const IV: [u64; 8] = [
    0x6a09e667f3bcc908,
    0xbb67ae8584caa73b,
    0x3c6ef372fe94f82b,
    0xa54ff53a5f1d36f1,
    0x510e527fade682d1,
    0x9b05688c2b3e6c1f,
    0x1f83d9abfb41bd6b,
    0x5be0cd19137e2179,
];

/// The round schedules: the ten distinct rows, reused for the last two rounds.
const SIGMA: [[usize; 16]; 12] = [
    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
    [14, 10, 4, 8, 9, 15, 13, 6, 1, 12, 0, 2, 11, 7, 5, 3],
    [11, 8, 12, 0, 5, 2, 15, 13, 10, 14, 3, 6, 7, 1, 9, 4],
    [7, 9, 3, 1, 13, 12, 11, 14, 2, 6, 5, 10, 4, 0, 15, 8],
    [9, 0, 5, 7, 2, 4, 10, 15, 14, 1, 11, 12, 6, 8, 3, 13],
    [2, 12, 6, 10, 0, 11, 8, 3, 4, 13, 7, 5, 15, 14, 1, 9],
    [12, 5, 1, 15, 14, 13, 4, 10, 0, 7, 6, 3, 9, 2, 8, 11],
    [13, 11, 7, 14, 12, 1, 3, 9, 5, 0, 15, 4, 8, 6, 2, 10],
    [6, 15, 14, 9, 11, 3, 0, 8, 12, 2, 13, 7, 1, 4, 10, 5],
    [10, 2, 8, 4, 7, 6, 1, 5, 15, 11, 9, 14, 3, 12, 13, 0],
    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
    [14, 10, 4, 8, 9, 15, 13, 6, 1, 12, 0, 2, 11, 7, 5, 3],
];

/// The round's eight quarter-rounds, each on four named words.
macro_rules! round {
    ($a0:ident, $b0:ident, $c0:ident, $d0:ident, $x0:expr, $y0:expr;
     $a1:ident, $b1:ident, $c1:ident, $d1:ident, $x1:expr, $y1:expr;
     $a2:ident, $b2:ident, $c2:ident, $d2:ident, $x2:expr, $y2:expr;
     $a3:ident, $b3:ident, $c3:ident, $d3:ident, $x3:expr, $y3:expr;
     $a4:ident, $b4:ident, $c4:ident, $d4:ident, $x4:expr, $y4:expr;
     $a5:ident, $b5:ident, $c5:ident, $d5:ident, $x5:expr, $y5:expr;
     $a6:ident, $b6:ident, $c6:ident, $d6:ident, $x6:expr, $y6:expr;
     $a7:ident, $b7:ident, $c7:ident, $d7:ident, $x7:expr, $y7:expr) => {{
        quarter!($a0, $b0, $c0, $d0, $x0, $y0);
        quarter!($a1, $b1, $c1, $d1, $x1, $y1);
        quarter!($a2, $b2, $c2, $d2, $x2, $y2);
        quarter!($a3, $b3, $c3, $d3, $x3, $y3);
        quarter!($a4, $b4, $c4, $d4, $x4, $y4);
        quarter!($a5, $b5, $c5, $d5, $x5, $y5);
        quarter!($a6, $b6, $c6, $d6, $x6, $y6);
        quarter!($a7, $b7, $c7, $d7, $x7, $y7);
    }};
}

/// One quarter-round on four words.
macro_rules! quarter {
    ($a:ident, $b:ident, $c:ident, $d:ident, $x:expr, $y:expr) => {{
        $a = $a.wrapping_add($b).wrapping_add($x);
        $d = ($d ^ $a).rotate_right(32);
        $c = $c.wrapping_add($d);
        $b = ($b ^ $c).rotate_right(24);
        $a = $a.wrapping_add($b).wrapping_add($y);
        $d = ($d ^ $a).rotate_right(16);
        $c = $c.wrapping_add($d);
        $b = ($b ^ $c).rotate_right(63);
    }};
}

/// The digest, in the lowercase hex the app writes.
pub fn hex(bytes: &[u8]) -> String {
    digest(bytes).iter().map(|byte| format!("{byte:02x}")).collect()
}

/// The 64-byte digest.
pub fn digest(bytes: &[u8]) -> [u8; 64] {
    let mut state = IV;
    // A 64-byte, keyless digest: the parameter block XORs its first word.
    if let Some(first) = state.first_mut() {
        *first ^= 0x0101_0000 ^ 64;
    }
    let mut count: u128 = 0;
    let (blocks, rest) = bytes.as_chunks::<128>();
    for block in blocks {
        count += 128;
        compress(&mut state, block, count, false);
    }
    let mut last = [0u8; 128];
    if let Some(head) = last.get_mut(..rest.len()) {
        head.copy_from_slice(rest);
    }
    count += rest.len() as u128;
    compress(&mut state, &last, count, true);
    let mut out = [0u8; 64];
    for (word, bytes) in state.iter().zip(out.as_chunks_mut::<8>().0) {
        bytes.copy_from_slice(&word.to_le_bytes());
    }
    out
}

/// One 128-byte block into the state.
fn compress(state: &mut [u64; 8], block: &[u8], count: u128, last: bool) {
    let mut message = [0u64; 16];
    for (word, bytes) in message.iter_mut().zip(block.as_chunks::<8>().0) {
        let mut eight = [0u8; 8];
        if let Some(fill) = eight.get_mut(..bytes.len()) {
            fill.copy_from_slice(bytes);
        }
        *word = u64::from_le_bytes(eight);
    }
    let [s0, s1, s2, s3, s4, s5, s6, s7] = state;
    let [i0, i1, i2, i3, i4, i5, i6, i7] = IV;
    let (mut v0, mut v1, mut v2, mut v3) = (*s0, *s1, *s2, *s3);
    let (mut v4, mut v5, mut v6, mut v7) = (*s4, *s5, *s6, *s7);
    let (mut v8, mut v9, mut v10, mut v11) = (i0, i1, i2, i3);
    let (mut v12, mut v13, mut v14, mut v15) = (i4 ^ count as u64, i5 ^ (count >> 64) as u64, i6, i7);
    if last {
        v14 = !v14;
    }
    for sigma in &SIGMA {
        let [a, b, c, d, e, f, g, h, i, j, k, l, m, n, o, p] = *sigma;
        let at = |index: usize| message.get(index).copied().unwrap_or_default();
        round!(
            v0, v4, v8, v12, at(a), at(b);
            v1, v5, v9, v13, at(c), at(d);
            v2, v6, v10, v14, at(e), at(f);
            v3, v7, v11, v15, at(g), at(h);
            v0, v5, v10, v15, at(i), at(j);
            v1, v6, v11, v12, at(k), at(l);
            v2, v7, v8, v13, at(m), at(n);
            v3, v4, v9, v14, at(o), at(p)
        );
    }
    *s0 ^= v0 ^ v8;
    *s1 ^= v1 ^ v9;
    *s2 ^= v2 ^ v10;
    *s3 ^= v3 ^ v11;
    *s4 ^= v4 ^ v12;
    *s5 ^= v5 ^ v13;
    *s6 ^= v6 ^ v14;
    *s7 ^= v7 ^ v15;
}

#[cfg(test)]
mod tests {
    use super::hex;

    #[test]
    fn the_empty_input_matches_the_specification() {
        assert_eq!(
            hex(b""),
            "786a02f742015903c6c6fd852552d272912f4740e15847618a86e217f71f5419d25e1031afee585313896444934eb04b903a685b1448b755d56f701afe9be2ce"
        );
    }

    #[test]
    fn a_short_input_matches_the_specification() {
        assert_eq!(
            hex(b"abc"),
            "ba80a53f981c4d0d6a2797b69f12f6e94c212f14685ac4b74b12bb6fdbffa2d17d87c5392aab792dc252d5de4533cc9518d38aa8dbf1925ab92386edd4009923"
        );
    }

    #[test]
    fn a_pangram_matches_the_specification() {
        assert_eq!(
            hex(b"The quick brown fox jumps over the lazy dog"),
            "a8add4bdddfd93e4877d2746e62817b116364a1fa7bc148d95090bc7333b3673f82401cf7aa2e4cb1ecd90296e3f14cb5413f8ed77be73045b13914cdcd6a918"
        );
    }

    #[test]
    fn more_than_one_block_matches_the_specification() {
        assert_eq!(
            hex(&[b'a'; 200]),
            "932355851d75f09c18646a9da87c25e055bc57f113121ad1ec63d45e7a1d62ab9133f8b7d1d7de9e0afa784eb6a8a11d78683013d0a672611f17668d9577d209"
        );
    }
}
