use std::io::Write;
use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::{fs::File, io::BufWriter};

use anyhow::{Context, Result};
use memmap2::Mmap;
use polars::frame::DataFrame;
use polars::io::SerReader;

fn map_mcap(p: &PathBuf) -> Result<Mmap> {
    let fd = File::open(p).context("Couldn't open MCAP file")?;
    unsafe { Mmap::map(&fd) }.context("Couldn't map MCAP file")
}

pub fn hacky_as_hell_mcap_to_dataframe(p: &PathBuf) -> Result<DataFrame> {
    let mapped = map_mcap(p)?;

    let f = File::create("/tmp/foo").expect("Unable to create file");
    let mut f = BufWriter::new(f);

    let mut fixed_channel_id: Option<u16> = None;
    let mut warned = false;

    for message in mcap::MessageStream::new(&mapped)? {
        let message = message?;
        if fixed_channel_id.is_none() {
            fixed_channel_id = Some(message.channel.id);
        } else if let Some(fixed_channel_id) = fixed_channel_id {
            if fixed_channel_id != message.channel.id {
                if !warned {
                    println!("Warning: only one topic per mcap file is currently supported");
                    warned = true;
                }
                continue;
            }
        }
        f.write(&message.data)?;
        f.write(b"\n")?;
    }
    f.flush()?;
    let reader = polars::io::ndjson::core::JsonLineReader::from_path("/tmp/foo");
    let reader = reader?.infer_schema_len(Some(NonZeroUsize::new(1).unwrap()));

    Ok(reader.finish()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        // hacky_as_hell_mcap_to_dataframe().unwrap();
    }
}
