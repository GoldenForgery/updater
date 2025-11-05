use std::path::PathBuf;

use hash::Hash;
use iced::widget::{center, center_x, column, image, progress_bar, text};
use iced::{Element, Task};
use manifest::Manifest;
use octocrab::models::repos::Release;
use reqwest;
use tempdir::TempDir;
use thiserror::Error;

mod hash;
mod manifest;

pub fn main() -> iced::Result {
    iced::application(boot, Progress::update, Progress::view)
        .window_size(iced::Size::new(300., 350.))
        .resizable(false)
        .run()
}

fn check_update_task() -> Task<Message> {
    Task::perform(check_update(), |f| match f {
        Ok((release, manifest)) => Message::BeginCompare(release, manifest),
        Err(err) => Message::Err(err.to_string()),
    })
}

fn boot() -> (Progress, Task<Message>) {
    let task = check_update_task();

    (Progress::default(), task)
}

async fn check_update() -> Result<(Release, Manifest), Error> {
    Manifest::from_repository("GoldenForgery", "files").await
}

async fn compare_files(manifest: Manifest) -> Result<Vec<PathBuf>, Error> {
    let mut invalid_files = Vec::<PathBuf>::new();
    for (manifest_hash, path) in manifest.files {
        // Missing file
        if !path.exists() {
            invalid_files.push(path);
            continue;
        }

        if let Ok(existing_file_hash) = Hash::try_from(&path) {
            // Hashes match, so we skip it
            if existing_file_hash == manifest_hash {
                continue;
            }
        } else {
            // Couldn't calculate hash of existing file for whatever reason, so we download it
            invalid_files.push(path);
        }
    }

    Ok(invalid_files)
}

async fn update(release: Release) -> Result<(), Error> {
    // Download release
    let release_zip_url = release
        .assets
        .iter()
        .find(|asset| {
            asset.content_type == "application/zip" && asset.name.starts_with("golden-forgery")
        })
        .map(|asset| asset.browser_download_url.clone())
        .ok_or(Error::ReleaseZipNotFound)?;

    let tmp_dir = TempDir::new("golden-forgery")
        .map_err(|_| Error::TmpDirCreateFail)?
        .into_path();
    let zip_bytes = reqwest::get(release_zip_url).await?.bytes().await?;

    let zip_path = tmp_dir.join("release.zip");
    std::fs::write(&zip_path, zip_bytes).map_err(|_| Error::GenericError)?;

    let file_reader = std::fs::File::open(&zip_path).map_err(|_| Error::GenericError)?;
    let mut zip = zip::ZipArchive::new(file_reader).map_err(|_| Error::GenericError)?;

    println!("Extracting...");
    zip.extract(".").map_err(|_| Error::GenericError)?;

    Ok(())
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("Could not find manifest file in latest release")]
    ManifestNotFound,

    #[error("Request error")]
    RequestError(#[from] octocrab::Error),

    #[error("Request error")]
    ReqwestError(#[from] reqwest::Error),

    #[error("Invalid hash in manifest file")]
    InvalidHash,

    #[error("Failed to read file")]
    FileReadError,

    #[error("Could not find the zip file in the latest release. Please contact the developers.")]
    ReleaseZipNotFound,

    #[error("Could not create temporary directory")]
    TmpDirCreateFail,

    #[error("GenericError")]
    GenericError,
}

#[derive(Default)]
struct Progress {
    value: f32,
    status: Status,
}

#[derive(Debug)]
enum Message {
    BeginCompare(Release, Manifest),
    BeginUpdate(Release, Vec<PathBuf>),
    Finish,
    Err(String),
}

#[derive(Default, Debug)]
enum Status {
    #[default]
    Checking,
    Comparing(Manifest),
    Updating(Vec<PathBuf>),
    Finished,
    Error(String),
}

impl ToString for Status {
    fn to_string(&self) -> String {
        match self {
            Status::Checking => "Checking for updates. Please wait.".into(),
            Status::Updating(invalid_files) => format!(
                "{} files failed to validate. Updating...",
                invalid_files.len()
            ),
            Status::Comparing(_manifest) => format!("Checking local files"),
            Status::Finished => "Updated! Launching Umineko: Golden Forgery".into(),
            Status::Error(err) => format!("Error: {err}"),
        }
    }
}

impl Progress {
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::BeginCompare(release, manifest) => {
                self.status = Status::Comparing(manifest.clone());

                Task::perform(compare_files(manifest), |f| match f {
                    Ok(invalid_files) => {
                        if invalid_files.len() > 0 {
                            Message::BeginUpdate(release, invalid_files)
                        } else {
                            Message::Finish
                        }
                    }
                    Err(err) => Message::Err(err.to_string()),
                })
            }
            Message::BeginUpdate(release, invalid_files) => {
                self.status = Status::Updating(invalid_files.clone());
                Task::perform(update(release), |f| match f {
                    Ok(_) => Message::Finish,
                    Err(err) => Message::Err(err.to_string()),
                })
                .chain(check_update_task())
            }
            Message::Finish => {
                self.status = Status::Finished;
                Task::none()
            }
            Message::Err(err) => {
                self.status = Status::Error(err);
                Task::none()
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let bar = progress_bar(0.0..=100.0, self.value);

        column![center(
            column![
                image("splash.png"),
                center_x(text(self.status.to_string())),
                bar,
            ]
            .spacing(20),
        ),]
        .spacing(20)
        .padding(20)
        .into()
    }
}
