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
 use clap::{Parser, arg, command};
 use std::io::Write;
 use log::{debug, info};

#[derive(Parser, Debug)]
#[command(author="alexgagnon", version, about="Process JSON with jq directly from a bzip2 file")]

struct Cli {
    #[arg(short = 'c', long = "continue-on-error", help = "Don't bail on error while filtering")]
    continue_on_error: bool,

    #[arg(short = 'i', long = "input", required = false, help = "Source file")]
    input_file_path: Option<PathBuf>,

    #[arg(short = 'o', long = "output", help = "Filename to output filtered entities (default is stdout)")]
    output_file_path: Option<PathBuf>,

    #[arg(short = 'f', long = "force", help = "Force overwriting files")]
    force_overwrite: bool,   

    #[arg(short = 'j', long = "jq-filter", default_value = "", help = "jq filter, see https://jqlang.github.io/jq/ for usage. NOTE: items are provided to jq as an array.")]
    jq_filter: String,

    #[arg(short = 't', long = "type", required = false, help = "Type of file to process which provides defaults to -p, -s, and -d. Options are 'jsonl' or 'wikidump'")]
    format_type: Option<String>,

    #[arg(short = 'p', long = "prefix", required = false, help = "Characters in the beginning of the file to skip")]
    prefix: Option<String>,

    #[arg(short = 's', long = "suffix", required = false, help = "Characters at the end of the file to skip")]
    suffix: Option<String>,

    #[arg(short = 'd', long = "delimiter", help = "Delimiter between elements.")]
    delimiter: Option<String>,

    #[arg(short = 'b', long = "buffer-size", default_value = "100000000", help = "the size of the buffer in bytes. NOTE: the buffer must be as large as the largest entity you're processing. Default is 1Gb.")]
    buffer_size: usize,

    // TODO: convert this capture any arguments for jq
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


