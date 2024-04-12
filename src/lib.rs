use bzip2::read::MultiBzDecoder;
use log::{debug, info, trace};
use simdutf8::basic::from_utf8;
use core::panic;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Read, Write};
use std::path::PathBuf;

use std::process::{Command, Stdio};
use std::time::Instant;

pub fn process(
    reader: &mut impl BufRead,
    size: u64,
    output: &mut impl Write,
    jq_filter: &String,
    buffer_size: usize,
    format_type: Option<String>,
    prefix: Option<String>,
    suffix: Option<String>,
    delimiter: Option<String>,
    continue_on_error: bool,
) -> Result<(), std::io::Error> {
    let mut stream = BufWriter::new(output);

    let mut md = MultiBzDecoder::new(reader);

    trace!("Initializing buffer to size {}", buffer_size);
    let mut buffer = vec![0; buffer_size];
    debug!("Buffer initialized: {:?}", buffer.len());

    let mut p = "".to_string();
    let mut s = "".to_string();
    let mut d = "".to_string();
    let t = format_type.unwrap_or("jsonl".to_string());
    let convert_to_array = t.eq("wikidump");

    match t.as_str() {
        "wikidump" => {
            p = "[\n".to_string();
            s = "\n]".to_string();
            d = ",\n".to_string();
        }
        "jsonl" => {
            p = "".to_string();
            s = "".to_string();
            d = "\n".to_string();
        }
        _ => {
            panic!("Invalid format type");
        }
    }

    if prefix.is_some() {
        p = prefix.unwrap();
    }

    if suffix.is_some() {
        s = suffix.unwrap();
    }

    if delimiter.is_some() {
        d = delimiter.unwrap();
    }

    debug!("Type: {:?}", t);
    debug!("Prefix: {:?}", p);
    debug!("Suffix: {:?}", s);
    debug!("Delimiter: {:?}", d);

    let start = Instant::now();

    // discard the prefix characters
    debug!("Stripping prefix");
    md.read(&mut vec![0u8; p.len()])
        .expect("Could not strip prefix");

    debug!("Filtering entities...");

    let mut n = md.read(&mut buffer)?;
    let mut total_bytes: u64 = n as u64;

    // buffer to hold the string, with a little extra space in case we need to
    // convert to an array
    let mut str_buffer = String::with_capacity(buffer_size * 2);

    let mut done = false;

    let mut jq = Command::new("jq");
    jq.args(["-r", jq_filter]);

    // if n == 0, we're at EOF
    while n > 0 && !done {
        // buffer has bytes in it, convert up to the number of bytes read to string
        str_buffer.push_str(from_utf8(&buffer[..n]).expect("Could not convert to utf8 string"));
        // possible scenarios:
        // []
        // [partial] -> error, buffer_size too small
        // [,partial] -> error, buffer_size too small
        // [a...] -> fine -> single entity smaller than buffer
        // [a,] -> fine -> exactly ends with delimiter, trim it
        // [,a] -> fine -> exactly starts with delimiter, trim it
        // [a,partial] -> fine, keep partial for next iteration

        // trim delimiter from start and end of the buffer to normalize what's in the buffer
        // NOTE: use slices to avoid string mutations
        let mut slice = str_buffer.trim_start_matches(&d);
        slice = slice.trim_end_matches(&d);

        // find the last delimiter in the string, if it exists
        let pos = slice.rfind(&d);

        debug!("Last delimiter at: {:?}", pos);

        // pos can be None if it's a single entity, or if the entity is larger than the buffer
        // TODO: this is an edge case... it could EXACTLY fit the buffer
        if pos.is_none() {
          info!("1 entity in buffer, or entity is larger than buffer");
          if slice.len() >= buffer_size {
            panic!("Entity is larger than buffer, increase --buffer-size value")
          }
        }

        let mut last = &slice[pos.unwrap_or(0)..];

        // if it's the last entity, trim the suffix and then put it back in the str_buffer
        if last.ends_with(&s) {
            done = true;
            debug!("Trimming suffix");
            last = last.trim_end_matches(&s);
            slice = slice.trim_end_matches(&s);
        }
        else {
            slice = &slice[0..pos.unwrap_or(slice.len())]
        }

        info!("Processing: {:?} - {:?}", &slice[0..10], &slice[slice.len()-10..]);

        let mut process = match jq.stdin(Stdio::piped()).stdout(Stdio::piped()).spawn() {
            Err(_) => panic!("Couldn't spawn jq, is it install?"),
            Ok(process) => process,
        };

        if let Some(mut stdin) = process.stdin.take() {
          if convert_to_array {
            stdin.write_all("[".as_bytes()).expect("Failed to write to stdin");
          }
          stdin.write_all(slice.as_bytes()).expect("Failed to write to stdin");
          if convert_to_array {
            stdin.write_all("]".as_bytes()).expect("Failed to write to stdin");
          }
        } else {
            return Err(std::io::Error::new(std::io::ErrorKind::Other, "Child process stdin not captured"));
        }

        debug!("Waiting for jq to finish...");

        let reader: BufReader<std::process::ChildStdout> = BufReader::new(process.stdout.unwrap());

        reader.lines().for_each(|line| {
          match line {
            Ok(line) => {
              stream.write_all(line.as_bytes()).expect("Could not write to stream");
              stream.write_all(b"\n").expect("Could not write to stream");
            },
            Err(_) => {
              // TODO: write to stderr and/or error log
              if continue_on_error {
                return;
              }
              panic!("Error processing jq filter");
            }
          }
        });

        str_buffer = last.to_string();

        buffer = vec![0; buffer_size];
        n = md.read(&mut buffer)?;

        total_bytes += n as u64;

        replace_line(format!("Processed {}", format_bytes(total_bytes as u64)).as_str());
        print!("Processed {}", format_bytes(total_bytes as u64));
    }

    stream.flush().expect("Could not flush");

    let duration = start.elapsed();
    replace_line("");
    print!(
        "Processed {} bytes in {} seconds",
        format_bytes(size),
        format!("{:.2}", duration.as_secs_f64())
    );

    Ok(())
}

fn replace_line(str: &str) {
    print!("\x1B[2K\r");
    std::io::stdout().flush().unwrap();
    print!("{}", str);
}

fn format_bytes(bytes: u64) -> String {
    let kb = 1024;
    let mb = kb * 1024;
    let gb = mb * 1024;
    let tb = gb * 1024;

    if bytes < kb {
        format!("{} B", bytes)
    } else if bytes < mb {
        format!("{:.2} KB", bytes as f64 / kb as f64)
    } else if bytes < gb {
        format!("{:.2} MB", bytes as f64 / mb as f64)
    } else if bytes < tb {
        format!("{:.2} GB", bytes as f64 / gb as f64)
    } else {
        format!("{:.2} TB", bytes as f64 / tb as f64)
    }
}

pub fn get_file_as_bufreader(path: &PathBuf) -> Result<(BufReader<File>, u64), std::io::Error> {
    let file = File::open(path)?;
    let size = file.metadata()?.len();
    debug!("Opening {:?}, size: {}", path, size);
    Ok((BufReader::new(file), size))
}