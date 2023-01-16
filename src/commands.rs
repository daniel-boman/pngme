use std::{
    io::{Read, Seek, Write},
    path::PathBuf,
    str::FromStr,
};

use crate::{chunk::Chunk, chunk_type::ChunkType, png::Png, Result};
use anyhow::{anyhow, bail};
use clap::{arg, command, Parser, Subcommand};

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
    Remove {
        #[arg(long)]
        file_path: std::path::PathBuf,
        #[arg(long)]
        chunk_type: String,
    },
    List {
        #[arg(long)]
        file_path: std::path::PathBuf,
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
        } => encode_chunk(file_path, message, chunk_type, output_file),
        Commands::Decode {
            file_path,
            chunk_type,
        } => decode_chunk(file_path, chunk_type),
        Commands::Remove {
            file_path,
            chunk_type,
        } => remove_chunk(file_path, chunk_type),
        Commands::List { file_path } => list_chunks(file_path),
    }?;

    println!("{}", output);

    Ok(())
}

fn remove_chunk(file_path: PathBuf, chunk_type: String) -> Result<String> {
    if !file_path.exists() {
        return Err(anyhow!("file at the provided path does not exist"));
    }

    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .read(true)
        .open(file_path)?;

    let mut png = Png::try_from(&mut file)?;

    let chunk = png.remove_chunk(chunk_type.as_str())?;

    file.seek(std::io::SeekFrom::Start(0))?;
    file.write_all(&png.as_bytes())?;
    file.sync_all()?;
    drop(file);

    Ok(format!(
        "removed chunk: [{}]: [{}]",
        chunk.chunk_type().to_string(),
        chunk.data_as_string()?
    ))
}

fn list_chunks(file_path: PathBuf) -> Result<String> {
    if !file_path.exists() {
        return Err(anyhow!("file at the provided path does not exist"));
    }
    let mut file = std::fs::OpenOptions::new().read(true).open(file_path)?;

    let png = Png::try_from(&mut file)?;

    png.chunks()
        .iter()
        .for_each(|chunk| println!("{}", chunk.chunk_type().to_string()));

    Ok("".to_string())
}

fn decode_chunk(file_path: PathBuf, chunk_type: String) -> Result<String> {
    let mut file = std::fs::OpenOptions::new().read(true).open(file_path)?;

    let png = Png::try_from(&mut file)?;

    match png.chunk_by_type(&chunk_type) {
        Some(chunk) => Ok(format!(
            "{}: {}",
            chunk.chunk_type().to_string(),
            chunk.data_as_string()?
        )),
        None => Err(anyhow!("could not find chunk by type {}", chunk_type)),
    }
}

fn encode_chunk(
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

    let buf: &[u8] = buf.as_ref();

    let png = match Png::try_from(buf) {
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
