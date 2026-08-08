use std::sync::Arc;

use futures::future::join_all;
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, Semaphore};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct Video {
    title: String,
    video_id: String,
    published_at: String,
    thumbnails: Thumbnails,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct Thumbnails {
    url: String,
    width: usize,
    height: usize,
}

#[tokio::main]
async fn main() {
    let test_channel_id = "UCaZkRdEEpePJ4EEZznuqh8g";
    let manifest_content: Vec<Video> =
        serde_json::from_str(&read_file(&format!("subs/{test_channel_id}/manifest.json"))).unwrap();

    let path = format!("subtitle:subs/{test_channel_id}");

    let semaphore = Arc::new(Semaphore::new(10));
    let mut handles = vec![];
    let failed = Arc::new(Mutex::new(vec![]));

    for video in manifest_content {
        let permit = semaphore.clone().acquire_owned().await.unwrap();
        let path_clone = path.clone();
        let f = failed.clone();

        let handle = tokio::spawn(async move {
            println!("running: {}", video.video_id);
            match std::process::Command::new("yt-dlp")
                .args([
                    "-P",
                    &path_clone.to_string(),
                    "--cookies",
                    "cookies.env",
                    "--skip-download",
                    "--write-auto-subs",
                    "--sub-langs",
                    "en",
                    "--sub-format",
                    "srt",
                    "-o",
                    "subtitle:%(id)s.%(ext)s",
                    &video.video_id,
                ])
                .status()
            {
                Ok(res) => {
                    println!("complete {} with status: {}", video.video_id, res);
                    drop(permit);
                }
                Err(e) => {
                    println!("failed {} with status: {}", video.video_id, e);
                    let mut failed_vec_guard = f.lock().await;
                    failed_vec_guard.push(video.video_id);
                    drop(permit);
                }
            }
        });

        handles.push(handle);
    }

    join_all(handles).await;

    println!("ok: failed to complete:");
    println!("{:?}", *failed.lock().await);
}

fn read_file(manifest_path: &str) -> String {
    std::fs::read_to_string(manifest_path).expect("failed to read file")
}
