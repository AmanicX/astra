use crate::error::{AppError, Result};
use crate::settings::SpeechSettings;
use base64::Engine;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceInfo {
    pub short_name: String,
    pub locale: String,
    pub gender: String,
    pub friendly: String,
}

/// A curated set of Microsoft Edge neural voices (a practical default set;
/// the full catalog is fetched live when available).
pub fn curated_voices() -> Vec<VoiceInfo> {
    let rows = [
        ("en-US-AriaNeural", "en-US", "Female", "Aria — warm, natural (default)"),
        ("en-US-JennyNeural", "en-US", "Female", "Jenny — friendly, conversational"),
        ("en-US-GuyNeural", "en-US", "Male", "Guy — calm, measured"),
        ("en-US-DavisNeural", "en-US", "Male", "Davis — deep, confident"),
        ("en-GB-SoniaNeural", "en-GB", "Female", "Sonia — British, composed"),
        ("en-GB-RyanNeural", "en-GB", "Male", "Ryan — British, clear"),
        ("en-AU-NatashaNeural", "en-AU", "Female", "Natasha — Australian"),
        ("de-DE-KatjaNeural", "de-DE", "Female", "Katja — German"),
        ("fr-FR-DeniseNeural", "fr-FR", "Female", "Denise — French"),
        ("es-ES-ElviraNeural", "es-ES", "Female", "Elvira — Spanish"),
        ("hi-IN-SwaraNeural", "hi-IN", "Female", "Swara — Hindi"),
        ("ja-JP-NanamiNeural", "ja-JP", "Female", "Nanami — Japanese"),
        ("zh-CN-XiaoxiaoNeural", "zh-CN", "Female", "Xiaoxiao — Mandarin"),
    ];
    rows.iter()
        .map(|(s, l, g, f)| VoiceInfo {
            short_name: s.to_string(),
            locale: l.to_string(),
            gender: g.to_string(),
            friendly: f.to_string(),
        })
        .collect()
}

const TRUSTED_CLIENT_TOKEN: &str = "6A5AA1D4EAFF4E9FB37E23D68491D6F4";
const SEC_MS_GEC_VERSION: &str = "2-143.0.3650.75";
const EDGE_ORIGIN: &str = "chrome-extension://jdiccldimpdaibmpdkjnbmckianbfold";
const EDGE_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/143.0.0.0 Safari/537.36 Edg/143.0.0.0";
/// Windows FILETIME epoch (1601-01-01) in Unix seconds.
const WIN_EPOCH: i64 = 11_644_473_600;

/// Server/client clock skew in seconds, learned from a rejected handshake.
static CLOCK_SKEW: std::sync::Mutex<i64> = std::sync::Mutex::new(0);

/// Chromium-style date format the Edge TTS service expects in message headers.
fn edge_date() -> String {
    chrono::Utc::now()
        .format("%a %b %d %Y %H:%M:%S GMT+0000 (Coordinated Universal Time)")
        .to_string()
}

fn hex_upper(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push_str(&format!("{b:02X}"));
    }
    out
}

fn rol16(v: u32) -> u32 {
    v.rotate_left(16)
}

fn words_le4_even(data: &[u8]) -> Vec<u32> {
    let mut padded = data.to_vec();
    let rem = padded.len() % 4;
    if rem != 0 {
        padded.resize(data.len() + (4 - rem), 0);
    }
    if padded.is_empty() {
        return Vec::new();
    }
    let mut words: Vec<u32> = padded
        .chunks_exact(4)
        .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect();
    if words.len() % 2 != 0 {
        words.push(0);
    }
    words
}

const CS64_COEFFICIENTS: [(u32, u32); 14] = [
    (0xE82E91B1, 0xB32700D0),
    (0x44DE24C6, 0x88DFF51D),
    (0x3BEB65BC, 0x29514038),
    (0xC89C5933, 0xB328CBDF),
    (0x8D1EC211, 0xD405B4D5),
    (0x314EF5D5, 0x26E55F3C),
    (0xE92491B1, 0x607801C6),
    (0xF0A5B9D5, 0x82B2C1FB),
    (0x06064CF3, 0xFA51A3DA),
    (0xDA79FA60, 0x207CC464),
    (0xBED20560, 0x3C3C74F3),
    (0x89F9C9F8, 0xD2D14B61),
    (0x35FA38C6, 0xBB04A2C1),
    (0xCB49E3C6, 0xC5626167),
];
const CS64_FACTOR: u32 = 0x82893933;
const CS64_ADJ0: u32 = 0xF2F90B1A;
const CS64_ADJ1: u32 = 0x0B1A0000;

