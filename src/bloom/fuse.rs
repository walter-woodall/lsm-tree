use std::io::{Read, Write};

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use xorf::{BinaryFuse8, Filter as FuseFilter};
use xxhash_rust::xxh3::xxh3_64;

use crate::{
    coding::{Decode, Encode},
    file::MAGIC_BYTES,
    DecodeError,
};

use super::Filter;

impl Filter for BinaryFuse8 {
    fn contains_key(&self, key: &[u8]) -> bool {
        let hash: u64 = xxh3_64(key);
        self.contains(&hash)
    }
}

impl Encode for BinaryFuse8 {
    fn encode_into<W: Write>(&self, writer: &mut W) -> Result<(), crate::EncodeError> {
        // Write header
        writer.write_all(&MAGIC_BYTES)?;

        writer.write_u8(1)?;

        // NOTE: Hash type (unused)
        writer.write_u8(0)?;

        if let Ok(bytes) = bincode::serialize(&self) {
            writer.write_u64::<BigEndian>(bytes.len() as u64)?;
            writer.write_all(&bytes)?;
        }
        Ok(())
    }
}

impl Decode for BinaryFuse8 {
    fn decode_from<R: std::io::Read>(reader: &mut R) -> Result<Self, crate::DecodeError>
    where
        Self: Sized,
    {
        // Check header
        let mut magic = [0u8; MAGIC_BYTES.len()];
        reader.read_exact(&mut magic)?;

        if magic != MAGIC_BYTES {
            return Err(DecodeError::InvalidHeader("BloomFilter"));
        }

        let filter_type = reader.read_u8()?;
        assert_eq!(1, filter_type, "Invalid filter type");

        let hash_type = reader.read_u8()?;
        assert_eq!(0, hash_type, "Invalid bloom hash type");

        let length = reader.read_u64::<BigEndian>()? as usize;

        let mut bytes = vec![0; length];
        reader.read_exact(&mut bytes)?;

        let decoded: Self = bincode::deserialize(&bytes).expect("Unable to decode BinaryFuse8");
        Ok(decoded)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use test_log::test;

    #[test]
    fn fuse_serde_round_trip() -> crate::Result<()> {
        let dir = tempfile::tempdir()?;

        let path = dir.path().join("fuse");
        let mut file = File::create(&path)?;

        let keys: Vec<u64> = (0..1000).map(|x| x as u64).collect();
        let hashes: Vec<u64> = (0..1000).map(|x: u64| xxh3_64(&x.to_be_bytes())).collect();
        let filter = BinaryFuse8::try_from(&hashes).expect("Failed to create filter");

        for key in keys {
            assert!(filter.contains_key(&key.to_be_bytes()));
        }

        assert!(!filter.contains_key(b"asdasads"));
        assert!(!filter.contains_key(b"item10"));
        assert!(!filter.contains_key(b"cxycxycxy"));

        filter.encode_into(&mut file)?;
        file.sync_all()?;
        drop(file);

        let mut file = File::open(&path)?;
        let filter_copy = BinaryFuse8::decode_from(&mut file)?;

        let keys: Vec<u64> = (0..1000).map(|x| x as u64).collect();
        for key in keys {
            assert!(filter_copy.contains_key(&key.to_be_bytes()));
        }
        assert!(!filter_copy.contains_key(b"asdasads"));
        assert!(!filter_copy.contains_key(b"item10"));
        assert!(!filter_copy.contains_key(b"cxycxycxy"));

        Ok(())
    }
}
