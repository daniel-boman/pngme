use std::{
    io::{Read, Seek, Write},
    path::PathBuf,
    str::FromStr,
};

use crate::{chunk::Chunk, chunk_type::ChunkType, png::Png, Error, Result};
use anyhow::{anyhow, bail};
use clap::{arg, command, Args, Parser, Subcommand};

#[derive(Parser)]
#[command(name = "pngme")]
#[command(bin_name = "pngme")]
#[command(author, version)]
#[command(propagate_version = true)]
struct PngMe {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Encode {
        #[arg(long)]
        file_path: std::path::PathBuf,
        #[arg(long)]
        message: String,
        #[arg(long)]
        chunk_type: String,
        output_file: Option<std::path::PathBuf>,
    },
    Decode {
        #[arg(long)]
        file_path: std::path::PathBuf,
        #[arg(long)]
        chunk_type: String,
    },
}

pub fn execute() -> Result<()> {
    let command = PngMe::parse();

    let output = match command.command {
        Commands::Encode {
            file_path,
            message,
            chunk_type,
            output_file,
        } => encode(file_path, message, chunk_type, output_file),
        Commands::Decode {
            file_path,
            chunk_type,
        } => decode(file_path, chunk_type),
    }?;

    println!("{}", output);

    Ok(())
}

fn decode(file_path: PathBuf, chunk_type: String) -> Result<String> {
    let mut file = std::fs::OpenOptions::new().read(true).open(file_path)?;

    let mut buf = Vec::<u8>::new();
    file.read_to_end(&mut buf)?;

    let png = Png::try_from(buf.as_ref())?;

    match png.chunk_by_type(&chunk_type) {
        Some(chunk) => Ok(format!(
            "{}: {}",
            chunk.chunk_type().to_string(),
            chunk.data_as_string()?
        )),
        None => Err(anyhow!("could not find chunk by type {}", chunk_type)),
    }
}

fn encode(
    file_path: PathBuf,
    message: String,
    chunk_type: String,
    output_file: Option<PathBuf>,
) -> Result<String> {
    let mut output_path = file_path;
    if output_file.is_some() {
        output_path = output_file.unwrap()
    }

    let chunk_type = ChunkType::from_str(&chunk_type)?;

    let chunk = Chunk::new(chunk_type, message.into_bytes());

    let mut output = std::fs::OpenOptions::new()
        .write(true)
        .read(true)
        .create(true)
        .open(output_path.clone())?;

    let mut buf = Vec::<u8>::new();
    let expected = output.metadata()?.len() as usize;

    if expected != 0 {
        println!("expected = {}, reading from file", expected);
        let read = output.read_to_end(&mut buf)?;

        if read != expected {
            bail!("did not fully read output file! {} != {}", read, expected)
        }
    }

    let png = match Png::try_from(buf.as_ref()) {
        Ok(mut png) => {
            png.append_chunk(chunk);
            png
        }
        Err(_) => Png::from_chunks(vec![chunk]),
    };

    let mut data = png.as_bytes();

    output.seek(std::io::SeekFrom::Start(0))?;

    output.write_all(&mut data)?;

    output.set_len(data.len() as u64)?;

    output.sync_all()?;

    drop(output);

    Ok(format!(
        "wrote message to file {}",
        output_path.to_str().unwrap()
    ))
}
