//! NXGX and NXCX (`*.NXGZ`, `*.NXZ`): a 16-byte header (magic, u32
//! unpacked size, u32 packed size, u32 0) and the data, gzip (NXGX) or
//! zlib (NXCX). The pictures inside are BNTX textures.

use std::io::Read;

use flate2::read::{GzDecoder, ZlibDecoder};

use super::FormatError;
use crate::read::Reader;

pub fn decompress(data: &[u8]) -> Result<Vec<u8>, FormatError> {
    let r = Reader(data);
    let size = r.u32(4)? as usize;
    let packed = r.slice(16, r.u32(8)? as usize)?;
    let mut out = Vec::new(); // not the header's size: a corrupt one would reserve gigabytes
    let read = match &data[..4] {
        b"NXGX" => GzDecoder::new(packed).read_to_end(&mut out),
        b"NXCX" => ZlibDecoder::new(packed).read_to_end(&mut out),
        _ => return Err(FormatError::BadMagic("NXGX/NXCX")),
    };
    read.map_err(|e| FormatError::Bad(format!("{}: {e}", String::from_utf8_lossy(&data[..4]))))?;
    if out.len() != size {
        return Err(FormatError::Bad(format!("unpacked to {} bytes, header says {size}", out.len())));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use flate2::Compression;
    use flate2::write::{GzEncoder, ZlibEncoder};

    use super::*;

    fn wrap(magic: &[u8], packed: Vec<u8>, size: u32) -> Vec<u8> {
        let mut nxx = magic.to_vec();
        nxx.extend(size.to_le_bytes());
        nxx.extend((packed.len() as u32).to_le_bytes());
        nxx.extend([0; 4]);
        nxx.extend(packed);
        nxx.resize(nxx.len() + 100, 0); // sector padding
        nxx
    }

    #[test]
    fn both_kinds() {
        let mut gz = GzEncoder::new(Vec::new(), Compression::default());
        gz.write_all(b"BNTX....").unwrap();
        assert_eq!(decompress(&wrap(b"NXGX", gz.finish().unwrap(), 8)).unwrap(), b"BNTX....");
        let mut z = ZlibEncoder::new(Vec::new(), Compression::default());
        z.write_all(b"BNTX").unwrap();
        let packed = z.finish().unwrap();
        assert_eq!(decompress(&wrap(b"NXCX", packed.clone(), 4)).unwrap(), b"BNTX");
        assert!(decompress(&wrap(b"NXCX", packed, 5)).is_err());
    }
}