/// CS64 keyed digest, ported from the reference Edge TTS DRM implementation.
fn cs64_digest(data: &[u8], digest: &[u8; 32]) -> [u8; 8] {
    const M: u64 = 0xFFFF_FFFF;
    let mask = |v: u64| (v & M) as u32;
    let mul = |a: u32, b: u32| ((a as u64 * b as u64) & M) as u32;

    let key_a = u32::from_le_bytes([digest[0], digest[1], digest[2], digest[3]]) | 1;
    let key_b = u32::from_le_bytes([digest[4], digest[5], digest[6], digest[7]]) | 1;
    let fac_a = key_a.wrapping_mul(CS64_FACTOR);
    let fac_b = key_b.wrapping_mul(CS64_FACTOR);

    let mut terms: Vec<u64> = Vec::with_capacity(48);
    let mut offsets: Vec<u64> = Vec::with_capacity(48);
    for (i, &(coef, off)) in CS64_COEFFICIENTS.iter().enumerate() {
        let f = if i % 2 == 0 { fac_a } else { fac_b };
        terms.push(coef as u64 * f as u64 & M);
        offsets.push(off as u64);
    }

    let mut msg = Vec::with_capacity(data.len() + 1);
    msg.push(0u8);
    msg.extend_from_slice(data);
    for (i, word) in words_le4_even(&msg).into_iter().enumerate() {
        let even = i % 2 == 0;
        let key = if even { key_a } else { key_b };
        let mut term = word as u64 * key as u64;
        let mut off = rol16(word) as u64 * if even { 0xEF0569FB } else { 0xC31713DB };
        if i == 0 {
            term += CS64_ADJ0 as u64 * fac_a as u64;
            off += CS64_ADJ1 as u64;
        }
        terms.push(term & M);
        offsets.push(off & M);
    }

    let (mut swap_acc, mut swap_last, mut state, mut acc) = (0u64, 0u64, 0u64, 0u64);
    let mut i = 0;
    while i < terms.len() {
        let x = terms[i];
        let y = terms[i + 1];

        let t = rol16(mask(offsets[i] + x));
        let q = mul(t, 0x76F50F34);
        swap_acc += (rol16(q) as u64 * 0xC3EFEA97 + q as u64) & M;
        let t = rol16(mask(offsets[i + 1] + y));
        let q = mul(t, 0x1881CE3C);
        swap_last = (rol16(q) as u64 * 0x35BD1EC9 + q as u64) & M;
        swap_acc = (swap_acc + swap_last) & M;

        state = (state * key_a as u64 + x) & M;
        state = (rol16(state as u32) as u64 * 0xCF98B111) & M;
        state = (rol16(state as u32) as u64 * 0x87085B9F) & M;
        state = (rol16(state as u32) as u64 * 0x12CEB96D) & M;
        state = (rol16(state as u32) as u64 * 0x257E1D83) & M;
        acc += state;

        let t = (state * key_b as u64 + y) & M;
        let t = (rol16(t as u32) as u64 * 0xA27416F5) & M;
        let t = (rol16(t as u32) as u64 * 0xD38396FF) & M;
        let inner = ((rol16(t as u32) as u64 * 0x7C932B89) & M) as u32;
        state = (rol16(inner) as u64 * 0xBFA49F69) & M;
        acc = (acc + state) & M;

        i += 2;
    }

    let mut out = [0u8; 8];
    out[0..4].copy_from_slice(&((swap_last as u32) ^ (state as u32)).to_le_bytes());
    out[4..8].copy_from_slice(&((swap_acc as u32) ^ (acc as u32)).to_le_bytes());
    out
}

