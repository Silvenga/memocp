use blake3::{Hash as Blake3Hash, HexError};
use borsh::{BorshDeserialize, BorshSerialize};
use std::io;
use std::io::{Read, Write};
use std::str::FromStr;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Hash {
    pub hash: Blake3Hash,
}

impl Hash {
    pub fn new(hash: Blake3Hash) -> Self {
        Self { hash }
    }

    pub fn as_string(&self) -> String {
        self.hash.to_hex().to_string()
    }

    pub fn empty_hash() -> Self {
        Self::new(Blake3Hash::from([0; 32]))
    }
}

impl Default for Hash {
    fn default() -> Self {
        Self::empty_hash()
    }
}

impl FromStr for Hash {
    type Err = HexError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let hash = Blake3Hash::from_str(s)?;
        Ok(Self::new(hash))
    }
}

impl From<Blake3Hash> for Hash {
    fn from(value: Blake3Hash) -> Self {
        Self::new(value)
    }
}

impl BorshSerialize for Hash {
    fn serialize<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        writer.write_all(self.hash.as_bytes())
    }
}

impl BorshDeserialize for Hash {
    fn deserialize_reader<R: Read>(reader: &mut R) -> io::Result<Self> {
        let mut buffer = [0; 32];
        reader.read_exact(&mut buffer)?;
        Blake3Hash::from_slice(&buffer)
            .map(Self::new)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Failed to deserialize hash"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn round_trip() {
        let hash =
            Hash::from_str("d74981efa70a0c880b8d8c1985d075dbcbf679b99a5f9914e5aaf96b831a9e24")
                .unwrap();

        let data = borsh::to_vec(&hash).unwrap();
        let result = borsh::from_slice(&data[..]).unwrap();

        assert_eq!(hash, result);
    }
}
