use structopt::StructOpt;

#[derive(Debug, StructOpt)]
pub struct New {}

// 1. Check and create the directory
// 2. Create `registry.toml`.
// 3. Clone the index project
// 4. Follow the index to download crates
// 5. Use the database to record downloads