/// The Edge TTS DRM does not use plain SHA-256: the reference implementation
/// seeds the state from the first seven bytes of the token, substitutes one
/// round constant, and folds two sigma adjustments into the schedule. We port
/// that variant exactly so the challenge matches the service (and the Python
/// reference, which the tests pin).
const EDGE_SHA_INITIAL: [u32; 8] = [
    0x6A09E667, 0xBB67AE85, 0x3C6EF372, 0xA54FF53A, 0x510E527F, 0x9B05688C, 0x1F83D9AB, 0x5BE0CD19,
];
const EDGE_SHA_STATE: [u32; 8] = [
    0xFD9FBEA2, 0x91243CE7, 0xBE499094, 0xCD2E1E0A, 0x6C006F49, 0x04BD35AB, 0xFEB0A7E4, 0x03EFFBD2,
];
const EDGE_SHA_SCHEDULE: [u32; 14] = [
    0xEB24BF9E, 0x1558691F, 0xA0C09EE2, 0x0CFA32BA, 0xA622EE8C, 0xD1DD9491, 0xC0DD25A0, 0xF9E70AB6,
    0xF0C31573, 0x7058CE4E, 0x7188E606, 0x00E5AB33, 0xD258B060, 0x72766564,
];
const EDGE_SHA_ROUND_CONSTANTS: [u32; 64] = [
    0x428A2F98, 0x71374491, 0xB5C0FBCF, 0xE9B5DBA5, 0x3956C25B, 0x59F111F1, 0x923F82A4, 0xAB1C5ED5,
    0xD807AA98, 0x12835B01, 0x243185BE, 0x550C7DC3, 0x72BE5D74, 0x80DEB1FE, 0x9BDC06A7, 0xC19BF174,
    0xE49B69C1, 0xEFBE4786, 0x0FC19DC6, 0x240CA1CC, 0x2DE92C6F, 0x4A7484AA, 0x5CB0A9DC, 0x76F988DA,
    0x983E5152, 0xA831C66D, 0xB00327C8, 0xBF597FC7, 0xC6E00BF3, 0xD5A79147, 0x06CA6351, 0x14292967,
    0x27B70A85, 0x2E1B2138, 0x4D2C6DFC, 0x53380D13, 0x650A7354, 0x766A0ABB, 0x81C2C92E, 0x92722C85,
    0xA2BFE8A1, 0xA81A664B, 0xC24B8B70, 0xC76C51A3, 0xD192E819, 0xD6990624, 0xF40E3585, 0x106AA070,
    0x19A4C116, 0x1E376C08, 0x2748774C, 0x34B0BCB5, 0x391C0CB3, 0x4ED8AA4A, 0x5B9CCA4F, 0x682E6FF3,
    0x748F82EE, 0x78A5636F, 0x84C87814, 0x8CC70208, 0x90BEFFFA, 0xA4506CEB, 0xBEF9A3F7, 0xC67178F2,
];
const EDGE_SHA_ADJUSTMENT: u32 = 0xC9DC06A7;
const EDGE_SHA_SIGMA0_ADJUSTMENT: u32 = 0x059C0B80;
const EDGE_SHA_SIGMA1_ADJUSTMENT: u32 = 0x000B92C0;
const EDGE_SHA_BLOCK_CONSTANTS: [u32; 64] = {
    let mut c = EDGE_SHA_ROUND_CONSTANTS;
    c[14] = EDGE_SHA_ADJUSTMENT;
    c
};

const U64_MASK: u64 = 0xFFFF_FFFF;

/// SHA-256 small sigma zero, truncated once (masked variant used by init).
fn edge_sigma0(value: u32) -> u32 {
    (value >> 7) ^ (value << 25) ^ (value >> 18) ^ (value << 14) ^ (value >> 3)
}

/// SHA-256 small sigma one, truncated once (masked variant used by init).
fn edge_sigma1(value: u32) -> u32 {
    (value >> 17) ^ (value << 15) ^ (value >> 19) ^ (value << 13) ^ (value >> 10)
}

/// SHA-256 message schedule expansion. Rotations keep their high bits until
/// the final masked sum; low 32 bits are unaffected, so u32 wrapping is exact.
fn edge_expand_schedule(words: &mut Vec<u32>, first_word: usize) {
    for index in first_word..64 {
        let x = words[index - 2];
        let y = words[index - 15];
        let sigma1 = (x >> 17) ^ (x << 15) ^ (x >> 19) ^ (x << 13) ^ (x >> 10);
        let sigma0 = (y >> 7) ^ (y << 25) ^ (y >> 18) ^ (y << 14) ^ (y >> 3);
        let value = sigma1
            .wrapping_add(words[index - 7])
            .wrapping_add(sigma0)
            .wrapping_add(words[index - 16]);
        words.push(value);
    }
}

