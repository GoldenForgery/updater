use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;

use crate::Error;
use crate::hash::Hash;
use octocrab;
use octocrab::models::repos::Release;
use reqwest;

#[derive(Debug, Clone)]
pub struct Manifest {
    pub files: HashMap<Hash, PathBuf>,
}

impl Manifest {
    pub async fn from_repository(owner: &str, repo: &str) -> Result<(Release, Self), crate::Error> {
        let release = octocrab::instance()
            .repos(owner, repo)
            .releases()
            .get_latest()
            .await
            .map_err(Error::RequestError)?;

        let manifest_url = release
            .assets
            .iter()
            .find(|asset| asset.name.to_lowercase() == "manifest")
            .map(|asset| asset.browser_download_url.clone())
            .ok_or(Error::ManifestNotFound)?;

        let manifest_str = reqwest::get(manifest_url).await?.text().await?;
        Ok((release, Self::try_from(manifest_str)?))
    }
}

impl TryFrom<String> for Manifest {
    type Error = crate::Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let files = value
            .lines()
            .map(|line| {
                line.split_once("  ").map(|(hash_str, path_str)| {
                    (Hash::from_str(hash_str).unwrap(), PathBuf::from(path_str))
                })
            })
            .filter(Option::is_some)
            .map(Option::unwrap)
            .collect::<HashMap<Hash, PathBuf>>();

        Ok(Self { files })
    }
}
