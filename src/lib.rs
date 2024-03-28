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
    prefix: Option<String>,
    suffix: Option<String>,
    delimiter: String,
    continue_on_error: bool,
) -> Result<(), std::io::Error> {
    let mut stream = BufWriter::new(output);

    let mut md = MultiBzDecoder::new(reader);

    trace!("Initializing buffer to size {}", buffer_size);
    let mut buffer = vec![0; buffer_size];

    let start = Instant::now();

    // discard the prefix characters
    if prefix.is_some() {
        debug!("Stripping prefix");
        md.read(&mut vec![0u8; prefix.unwrap().len()])
            .expect("Could not strip prefix");
    }

    debug!("Filtering entities...");

    let suffix = suffix.unwrap_or("".to_string());
    let mut n = md.read(&mut buffer)?;
    let mut total_bytes: u64 = n as u64;

    let mut str_buffer = String::new();
    str_buffer.reserve(buffer_size); // reserve space for the buffer

    let mut done = false;

    // if n == 0, we're at EOF
    while n > 0 && !done {
        // buffer has bytes in it, convert up to the number of bytes read to string
        str_buffer.push_str(from_utf8(&buffer[..n]).expect("Could not convert to utf8 string"));

        // []
        // [partial] -> error, buffer_size too small
        // [,partial] -> error, buffer_size too small
        // [a...] -> fine -> single entity smaller than buffer
        // [a,] -> fine -> exactly ends with delimiter, trim it
        // [a,partial] -> fine, keep partial for next iteration
        // [a, b...] -> fine, concat a and b
        
        // trim delimiter from start and end of the buffer to normalize what's in the buffer
        str_buffer = str_buffer.trim_start_matches(&delimiter).to_string();
        str_buffer = str_buffer.trim_end_matches(&delimiter).to_string();

        // find the last delimiter in the string, if it exists
        let pos = str_buffer.rfind(&delimiter);

        // pos can be None if it's a single entity, or if the entity is larger than the buffer
        // TODO: this is an edge case... it could EXACTLY fit the buffer
        // maybe keep a flag and then continue and see if the first char of the next buffer is a delimiter
        if pos.is_none() {
          if str_buffer.len() >= buffer_size {
            panic!("Entity is larger than buffer, increase --buffer-size value")
          }
        }

        let mut last = str_buffer.split_off(pos.unwrap_or(0));

        // if it's the last entity, trim the suffix and then put it back in the str_buffer
        if last.ends_with(&suffix) {
            done = true;
            last.truncate(last.len() - suffix.len());
            str_buffer.push_str(&last);
        }

        // convert the delimiter to newline for jq --raw if not already (i.e. jsonl format)
        if !delimiter.eq("\n") {
          str_buffer = str_buffer.replace(&delimiter, "\n");
        }

        let mut jq = Command::new("jq");
        jq.args(["-r", jq_filter]);

        let process = match jq.stdin(Stdio::piped()).stdout(Stdio::piped()).spawn() {
            Err(_) => panic!("Couldn't spawn jq, is it install?"),
            Ok(process) => process,
        };

        match process.stdin.unwrap().write_all(str_buffer.as_bytes()) {
          Err(why) => panic!("Couldn't write to stdin: {}", why),
          Ok(_) => {},
        }

        let reader: BufReader<std::process::ChildStdout> = BufReader::new(process.stdout.unwrap());

        reader.lines().for_each(|line| {
          match line {
            Ok(line) => {
              stream.write_all(line.as_bytes()).expect("Could not write to stream");
              stream.write_all(b"\n").expect("Could not write to stream");
            },
            Err(_) => {
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
        debug!("Read {} bytes", n);

        total_bytes += n as u64;

        replace_line(format!("Processed {}", format_bytes(total_bytes as u64)).as_str());
        print!("Processed {}", format_bytes(total_bytes as u64));
    }

    stream.flush().expect("Could not flush");

    let duration = start.elapsed();
    replace_line("");
    info!(
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