/// SHA-256 rounds, resuming inside the first block when `first_round` is 14.
fn edge_sha256_rounds(state: &[u32; 8], words: &[u32], first_round: usize) -> [u32; 8] {
    let (mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h) =
        (state[0], state[1], state[2], state[3], state[4], state[5], state[6], state[7]);
    let constants: &[u32; 64] = if first_round == 14 {
        &EDGE_SHA_BLOCK_CONSTANTS
    } else {
        &EDGE_SHA_ROUND_CONSTANTS
    };
    let aligned = std::cmp::min(64, ((first_round + 7) / 8) * 8);
    for index in first_round..aligned {
        let rot_e = (e as u64) * 0x1_0000_0001;
        let rot_a = (a as u64) * 0x1_0000_0001;
        let t1 = (h as u64)
            .wrapping_add((rot_e >> 6) ^ (rot_e >> 11) ^ (rot_e >> 25))
            .wrapping_add((g ^ (e & (f ^ g))) as u64)
            .wrapping_add(constants[index] as u64)
            .wrapping_add(words[index] as u64);
        let t2 = ((rot_a >> 2) ^ (rot_a >> 13) ^ (rot_a >> 22))
            .wrapping_add(((a & b) | (c & (a | b))) as u64);
        let next_a = (t1.wrapping_add(t2) & U64_MASK) as u32;
        let next_e = ((d as u64).wrapping_add(t1) & U64_MASK) as u32;
        (a, b, c, d, e, f, g, h) = (next_a, a, b, c, next_e, e, f, g);
    }
    for index in (aligned..64).step_by(8) {
        let rot_e = (e as u64) * 0x1_0000_0001;
        let rot_a = (a as u64) * 0x1_0000_0001;
        let t1 = (h as u64)
            .wrapping_add((rot_e >> 6) ^ (rot_e >> 11) ^ (rot_e >> 25))
            .wrapping_add((g ^ (e & (f ^ g))) as u64)
            .wrapping_add(constants[index] as u64)
            .wrapping_add(words[index] as u64);
        d = ((d as u64).wrapping_add(t1) & U64_MASK) as u32;
        h = (t1
            .wrapping_add((rot_a >> 2) ^ (rot_a >> 13) ^ (rot_a >> 22))
            .wrapping_add(((a & b) | (c & (a | b))) as u64)
            & U64_MASK) as u32;

        let rot_e = (d as u64) * 0x1_0000_0001;
        let rot_a = (h as u64) * 0x1_0000_0001;
        let t1 = (g as u64)
            .wrapping_add((rot_e >> 6) ^ (rot_e >> 11) ^ (rot_e >> 25))
            .wrapping_add((f ^ (d & (e ^ f))) as u64)
            .wrapping_add(constants[index + 1] as u64)
            .wrapping_add(words[index + 1] as u64);
        c = ((c as u64).wrapping_add(t1) & U64_MASK) as u32;
        g = (t1
            .wrapping_add((rot_a >> 2) ^ (rot_a >> 13) ^ (rot_a >> 22))
            .wrapping_add(((h & a) | (b & (h | a))) as u64)
            & U64_MASK) as u32;

        let rot_e = (c as u64) * 0x1_0000_0001;
        let rot_a = (g as u64) * 0x1_0000_0001;
        let t1 = (f as u64)
            .wrapping_add((rot_e >> 6) ^ (rot_e >> 11) ^ (rot_e >> 25))
            .wrapping_add((e ^ (c & (d ^ e))) as u64)
            .wrapping_add(constants[index + 2] as u64)
            .wrapping_add(words[index + 2] as u64);
        b = ((b as u64).wrapping_add(t1) & U64_MASK) as u32;
        f = (t1
            .wrapping_add((rot_a >> 2) ^ (rot_a >> 13) ^ (rot_a >> 22))
            .wrapping_add(((g & h) | (a & (g | h))) as u64)
            & U64_MASK) as u32;

        let rot_e = (b as u64) * 0x1_0000_0001;
        let rot_a = (f as u64) * 0x1_0000_0001;
        let t1 = (e as u64)
            .wrapping_add((rot_e >> 6) ^ (rot_e >> 11) ^ (rot_e >> 25))
            .wrapping_add((d ^ (b & (c ^ d))) as u64)
            .wrapping_add(constants[index + 3] as u64)
            .wrapping_add(words[index + 3] as u64);
        a = ((a as u64).wrapping_add(t1) & U64_MASK) as u32;
        e = (t1
            .wrapping_add((rot_a >> 2) ^ (rot_a >> 13) ^ (rot_a >> 22))
            .wrapping_add(((f & g) | (h & (f | g))) as u64)
            & U64_MASK) as u32;

        let rot_e = (a as u64) * 0x1_0000_0001;
        let rot_a = (e as u64) * 0x1_0000_0001;
        let t1 = (d as u64)
            .wrapping_add((rot_e >> 6) ^ (rot_e >> 11) ^ (rot_e >> 25))
            .wrapping_add((c ^ (a & (b ^ c))) as u64)
            .wrapping_add(constants[index + 4] as u64)
            .wrapping_add(words[index + 4] as u64);
        h = ((h as u64).wrapping_add(t1) & U64_MASK) as u32;
        d = (t1
            .wrapping_add((rot_a >> 2) ^ (rot_a >> 13) ^ (rot_a >> 22))
            .wrapping_add(((e & f) | (g & (e | f))) as u64)
            & U64_MASK) as u32;

        let rot_e = (h as u64) * 0x1_0000_0001;
        let rot_a = (d as u64) * 0x1_0000_0001;
        let t1 = (c as u64)
            .wrapping_add((rot_e >> 6) ^ (rot_e >> 11) ^ (rot_e >> 25))
            .wrapping_add((b ^ (h & (a ^ b))) as u64)
            .wrapping_add(constants[index + 5] as u64)
            .wrapping_add(words[index + 5] as u64);
        g = ((g as u64).wrapping_add(t1) & U64_MASK) as u32;
        c = (t1
            .wrapping_add((rot_a >> 2) ^ (rot_a >> 13) ^ (rot_a >> 22))
            .wrapping_add(((d & e) | (f & (d | e))) as u64)
            & U64_MASK) as u32;

        let rot_e = (g as u64) * 0x1_0000_0001;
        let rot_a = (c as u64) * 0x1_0000_0001;
        let t1 = (b as u64)
            .wrapping_add((rot_e >> 6) ^ (rot_e >> 11) ^ (rot_e >> 25))
            .wrapping_add((a ^ (g & (h ^ a))) as u64)
            .wrapping_add(constants[index + 6] as u64)
            .wrapping_add(words[index + 6] as u64);
        f = ((f as u64).wrapping_add(t1) & U64_MASK) as u32;
        b = (t1
            .wrapping_add((rot_a >> 2) ^ (rot_a >> 13) ^ (rot_a >> 22))
            .wrapping_add(((c & d) | (e & (c | d))) as u64)
            & U64_MASK) as u32;

        let rot_e = (f as u64) * 0x1_0000_0001;
        let rot_a = (b as u64) * 0x1_0000_0001;
        let t1 = (a as u64)
            .wrapping_add((rot_e >> 6) ^ (rot_e >> 11) ^ (rot_e >> 25))
            .wrapping_add((h ^ (f & (g ^ h))) as u64)
            .wrapping_add(constants[index + 7] as u64)
            .wrapping_add(words[index + 7] as u64);
        e = ((e as u64).wrapping_add(t1) & U64_MASK) as u32;
        a = (t1
            .wrapping_add((rot_a >> 2) ^ (rot_a >> 13) ^ (rot_a >> 22))
            .wrapping_add(((b & c) | (d & (b | c))) as u64)
            & U64_MASK) as u32;
    }
    [a, b, c, d, e, f, g, h]
}

