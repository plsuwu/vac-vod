mod ingest;
mod misc;
mod tokenizer;
mod safetensors;

use tokio::sync::mpsc::unbounded_channel;

use tokenizer::wordpiece::WordPiece;

#[tokio::main]
async fn main() {
    println!("building corpus chunks");
    let ingest_chunks = ingest::parse_all("subs").await.unwrap();
    let num_chunks = ingest_chunks.len();
    println!("corpus length: {} chunks", num_chunks);

    let (tx, rx) = unbounded_channel::<()>();
    tokio::task::spawn(async move { misc::progress(rx, "vocab", num_chunks).await });

    let wp = WordPiece::from_vocab_file(tokenizer::VOCAB_PATH, 256).unwrap();
    let chunk_vecs = ingest_chunks
        .iter()
        .map(|c| {
            let tok = wp.encode(&c.text);
            _ = tx.send(());
            tok
        })
        .collect::<Vec<_>>();

    println!("{:?}", chunk_vecs[0]);
    println!("{:?}", wp.debug_decode(&chunk_vecs[0]));
}
