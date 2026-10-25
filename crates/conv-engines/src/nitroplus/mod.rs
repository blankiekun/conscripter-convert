//! Nitroplus's engine (装甲悪鬼村正, 2009): NSS source scripts in NPA
//! archives; the English release's NPK3 archives and Squirrel scripts
//! for its text.

pub mod assets;
pub mod build;
pub mod convert;
pub mod english;
pub mod npa;
pub mod npk;
pub mod png_gamma;
pub mod nss;
pub mod refs;
pub mod route;
pub mod squirrel;

/// FNV-1a 64 as a `Hasher`, for generated file names: unlike std's
/// `DefaultHasher`, it gives the same names in every Rust version and build.
#[derive(Clone, Copy)]
pub struct NameHasher(u64);

impl Default for NameHasher {
    fn default() -> Self {
        NameHasher(0xcbf2_9ce4_8422_2325)
    }
}

impl std::hash::Hasher for NameHasher {
    fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.0 = (self.0 ^ u64::from(b)).wrapping_mul(0x100_0000_01b3);
        }
    }

    fn finish(&self) -> u64 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use std::hash::Hasher;

    #[test]
    fn name_hasher_is_fnv1a() {
        let mut h = super::NameHasher::default();
        h.write(b"a");
        assert_eq!(h.finish(), 0xaf63_dc4c_8601_ec8c);
    }
}