/// Compute the Edge DRM modified SHA-256 state for token data.
fn edge_sha256_init(data: &[u8]) -> [u32; 8] {
    let mut words: Vec<u32> = vec![0; 14];
    words.push(u32::from_be_bytes([0, data[0], data[1], data[2]]));
    words.push(u32::from_be_bytes([data[3], data[4], data[5], data[6]]));
    for index in 16..30 {
        let mut value = EDGE_SHA_SCHEDULE[index - 16];
        if index - 2 >= 14 {
            value = value.wrapping_add(if index == 16 {
                edge_sigma1(words[index - 2]) ^ EDGE_SHA_SIGMA1_ADJUSTMENT
            } else {
                edge_sigma1(words[index - 2])
            });
        }
        if index - 7 >= 14 {
            value = value.wrapping_add(words[index - 7]);
        }
        if index - 15 >= 14 {
            value = value.wrapping_add(if index == 29 {
                edge_sigma0(words[index - 15]) ^ EDGE_SHA_SIGMA0_ADJUSTMENT
            } else {
                edge_sigma0(words[index - 15])
            });
        }
        words.push(value);
    }
    words.push(
        edge_sigma1(words[28])
            .wrapping_add(words[23])
            .wrapping_add(edge_sigma0(words[15]))
            .wrapping_add(words[14])
            .wrapping_add(EDGE_SHA_ADJUSTMENT)
            .wrapping_sub(EDGE_SHA_ROUND_CONSTANTS[14]),
    );
    edge_expand_schedule(&mut words, 31);
    let working = edge_sha256_rounds(&EDGE_SHA_STATE, &words, 14);
    let mut out = [0u32; 8];
    for i in 0..8 {
        out[i] = EDGE_SHA_INITIAL[i].wrapping_add(working[i]);
    }
    out
}

