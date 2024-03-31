/**
 * This is an ETL app that takes as input a bzip2 encoded JSON file,
 * streams it through a decoder, extracts the desirable fields, and outputs
 * the result
 *
 * THINGS TO NOTE: in Rust, strings are UTF8 encoded (meaning a given character
 * can be anywhere from 1 to 4 bytes).
 */

 use std::fs::File;
 use std::path::PathBuf;
 use clap::Parser;
 use std::io::Write;
 use log::{debug, info};

#[derive(Parser, Debug)]
#[clap(author="alexgagnon", version, about="Decompress and process bzip2 compressed files.", long_about = "Best suited for jsonl or a single JSON array of elements. Currently, rust-jq outputs strings ending with newline characters. If you want raw text you'll need to use the `-r` flag.")]
struct Cli {
    #[clap(short = 'c', long = "continue-on-error", help = "Don't bail on error while filtering")]
    continue_on_error: bool,

    #[clap(parse(from_os_str), short = 'i', long = "input", required = false, takes_value = true, required = false, help = "Source wikidata dump source")]
    input_file_path: Option<PathBuf>,

    #[clap(parse(from_os_str), short = 'o', long = "output", help = "Filename to output filtered entities (default is stdout)")]
    output_file_path: Option<PathBuf>,

    #[clap(short = 'f', long = "force", help = "Force overwriting files")]
    force_overwrite: bool,   

    #[clap(short = 'j', long = "jq-filter", default_value = "", help = "jq filter, see https://stedolan.github.io/jq/ for usage. NOTE: The filter is applied to EACH ELEMENT!")]
    jq_filter: String,

    #[clap(short = 't', long = "type", required = false, help = "Type of file to process. Options are 'jsonl' or 'wikidump'")]
    format_type: Option<String>,

    #[clap(short = 'p', long = "prefix", required = false, help = "Characters in the beginning of the file to skip")]
    prefix: Option<String>,

    #[clap(short = 's', long = "suffix", required = false, help = "Characters at the end of the file to skip")]
    suffix: Option<String>,

    #[clap(short = 'd', long = "delimiter", default_value = "\n", help = "Delimiter between elements. For jsonl, the default of '\\n' is fine, and for wikidumps use ',\\n'")]
    delimiter: String,

    #[clap(short = 'b', long = "buffer-size", default_value = "500000", help = "the size of the buffer in bytes. NOTE: the buffer must be as large as the largest entity you're processing, and larger buffers are faster")]
    buffer_size: usize,

    #[clap(short = 'r', long = "raw", help = "Output raw text (instead of quoted strings)")]
    raw: bool,
}

#[tokio::main]
async fn main() {
    env_logger::init();
    info!("Starting...");

    let args = Cli::parse();
    debug!("{:?}", args);
    cli(args).await.expect("Could not execute CLI");
}

async fn cli(args: Cli) -> Result<(), Box<dyn std::error::Error>> {
    if !args.jq_filter.is_empty() {
        let mut output: Box<dyn Write>;
        if args.output_file_path.is_none() {
            info!("Outputting to stdout");
            let stdout = std::io::stdout(); // get the global stdout entity
            output = Box::new(stdout.lock()) as Box<dyn Write>; // acquire a lock on it
        }
        else {
            if args.output_file_path.clone().unwrap().exists() && !args.force_overwrite {
                panic!("Output file already exists, must use `--force` flag to continue");
            }
            // TODO: handle gracefully
            let output_path = args.output_file_path.unwrap();
            info!("Outputting to {:?}", output_path);
            let output_file = File::create(output_path);
            output = Box::new(output_file?) as Box<dyn Write>;
        }

        let input = args.input_file_path.expect("Could not get path");
        let (mut reader, size) = jbzip2::get_file_as_bufreader(&input).expect("Could not get BufReader");
        jbzip2::process(&mut reader, size, &mut output, &args.jq_filter, args.buffer_size, args.format_type, args.prefix, args.suffix, args.delimiter, args.continue_on_error)?;
    }
    else {
        info!("No filter provided");
    }
    
    Ok(())
}


