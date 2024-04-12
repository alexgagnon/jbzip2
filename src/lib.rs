use bzip2::read::MultiBzDecoder;
use log::{debug, info};
use simdutf8::basic::from_utf8;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Read, Write};
use std::path::PathBuf;

use std::process::{Command, Stdio};
use std::thread;
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

    // override the defaults if they're provided
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

    let s_slice = s.as_str();

    let start = Instant::now();

    // discard the prefix characters into a null buffer
    debug!("Stripping prefix");
    md.read(&mut vec![0u8; p.len()])
        .expect("Could not strip prefix");

    debug!("Filtering entities...");

    let mut n = md.read(&mut buffer)?;
    let mut current_bytes = n as u64;
    let mut total_bytes: u64 = n as u64;
    let mut num_entities = 0;
    let mut i = 0;

    // buffer to hold the string, with a little extra space in case we need to
    // convert to an array
    let mut str_buffer = String::with_capacity(buffer_size + 2);

    let mut done = false;

    // if n == 0, we're at EOF
    // NOTE: `read` will pull in only up to a single block at at time (which for parallel compressed
    // data like wikidumps is around 900k), so we need to keep reading until we've filled the buffer
    // this is important because some entities may be larger than the block size
    while !done {

        str_buffer.push_str(from_utf8(&buffer[..n]).expect("Could not convert to utf8 string"));

        let mut is_last = str_buffer.ends_with(s_slice);
        while (!is_last || n > 0) && current_bytes < buffer_size as u64 - 1000000 {
            debug!("{} < {}, reading more bytes", current_bytes, buffer_size);
            n = md.read(&mut buffer).expect("Could not read from buffer");
            current_bytes += n as u64;
            let string = from_utf8(&buffer[..n]).expect("Could not convert to utf8 string");
            is_last = str_buffer.ends_with(s_slice);
            str_buffer.push_str(string);
        }

        // buffer has bytes in it, convert up to the number of bytes read to string
        debug!("{:?} - {:?}", &str_buffer[0..10], &str_buffer[str_buffer.len()-10..]);

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
          debug!("Slice length: {}", slice.len());
          if slice.len() <= buffer_size {
            info!("1 entity");
          }
          else {
            panic!("Entity is larger than buffer, increase --buffer-size value")
          }
        }

        let last = &slice[pos.unwrap_or(0) + d.len()..];

        // if it's the last entity, trim the suffix
        if last.ends_with(&s) {
            done = true;
            debug!("Trimming suffix");
            slice = slice.trim_end_matches(&s);
        }
        else {
            debug!("Saving last entity for next iteration");
            slice = &slice[..pos.unwrap_or(slice.len())]
        }

        debug!("Processing: {:?} - {:?}", &slice[0..100], &slice[slice.len()-5..]);
        debug!("Last: {:?} - {:?}", &last[0..100], &last[last.len()-5..]);

        let mut jq = Command::new("jq");
        jq.args(["-r", jq_filter]);

        let mut process = match jq.stdin(Stdio::piped()).stdout(Stdio::piped()).spawn() {
            Err(_) => panic!("Couldn't start jq, is it installed?"),
            Ok(process) => process,
        };

        let stdin = process.stdin.take().expect("Failed to take stdin");
        let stdout = process.stdout.take().expect("Failed to take stdout");
        let reader = BufReader::new(stdout);

        thread::scope(|s| {
          s.spawn(|| {
            let mut stdin= stdin;
            if convert_to_array {
              stdin.write_all("[".as_bytes()).expect("Failed to write to stdin");
            }
            stdin.flush().expect("Failed to flush stdin");
            stdin.write_all(slice.as_bytes()).expect("Failed to write to stdin");
            stdin.flush().expect("Failed to flush stdin");
            if convert_to_array {
              stdin.write_all("]".as_bytes()).expect("Failed to write to stdin");
            }
          });

          reader.lines().for_each(|line| {
            match line {
              Ok(line) => {
                stream.write_all(line.as_bytes()).expect("Could not write to stream");
                stream.write_all(b"\n").expect("Could not write to stream");
                num_entities += 1;
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
        });

        str_buffer = last.to_string();
        str_buffer.reserve(buffer_size - str_buffer.len());

        n = md.read(&mut buffer)?;

        total_bytes += current_bytes as u64;
        current_bytes = 0;
        
        if i == 0 {
          replace_line(&format!("Processed {} entities from {} bytes in {} seconds", num_entities, format_bytes(total_bytes), format!("{:.2}", start.elapsed().as_secs_f64())));
          print!("Processed {} entities from {} bytes in {} seconds", num_entities, format_bytes(total_bytes), format!("{:.2}", start.elapsed().as_secs_f64()));
        }
        i += 1;
    }

    stream.flush().expect("Could not flush");

    let duration = start.elapsed();
    replace_line("");
    println!(
        "Processed {} entities from {} bytes in {} seconds",
        num_entities,
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