/// Compute the Edge DRM modified SHA-256 digest for token data.
fn edge_sha256_digest(data: &[u8]) -> [u8; 32] {
    assert!(data.len() >= 7, "token salt must contain at least seven bytes");
    let length = 57 + data.len();
    let mut padded: Vec<u8> = data[7..].to_vec();
    padded.push(0x80);
    let zeros = ((55i64 - length as i64).rem_euclid(64)) as usize;
    padded.extend(std::iter::repeat(0u8).take(zeros));
    padded.extend_from_slice(&((length as u64) * 8).to_be_bytes());

    let mut state = edge_sha256_init(&data[..7]);
    for chunk in padded.chunks_exact(64) {
        let mut words: Vec<u32> = (0..16)
            .map(|i| {
                u32::from_be_bytes([chunk[4 * i], chunk[4 * i + 1], chunk[4 * i + 2], chunk[4 * i + 3]])
            })
            .collect();
        edge_expand_schedule(&mut words, 16);
        let working = edge_sha256_rounds(&state, &words, 0);
        for i in 0..8 {
            state[i] = state[i].wrapping_add(working[i]);
        }
    }
    let mut out = [0u8; 32];
    for i in 0..8 {
        out[4 * i..4 * i + 4].copy_from_slice(&state[i].to_be_bytes());
    }
    out
}

/// Sec-MS-GEC v2 challenge for a Unix timestamp. Pure so tests can pin it
/// against reference vectors.
fn gec_token(now: f64, skew: i64) -> String {
    let mut ticks = now + skew as f64 + WIN_EPOCH as f64;
    ticks -= ticks % 300.0;
    ticks *= 1e7;
    let data = format!("{ticks:.0}{TRUSTED_CLIENT_TOKEN}");
    let digest = edge_sha256_digest(data.as_bytes());
    let cs64 = cs64_digest(data.as_bytes(), &digest);
    format!("{}-{}", hex_upper(&cs64), hex_upper(&digest))
}

fn generate_sec_ms_gec() -> String {
    let now = {
        let d = chrono::Utc::now();
        d.timestamp() as f64 + d.timestamp_subsec_micros() as f64 / 1_000_000.0
    };
    let skew = *CLOCK_SKEW.lock().unwrap();
    gec_token(now, skew)
}

fn generate_muid() -> String {
    uuid::Uuid::new_v4().simple().to_string().to_uppercase()
}

/// A rejected handshake carries the server's clock in its Date header; learn
/// the offset so the retried challenge lands in the right 5-minute window.
fn correct_clock_skew(err: &tokio_tungstenite::tungstenite::Error) -> bool {
    let tokio_tungstenite::tungstenite::Error::Http(resp) = err else {
        return false;
    };
    let Some(date) = resp.headers().get("date").and_then(|v| v.to_str().ok()) else {
        return false;
    };
    let Ok(server) = chrono::DateTime::parse_from_rfc2822(date) else {
        return false;
    };
    let skew = server.timestamp() - chrono::Utc::now().timestamp();
    *CLOCK_SKEW.lock().unwrap() += skew;
    log::info!("edge tts clock skew corrected by {skew}s");
    true
}

pub struct SpokenAudio {
    pub bytes: Vec<u8>,
    pub mime: String,
    pub engine: String,
    pub voice: String,
}

/// Synthesize speech via the Edge neural TTS websocket; falls back to the
/// Windows SAPI voice if the service is unreachable.
pub async fn speak(text: &str, speech: &SpeechSettings) -> Result<SpokenAudio> {
    let text = text.trim();
    if text.is_empty() {
        return Err(AppError::Precondition("nothing to speak".into()));
    }
    let clipped = clip(text, speech.max_spoken_chars as usize);

    match edge_speak(&clipped, speech).await {
        Ok(audio) => Ok(audio),
        Err(edge_err) => {
            log::warn!("edge tts failed ({edge_err}); trying SAPI fallback");
            match sapi_fallback(&clipped, speech) {
                Ok(audio) => Ok(audio),
                Err(sapi_err) => Err(AppError::msg(format!(
                    "text-to-speech failed. Edge: {edge_err}. Fallback: {sapi_err}"
                ))),
            }
        }
    }
}

fn clip(text: &str, max: usize) -> String {
    let max = max.max(80);
    if text.chars().count() <= max {
        return text.to_string();
    }
    let truncated: String = text.chars().take(max).collect();
    format!("{truncated}…")
}

