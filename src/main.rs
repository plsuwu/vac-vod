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
    let channel_ids: [String; 2] = [
        "UCaZkRdEEpePJ4EEZznuqh8g".to_string(),
        "UCBustguC_fsnqDQZxOBgGEg".to_string(),
    ];

    let mut failed = vec![];

    for channel_id in channel_ids.iter() {
        println!(
            "\n\n--------------------\nstarting: '{}'\n--------------------\n",
            channel_id
        );

        let f = fetch_subs(channel_id).await;
        failed.extend(f);
    }

    println!("completed {}; failed {:?}", channel_ids.len(), failed);
}

async fn fetch_subs(channel_id: &str) -> Vec<(String, String)> {
    let failed = Arc::new(Mutex::new(vec![]));
    let semaphore = Arc::new(Semaphore::new(10));
    let mut handles = vec![];

    let manifest_content: Vec<Video> =
        serde_json::from_str(&read_file(&format!("subs/{channel_id}/manifest.json"))).unwrap();

    let path = format!("subtitle:subs/{channel_id}");

    for video in manifest_content {
        let permit = semaphore.clone().acquire_owned().await.unwrap();
        let path_clone = path.clone();
        let f = failed.clone();

        let handle = tokio::spawn(async move {
            println!("running: {}", video.video_id);
            match std::process::Command::new("yt-dlp")
                .args([
                    "-P",
                    &path_clone,
                    "--cookies",
                    "cookies.env",
                    "--skip-download",
                    "--write-auto-subs",
                    "--sub-langs",
                    "en",
                    "--sub-format",
                    "vtt",
                    "-o",
                    "subtitle:%(id)s.%(ext)s",
                    &format!("https://youtube.com/watch?v={}", video.video_id),
                ])
                .status()
            {
                // I haven't really implemented a way to determine if a video actually has an
                // available subtitle track, so some "silently" fail due to a lack of an
                // auto-generated EN subs track for now...
                Ok(res) => {
                    if !res.success() {
                        println!("failed {} with status: {}", video.video_id, res);
                        let mut failed_vec_guard = f.lock().await;
                        failed_vec_guard.push((video.video_id, res.to_string()));
                    } else {
                        println!("complete {} with status: {}", video.video_id, res);
                    }
                    drop(permit);
                }
                Err(e) => {
                    println!("failed {} with status: {}", video.video_id, e);
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

fn read_file(manifest_path: &str) -> String {
    std::fs::read_to_string(manifest_path).expect("failed to read file")
}
