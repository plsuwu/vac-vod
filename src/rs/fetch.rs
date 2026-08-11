use std::{fs, process::Stdio, sync::Arc};

use futures::future::join_all;
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, Semaphore};

use crate::rs::util::paths::ChannelDirectory;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Video {
    pub title: String,
    pub video_id: String,
    pub published_at: String,
    pub thumbnails: Thumbnails,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Thumbnails {
    pub url: String,
    pub width: usize,
    pub height: usize,
}

/// Main runner function for fetching video playlists from a channel's uploads.
///
/// Sub format defaults to VTT (better timecode specificity per word). Should probably just
/// glob for channel names based on the directories but for now we are doing things hackyjanky
/// style around here.
///
/// # Params
/// - `channel_id` is pretty self-explanatory
/// - `concurrency` is the number of semaphore permits - this (naively) rate limits by
///   restricting the number of concurrent `yt-dlp` instances to the given value.
///
/// # Return
///
/// The `Vec<(String, String)>` returned by this function represents the failed fetches with
/// `(video_id, cmd_result)`.
pub async fn fetch_uploads(
    channel_id: &str,
    base_outdir: &str,
    concurrency: usize,
) -> Vec<(String, String)> {
    let mut failed = vec![];
    let subtitle_path = ChannelDirectory::new(channel_id, base_outdir);
    let manifest = read_manifest(&subtitle_path);
    let manifest_len = manifest.len();

    // println!("[channel_id='{channel_id}'] fetching {manifest_len} sub tracks");
    tracing::info!(channel_id, manifest_len, "fetching subtitle tracks");

    let semaphore = Arc::new(Semaphore::new(concurrency));
    let fetch = exec_cmd(manifest, &subtitle_path, &semaphore).await;
    failed.extend(fetch);

    failed
}

async fn exec_cmd(
    manifest_content: Vec<Video>,
    path: &ChannelDirectory,
    semaphore: &Arc<Semaphore>,
) -> Vec<(String, String)> {
    let failed = Arc::new(Mutex::new(vec![]));
    let mut handles = vec![];

    let path = path.raw_output_dir();
    for video in manifest_content {
        let permit = Arc::clone(semaphore).acquire_owned().await.unwrap();
        let path_clone = path.clone();
        let f = failed.clone();

        let handle = tokio::spawn(async move {
            tracing::trace!(id = video.video_id, "fetching sub track");
            match std::process::Command::new("yt-dlp")
                .args([
                    "-P",
                    &path_clone,
                    "--cookies",
                    //
                    // TODO pass this filepath on the command line
                    //
                    "cookies.env",
                    "--skip-download",
                    "--write-auto-subs",
                    "--sub-langs",
                    "en",
                    "--sub-format",
                    "raw",
                    "-o",
                    "subtitle:%(id)s.%(ext)s",
                    // for some reason this refuses to recognise the video id solely by ID if
                    // the id starts with _ or - and i truly do not know why
                    &format!("https://youtube.com/watch?v={}", video.video_id),
                ])
                .stdout(Stdio::null())
                .stderr(Stdio::piped())
                .status()
            {
                // I haven't really implemented a way to determine if a video actually has an
                // available subtitle track, so some "silently" fail due to a lack of an
                // auto-generated EN subs track for now
                Ok(res) => {
                    if !res.success() {
                        tracing::warn!(id = video.video_id, status = %res, "track fetch failed");
                        let mut failed_vec_guard = f.lock().await;
                        failed_vec_guard.push((video.video_id, res.to_string()));
                    } else {
                        tracing::info!(id = video.video_id, status = %res, "track fetch success");
                    }

                    drop(permit);
                }
                Err(e) => {
                    tracing::error!(id = video.video_id, status = %e, "track fetch error");
                    let mut failed_vec_guard = f.lock().await;
                    failed_vec_guard.push((video.video_id, e.to_string()));
                    drop(permit);
                }
            }
        });

        handles.push(handle);
    }

    join_all(handles).await;
    failed.lock().await.clone()
}

fn read_manifest(s: &ChannelDirectory) -> Vec<Video> {
    let manifest_path = s.manifest_filepath();
    let file_content =
        fs::read_to_string(&manifest_path).expect("failed to read the manifest file");

    serde_json::from_str(&file_content).expect("failed to parse the manifest JSON")
}