async fn edge_speak(text: &str, speech: &SpeechSettings) -> Result<SpokenAudio> {
    use futures_util::{SinkExt, StreamExt};
    use tokio_tungstenite::tungstenite::client::IntoClientRequest;
    use tokio_tungstenite::tungstenite::Message as Ws;

    let connect_id = uuid::Uuid::new_v4().simple().to_string();
    let mut connected = None;
    let mut last_err = None;
    for attempt in 0..2u8 {
        // The challenge hashes the (corrected) clock, so rebuild it per try.
        let url = format!(
            "wss://speech.platform.bing.com/consumer/speech/synthesize/readaloud/edge/v1\
             ?TrustedClientToken={TRUSTED_CLIENT_TOKEN}&ConnectionId={connect_id}\
             &Sec-MS-GEC={}&Sec-MS-GEC-Version={SEC_MS_GEC_VERSION}",
            generate_sec_ms_gec()
        );
        let mut request = url
            .as_str()
            .into_client_request()
            .map_err(|e| AppError::msg(format!("edge tts request: {e}")))?;
        {
            let headers = request.headers_mut();
            headers.insert("Origin", EDGE_ORIGIN.parse().unwrap());
            headers.insert("User-Agent", EDGE_USER_AGENT.parse().unwrap());
            headers.insert("Pragma", "no-cache".parse().unwrap());
            headers.insert("Cache-Control", "no-cache".parse().unwrap());
            headers.insert(
                "Cookie",
                format!("muid={};", generate_muid()).parse().unwrap(),
            );
        }
        match tokio_tungstenite::connect_async(request).await {
            Ok((stream, _)) => {
                connected = Some(stream);
                break;
            }
            Err(e) => {
                let corrected = correct_clock_skew(&e);
                last_err = Some(AppError::msg(format!("connect edge tts: {e}")));
                if attempt == 0 {
                    log::debug!(
                        "edge tts connect failed (clock skew corrected: {corrected}); retrying"
                    );
                }
            }
        }
    }
    let mut ws = connected
        .ok_or_else(|| last_err.unwrap_or_else(|| AppError::msg("edge tts connect failed")))?;

    let timestamp = edge_date();
    let config = format!(
        "X-Timestamp:{timestamp}\r\nContent-Type:application/json; charset=utf-8\r\nPath:speech.config\r\n\r\n\
{{\"context\":{{\"synthesis\":{{\"audio\":{{\"metadataoptions\":{{\"sentenceBoundaryEnabled\":\"false\",\"wordBoundaryEnabled\":\"false\"}},\"outputFormat\":\"audio-24khz-48kbitrate-mono-mp3\"}}}}}}}}\r\n"
    );
    ws.send(Ws::Text(config.into()))
        .await
        .map_err(|e| AppError::msg(format!("edge tts config: {e}")))?;

    let voice = if speech.voice.trim().is_empty() {
        "en-US-AriaNeural"
    } else {
        speech.voice.trim()
    };
    let rate_pct = ((speech.rate - 1.0) * 100.0).round() as i64;
    let volume_pct = ((speech.volume - 1.0) * 100.0).round() as i64;
    let rate_attr = format!("{rate_pct:+}");
    let volume_attr = format!("{volume_pct:+}");
    let escaped = escape_xml(text);
    let request_id = uuid::Uuid::new_v4().simple().to_string();
    let ssml = format!(
        "X-RequestId:{request_id}\r\nContent-Type:application/ssml+xml\r\nX-Timestamp:{timestamp}Z\r\nPath:ssml\r\n\r\n\
<speak version='1.0' xmlns='http://www.w3.org/2001/10/synthesis' xml:lang='en-US'>\
<voice name='{voice}'><prosody pitch='+0Hz' rate='{rate_attr}%' volume='{volume_attr}%'>{escaped}</prosody></voice></speak>"
    );
    ws.send(Ws::Text(ssml.into()))
        .await
        .map_err(|e| AppError::msg(format!("edge tts ssml: {e}")))?;

    let mut audio: Vec<u8> = Vec::new();
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(45);
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            return Err(AppError::Timeout("edge tts timed out".into()));
        }
        let msg = tokio::time::timeout(remaining, ws.next()).await;
        match msg {
            Ok(Some(Ok(Ws::Binary(data)))) => {
                // 2-byte big-endian header length, then the audio payload.
                if data.len() >= 2 {
                    let header_len = u16::from_be_bytes([data[0], data[1]]) as usize;
                    let start = 2 + header_len;
                    if data.len() > start {
                        audio.extend_from_slice(&data[start..]);
                    }
                }
            }
            Ok(Some(Ok(Ws::Text(text)))) => {
                if text.contains("Path:turn.end") {
                    break;
                }
            }
            Ok(Some(Ok(Ws::Close(_)))) => break,
            Ok(Some(Ok(_))) => {}
            Ok(Some(Err(e))) => return Err(AppError::msg(format!("edge tts stream: {e}"))),
            Ok(None) => break,
            Err(_) => return Err(AppError::Timeout("edge tts timed out".into())),
        }
    }

    if audio.is_empty() {
        return Err(AppError::msg("edge tts returned no audio"));
    }
    Ok(SpokenAudio {
        bytes: audio,
        mime: "audio/mpeg".into(),
        engine: "edge".into(),
        voice: voice.to_string(),
    })
}

