use bzip2::read::MultiBzDecoder;
use indicatif::{HumanDuration, ProgressBar, ProgressDrawTarget, ProgressStyle};
use jq_rs::JqProgram;
use log::{debug, info, trace};
use serde_json::Value;
use simdutf8::basic::from_utf8;
use core::panic;
use std::env;
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
    raw: bool
) -> Result<(), std::io::Error> {
    let no_progress_bar = env::var("NO_PROGRESS_BAR").is_ok();
    let mut stream = BufWriter::new(output);

    let mut md = MultiBzDecoder::new(reader);

    trace!("Initializing buffer to size {}", buffer_size);
    let mut buffer = vec![0; buffer_size];

    let bar = ProgressBar::new(size);
    bar.set_draw_rate(1);
    bar.set_style(
        ProgressStyle::default_bar().template("{spinner:.green} [{elapsed_precise}] {msg}"),
    );

    // hide progress bar if runnings tests or explicitely set with env
    if cfg!(test) || no_progress_bar {
        bar.set_draw_target(ProgressDrawTarget::hidden());
    }

    let start = Instant::now();

    // discard the prefix characters
    if prefix.is_some() {
        debug!("Stripping prefix");
        md.read(&mut vec![0u8; prefix.unwrap().len()])
            .expect("Could not strip prefix");
    }

    debug!("Filtering entities...");
    let mut num_entities = 0;
    let mut num_entities_filtered = 0;
    let mut num_errors = 0;

    let suffix = suffix.unwrap_or("".to_string());
    let mut n = md.read(&mut buffer)?;
    debug!("Read {} bytes", n);

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
        
        // trim delimiter from start and end of the buffer to normalize
        str_buffer = str_buffer.trim_start_matches(&delimiter).to_string();
        str_buffer = str_buffer.trim_end_matches(&delimiter).to_string();

        // find the last delimiter in the string, if it exists
        let pos = str_buffer.rfind(&delimiter);

        // pos can be None if it's a single entity, or if the entity is larger than the buffer
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

        // convert the delimiter to newline for jq --raw if not already (i.e. jsonl)
        if !delimiter.eq("\n") {
          str_buffer = str_buffer.replace(&delimiter, "\n");
        }

        let mut jq = Command::new("jq");
        jq.args(["-r", jq_filter]);

        let process = match jq.stdin(Stdio::piped()).stdout(Stdio::piped()).spawn() {
            Err(why) => panic!("couldn't spawn wc: {}", why),
            Ok(process) => process,
        };

        match process.stdin.unwrap().write_all(str_buffer.as_bytes()) {
          Err(why) => panic!("couldn't write to wc stdin: {}", why),
          Ok(_) => println!("sent to wc"),
        }

        let reader: BufReader<std::process::ChildStdout> = BufReader::new(process.stdout.unwrap());
        // reader
        // .lines()
        // .filter_map(|line| line.ok())
        // .for_each(|line| println!("{}", line));

        // stream.write_all(&buffer[..n]).expect("Could not write to stream");
        reader.lines().filter_map(|line| line.ok()).for_each(|line| {
          stream.write_all(line.as_bytes()).expect("Could not write to stream");
          stream.write_all(b"\n").expect("Could not write to stream");
        });

        // println!("jq output: {}", String::from_utf8_lossy(&jq.stdout));
        // println!("jq error: {}", String::from_utf8_lossy(&jq.stderr));


        // for entity in entities {
        //     let filtered_entity = filter_entity(entity, &mut filter, continue_on_error);
            
          // output_entity(
          //     &mut stream,
          //     filtered_entity,
          //     &mut num_entities,
          //     &mut num_entities_filtered,
          //     &mut num_errors,
          //     raw,
          // );
      // }

        // // the last item could be:
        // // 1. incomplete, so just iterate
        // // 2. shorter than the filled buffer, meaning we're EOF
        // // 3. splitting the suffix (should iterate fine)
        // // 4. exactly before the suffix (should iterate fine)
        // let last = last.trim();
        // if last.ends_with(&suffix) {
        //     debug!("Last entity");
        //     let filtered_entity = filter_entity(
        //         &last[..last.len() - suffix.len()],
        //         &mut filter,
        //         continue_on_error,
        //     );
        //     output_entity(
        //         &mut stream,
        //         filtered_entity,
        //         &mut num_entities,
        //         &mut num_entities_filtered,
        //         &mut num_errors,
        //         raw,
        //     );
        //     break;
        // }

        if !no_progress_bar {
            bar.set_message(format!(
                "Processed {} entities, {} filtered out and {} errors",
                num_entities, num_entities_filtered, num_errors
            ));
        }

        str_buffer = last.to_string();

        buffer = vec![0; buffer_size];
        n = md.read(&mut buffer)?;
        debug!("Read {} bytes", n);
    }

    stream.flush().expect("Could not flush");

    bar.finish_with_message(format!(
        "Finished in {}. Processed {} entities, {} filtered out and {} errors",
        HumanDuration(start.elapsed()),
        num_entities,
        num_entities_filtered,
        num_errors
    ));
    Ok(())
}

pub fn get_file_as_bufreader(path: &PathBuf) -> Result<(BufReader<File>, u64), std::io::Error> {
    let file = File::open(path)?;
    let size = file.metadata()?.len();
    debug!("Opening {:?}, size: {}", path, size);
    Ok((BufReader::new(file), size))
}

// TODO: replace the Option return with Result so we can output the error for easier debugging
fn filter_entity(entity: &str, filter: &mut JqProgram, continue_on_error: bool) -> Option<String> {
    trace!(">> filter_entity");
    trace!("{}", entity);
    let result = filter.run(&entity);
    let filtered_entity = match result {
        Ok(e) => e,
        Err(error) => {
            if !continue_on_error {
                println!("Could not parse: {}", error);
                panic!("Could not parse: {}", error);
            } else {
                info!("Could not parse: {}", error);
                return None;
            }
        }
    };
    trace!("{}", filtered_entity);
    trace!("<< filter_entity");
    Some(filtered_entity)
}

fn output_entity(
    stream: &mut BufWriter<&mut impl Write>,
    filtered_entity: Option<String>,
    num_entities: &mut i32,
    num_entities_filtered: &mut i32,
    num_errors: &mut i32,
    raw: bool,
) {
    *num_entities += 1;
    if filtered_entity.is_some() {
        let filtered_entity = filtered_entity.unwrap();
        if !filtered_entity.is_empty() {
            if !raw {
              // TODO: handle various types recursively...
              let parsed: Value = serde_json::from_str(&filtered_entity).unwrap();
              stream.write(parsed.as_str().unwrap().as_bytes()).expect("Could not write");
            }
            else {
              stream
              .write(filtered_entity.as_bytes())
              .expect("Could not write");
            }

        }
        else {
          *num_entities_filtered += 1;
        }
    }
    // jq-error
    else {
        *num_errors += 1;
        // TODO: output to log file?
    }
}
