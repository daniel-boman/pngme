use anyhow::{Error, Result};

mod chunk;
mod chunk_type;
mod commands;
mod png;
mod util;

fn main() -> Result<()> {
    commands::execute()
}
