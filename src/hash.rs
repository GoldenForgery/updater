use sha2::{Digest, Sha256};

use crate::Error;
use std::{path::PathBuf, str::FromStr};

#[derive(std::hash::Hash, Debug, PartialEq, Eq, Clone)]
pub struct Hash(String);

impl FromStr for Hash {
    type Err = crate::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() != 64 {
            return Err(Error::InvalidHash);
        }

        Ok(Self(s.to_string()))
    }
}

impl TryFrom<&PathBuf> for Hash {
    type Error = crate::Error;

    fn try_from(path: &PathBuf) -> Result<Self, Self::Error> {
        let data = std::fs::read(path).map_err(|_| crate::Error::FileReadError)?;
        let hash = Sha256::digest(data);
        let hexstr = base16ct::lower::encode_string(&hash);

        Self::from_str(&hexstr)
    }
}
