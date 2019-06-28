use structopt::StructOpt;

#[derive(Debug, StructOpt)]
pub struct Update {}

// 0. Check and create the mirror directory
// 1. Clone the index project to the index directory under the image directory
// 2. Follow the index to download crates
// 3. Use the database to record downloads
