use std::io::Write;

use tokio::sync::mpsc::UnboundedReceiver;

pub async fn progress(mut rx: UnboundedReceiver<()>, prefix: &str, len: usize) {
    let mut curr = 0;
    let mut stdout = std::io::stdout();

    let init = format!("\r{}: processing {} items\n", prefix, len);
    stdout.write_all(init.as_bytes()).unwrap();
    let init = format!("\r{}: 0%", prefix);
    stdout.write_all(init.as_bytes()).unwrap();
    stdout.flush().unwrap();

    loop {
        tokio::select! {
            Some(()) = rx.recv() => {
                curr += 1;
                let prog = ((curr as f64 / len as f64) * 100.0).floor() as usize;

                let p = format!("\r{}: {}%", prefix, prog);
                stdout.write_all(p.as_bytes()).unwrap();
                stdout.flush().unwrap();

                if curr == len {
                    println!("\r{}: {}%", prefix, prog);
                    break;
                }
            },
        }
    }
    println!("{}: all items processed\n", prefix);
}
