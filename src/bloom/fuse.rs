use std::io::Write;

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use xorf::BinaryFuse8;

use crate::coding::{Decode, Encode};

impl Encode for BinaryFuse8 {
    fn encode_into<W: Write>(&self, writer: &mut W) -> Result<(), crate::EncodeError> {
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
        let hash_type = reader.read_u8()?;
        assert_eq!(0, hash_type, "Invalid fuse hash type");

        let length = reader.read_u64::<BigEndian>()? as usize;

        let mut bytes = vec![0; length];
        reader.read_exact(&mut bytes)?;

        let decoded: Self = bincode::deserialize(&bytes).expect("Unable to decode BinaryFuse8");
        Ok(decoded)
    }
}

#[cfg(test)]
mod tests {
    use crate::bloom::Filter;

    use super::*;
    use std::fs::File;
    use test_log::test;
    use xxhash_rust::xxh3::xxh3_64;

    #[test]
    fn fuse_serde_round_trip() -> crate::Result<()> {
        let dir = tempfile::tempdir()?;

        let path = dir.path().join("fuse");
        let mut file = File::create(&path)?;

        let keys: Vec<u64> = (0..1000).map(|x| x as u64).collect();
        let hashes: Vec<u64> = (0..1000).map(|x: u64| xxh3_64(&x.to_be_bytes())).collect();
        let binary_fuse = BinaryFuse8::try_from(&hashes).expect("Failed to create filter");
        let filter = Filter::BinaryFuse(binary_fuse);

        for key in keys {
            assert!(filter.contains(&key.to_be_bytes()));
        }

        assert!(!filter.contains(b"asdasads"));
        assert!(!filter.contains(b"item10"));
        assert!(!filter.contains(b"cxycxycxy"));

        filter.encode_into(&mut file)?;
        file.sync_all()?;
        drop(file);

        let mut file = File::open(&path)?;
        let filter_copy = Filter::decode_from(&mut file)?;

        let keys: Vec<u64> = (0..1000).map(|x| x as u64).collect();
        for key in keys {
            assert!(filter_copy.contains(&key.to_be_bytes()));
        }
        assert!(!filter_copy.contains(b"asdasads"));
        assert!(!filter_copy.contains(b"item10"));
        assert!(!filter_copy.contains(b"cxycxycxy"));

        Ok(())
    }
}
