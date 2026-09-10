mod ingest;

fn main() {
    ingest::parse_all("subs").unwrap();
}
