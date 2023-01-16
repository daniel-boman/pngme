#![allow(unused_variables)]
use anyhow::bail;
use crc::{Algorithm, Crc};
use std::fmt::Display;

use crate::chunk_type::ChunkType;
use crate::{Error, Result};

pub struct Chunk {
    chunk_type: ChunkType,
    data: Vec<u8>,
    crc: Crc<u32>,
}

impl Chunk {
    pub const DATA_LENGTH_BYTES: usize = 4;
    pub const CHUNK_TYPE_BYTES: usize = 4;
    pub const CRC_BYTES: usize = 4;
    pub const METADATA_BYTES: usize =
        Chunk::DATA_LENGTH_BYTES + Chunk::CHUNK_TYPE_BYTES + Chunk::CRC_BYTES;
    pub const CRC_ALGORITHM: &Algorithm<u32> = &crc::CRC_32_ISO_HDLC;

    pub fn new(chunk_type: ChunkType, data: Vec<u8>) -> Self {
        let crc = crc::Crc::<u32>::new(Chunk::CRC_ALGORITHM);
        Self {
            chunk_type,
            data,
            crc,
        }
    }

    pub fn length(&self) -> usize {
        self.data.len()
    }

    pub fn chunk_type(&self) -> &ChunkType {
        &self.chunk_type
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn data_as_string(&self) -> Result<String, Error> {
        match std::str::from_utf8(&self.data) {
            Ok(s) => Ok(s.to_string()),
            Err(e) => Err(anyhow::anyhow!("utf8error: {}", e)),
        }
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        let data_length = self.length() as u32;
        data_length
            .to_be_bytes()
            .iter()
            .chain(self.chunk_type.bytes().iter())
            .chain(self.data.iter())
            .chain(self.crc().to_be_bytes().iter())
            .copied()
            .collect()
    }

    fn crc(&self) -> u32 {
        let bytes: Vec<u8> = self
            .chunk_type
            .bytes()
            .iter()
            .chain(self.data.iter())
            .copied()
            .collect();

        self.crc.checksum(&bytes)
    }
}

impl TryFrom<&[u8]> for Chunk {
    type Error = Error;
    fn try_from(value: &[u8]) -> Result<Self> {
        if value.len() < Chunk::METADATA_BYTES {
            bail!("input length lower than metadata length")
        }

        let (data_length, value) = value.split_at(Chunk::DATA_LENGTH_BYTES);

        let data_length = u32::from_be_bytes(data_length.try_into()?) as usize;

        let (chunk_type_bytes, value) = value.split_at(Chunk::CHUNK_TYPE_BYTES);

        let chunk_type_bytes: [u8; 4] = chunk_type_bytes.try_into()?;

        let chunk_type = ChunkType::try_from(chunk_type_bytes)?;
        if !chunk_type.is_valid() {
            bail!("chunk type [{}] is invalid", chunk_type)
        }
        let (chunk_data, value) = value.split_at(data_length);

        let (crc_bytes, _) = value.split_at(Chunk::CRC_BYTES);

        let new = Self {
            chunk_type: chunk_type,
            data: chunk_data.into(),
            crc: Crc::<u32>::new(Chunk::CRC_ALGORITHM),
        };

        let actual_crc = new.crc();
        let expected_crc = u32::from_be_bytes(crc_bytes.try_into()?);

        if expected_crc != actual_crc {
            bail!("Invalid crc: [{}] != [{}]", expected_crc, actual_crc)
        }

        Ok(new)
    }
}

impl Display for Chunk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.data_as_string().unwrap())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunk_type::ChunkType;
    use std::str::FromStr;

    fn testing_chunk() -> Chunk {
        let data_length: u32 = 42;
        let chunk_type = "RuSt".as_bytes();
        let message_bytes = "This is where your secret message will be!".as_bytes();
        let crc: u32 = 2882656334;

        let chunk_data: Vec<u8> = data_length
            .to_be_bytes()
            .iter()
            .chain(chunk_type.iter())
            .chain(message_bytes.iter())
            .chain(crc.to_be_bytes().iter())
            .copied()
            .collect();

        Chunk::try_from(chunk_data.as_ref()).unwrap()
    }

    #[test]
    fn test_new_chunk() {
        let chunk_type = ChunkType::from_str("RuSt").unwrap();
        let data = "This is where your secret message will be!"
            .as_bytes()
            .to_vec();
        let chunk = Chunk::new(chunk_type, data);
        assert_eq!(chunk.length(), 42);
        assert_eq!(chunk.crc(), 2882656334);
    }

    #[test]
    fn test_chunk_length() {
        let chunk = testing_chunk();
        assert_eq!(chunk.length(), 42);
    }

    #[test]
    fn test_chunk_type() {
        let chunk = testing_chunk();
        assert_eq!(chunk.chunk_type().to_string(), String::from("RuSt"));
    }

    #[test]
    fn test_chunk_string() {
        let chunk = testing_chunk();
        let chunk_string = chunk.data_as_string().unwrap();
        let expected_chunk_string = String::from("This is where your secret message will be!");
        assert_eq!(chunk_string, expected_chunk_string);
    }

    #[test]
    fn test_chunk_crc() {
        let chunk = testing_chunk();
        assert_eq!(chunk.crc(), 2882656334);
    }

    #[test]
    fn test_valid_chunk_from_bytes() {
        let data_length: u32 = 42;
        let chunk_type = "RuSt".as_bytes();
        let message_bytes = "This is where your secret message will be!".as_bytes();
        let crc: u32 = 2882656334;

        let chunk_data: Vec<u8> = data_length
            .to_be_bytes()
            .iter()
            .chain(chunk_type.iter())
            .chain(message_bytes.iter())
            .chain(crc.to_be_bytes().iter())
            .copied()
            .collect();

        let chunk = Chunk::try_from(chunk_data.as_ref()).unwrap();

        let chunk_string = chunk.data_as_string().unwrap();
        let expected_chunk_string = String::from("This is where your secret message will be!");

        assert_eq!(chunk.length(), 42);
        assert_eq!(chunk.chunk_type().to_string(), String::from("RuSt"));
        assert_eq!(chunk_string, expected_chunk_string);
        assert_eq!(chunk.crc(), 2882656334);
    }

    #[test]
    fn test_invalid_chunk_from_bytes() {
        let data_length: u32 = 42;
        let chunk_type = "RuSt".as_bytes();
        let message_bytes = "This is where your secret message will be!".as_bytes();
        let crc: u32 = 2882656333;

        let chunk_data: Vec<u8> = data_length
            .to_be_bytes()
            .iter()
            .chain(chunk_type.iter())
            .chain(message_bytes.iter())
            .chain(crc.to_be_bytes().iter())
            .copied()
            .collect();

        let chunk = Chunk::try_from(chunk_data.as_ref());

        assert!(chunk.is_err());
    }

    #[test]
    pub fn test_chunk_trait_impls() {
        let data_length: u32 = 42;
        let chunk_type = "RuSt".as_bytes();
        let message_bytes = "This is where your secret message will be!".as_bytes();
        let crc: u32 = 2882656334;

        let chunk_data: Vec<u8> = data_length
            .to_be_bytes()
            .iter()
            .chain(chunk_type.iter())
            .chain(message_bytes.iter())
            .chain(crc.to_be_bytes().iter())
            .copied()
            .collect();

        let chunk: Chunk = TryFrom::try_from(chunk_data.as_ref()).unwrap();

        let _chunk_string = format!("{}", chunk);
    }
}