fn sapi_fallback(text: &str, speech: &SpeechSettings) -> Result<SpokenAudio> {
    let rate = ((speech.rate - 1.0) * 10.0).round() as i64;
    let safe = text.replace('\'', "''").replace('\n', " ");
    let script = format!(
        "$sp = New-Object -ComObject SAPI.SpVoice; \
         $sp.Rate = {rate}; \
         $stream = New-Object -ComObject SAPI.SpFileStream; \
         $file = Join-Path $env:TEMP ('astra-tts-' + [guid]::NewGuid() + '.wav'); \
         $stream.Open($file, 3, $false); \
         $sp.AudioOutputStream = $stream; \
         $sp.Speak('{safe}') | Out-Null; \
         $stream.Close(); \
         $file"
    );
    let out = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .output()
        .map_err(|e| AppError::msg(format!("sapi: {e}")))?;
    let file = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if file.is_empty() || !std::path::Path::new(&file).exists() {
        let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return Err(AppError::msg(format!("windows speech unavailable: {err}")));
    }
    let bytes = std::fs::read(&file).map_err(|e| AppError::msg(format!("read wav: {e}")))?;
    let _ = std::fs::remove_file(&file);
    Ok(SpokenAudio {
        bytes,
        mime: "audio/wav".into(),
        engine: "windows_sapi".into(),
        voice: "system default".into(),
    })
}

fn escape_xml(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('\'', "&apos;")
        .replace('"', "&quot;")
}

// ---------------- speech-to-text (Groq Whisper) ----------------

pub async fn transcribe(audio: Vec<u8>, filename: &str, mime: &str) -> Result<String> {
    let key = crate::credentials::get(&crate::ai::api_key_account("groq"))
        .ok()
        .flatten()
        .ok_or_else(|| {
            AppError::Precondition(
                "speech-to-text needs a Groq API key (Settings → AI). Voice input is unavailable offline."
                    .into(),
            )
        })?;

    let part = reqwest::multipart::Part::bytes(audio)
        .file_name(filename.to_string())
        .mime_str(mime)
        .map_err(|e| AppError::msg(format!("audio part: {e}")))?;
    let form = reqwest::multipart::Form::new()
        .part("file", part)
        .text("model", "whisper-large-v3-turbo")
        .text("response_format", "json");

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(90))
        .build()
        .map_err(|e| AppError::msg(format!("client: {e}")))?;
    let resp = client
        .post("https://api.groq.com/openai/v1/audio/transcriptions")
        .bearer_auth(key)
        .multipart(form)
        .send()
        .await
        .map_err(|e| AppError::Provider(format!("transcription request failed: {e}")))?;

    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(AppError::Provider(format!("transcription failed ({status}): {body}")));
    }
    let parsed: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| AppError::Provider(format!("bad transcription response: {e}")))?;
    Ok(parsed
        .get("text")
        .and_then(|t| t.as_str())
        .unwrap_or_default()
        .trim()
        .to_string())
}

pub fn audio_b64(audio: &SpokenAudio) -> String {
    base64::engine::general_purpose::STANDARD.encode(&audio.bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gec_v2_matches_reference_vectors() {
        // Generated from edge_tts.drm.generate_sec_ms_gec_v2 at fixed timestamps.
        assert_eq!(
            gec_token(1_760_000_000.0, 0),
            "6D50DDDB91812C33-DD68EFB01898B0750C9797D51A5FF93471DFDAE6888A1039D532CBDCCE54545D"
        );
        assert_eq!(
            gec_token(1_790_000_000.0, 0),
            "D4216A7D3DE0C94F-91D741832FD084A110FDAD52BE2FFA569C6EA5F7D06A9429380A398738F4559A"
        );
        assert_eq!(
            gec_token(1_820_000_000.25, 0),
            "5518A766E1D17717-2AFDC18074E7A97AA650F5E465021E76AD556FA7D4325B34D646322D873B1502"
        );
        assert_eq!(
            gec_token(1_760_000_149.75, 0),
            "BAF4076F572BFE4E-2D6FBE5C0E24F56622EB94F37395F2D153C6C0C451DAAE2249A459BB3AFAD62C"
        );
    }

    #[tokio::test]
    #[ignore = "hits the live Edge TTS service"]
    async fn edge_live_synthesis() {
        let speech = crate::settings::SpeechSettings::default();
        let audio = edge_speak("Astra online. Systems ready.", &speech)
            .await
            .expect("edge tts synthesis");
        assert!(audio.bytes.len() > 1_000, "expected real mp3 audio");
        assert_eq!(audio.mime, "audio/mpeg");
    }
}
