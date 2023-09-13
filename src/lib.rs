use bzip2::read::MultiBzDecoder;
use indicatif::{HumanDuration, ProgressBar, ProgressDrawTarget, ProgressStyle};
use jq_rs::JqProgram;
use log::{debug, info, trace};
use serde_json::Value;
use simdutf8::basic::from_utf8;
use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Read, Write};
use std::path::PathBuf;
use std::str::from_utf8_unchecked;
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
    let mut filter = jq_rs::compile(jq_filter).expect("Could not compile jq filter");

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

    // BYTE ITERATOR METHOD, 2x slower than jstream
    if env::var("LIST_ITERATOR").is_ok() {
        debug!("List iterator");
        let mut i = 0;
        let delimiter = delimiter.as_bytes();
        let mut bytes = md.bytes();
        let mut d = 0;

        loop {
            match bytes.next() {
                Some(Ok(byte)) => {
                    buffer[i] = byte;
                    if delimiter[d] == byte {
                        if delimiter.len() - 1 == d {
                            // we have a whole entity
                            // strip away the delimiter parts and convert to utf8 string
                            let entity = from_utf8(&buffer[..i - d])
                                .expect("Could not convert to utf8 string");
                            let filtered_entity =
                                filter_entity(entity, &mut filter, continue_on_error);
                            num_entities += 1;
                            output_entity(
                                &mut stream,
                                filtered_entity,
                                &mut num_entities,
                                &mut num_entities_filtered,
                                &mut num_errors,
                                raw,
                            );

                            if !no_progress_bar {
                                bar.set_message(format!(
                                    "Output {} of {} entities processed",
                                    num_entities_filtered, num_entities
                                ));
                            }

                            // reset buffer and indexes
                            buffer = vec![0; buffer_size];
                            i = 0;
                            d = 0;
                            continue;
                        }
                        d += 1;
                    } else {
                        d = 0;
                    }
                    i += 1;
                }

                Some(Err(_)) => {
                    panic!("Invalid byte");
                }

                None => {
                    debug!("EOF reached");
                    let end = if suffix.is_some() {
                        suffix.unwrap().len()
                    } else {
                        0
                    };
                    let entity =
                        from_utf8(&buffer[..i - end]).expect("Could not convert to utf8 string");
                    let filtered_entity = filter_entity(entity, &mut filter, continue_on_error);
                    output_entity(
                        &mut stream,
                        filtered_entity,
                        &mut num_entities,
                        &mut num_entities_filtered,
                        &mut num_errors,
                        raw,
                    );
                    break;
                }
            }
        }
    }
    // BUFFER METHOD,
    else {
        debug!("Buffer");

        // TODO: find a way to detect last element without a suffix (maybe try JqFilter and see if it passes?)
        let suffix = suffix.unwrap_or("".to_string());
        let mut n = md.read(&mut buffer)?;
        debug!("Read {} bytes", n);

        let mut str_buffer = String::new();

        while n > 0 {
            // buffer has bytes in it, convert up to the number of bytes read to string
            str_buffer.push_str(from_utf8(&buffer[..n]).expect("Could not convert to utf8 string"));
            let entities: Vec<&str> = str_buffer.split(&delimiter).collect();
            let (last, entities) = entities.split_last().unwrap();
            for entity in entities {
                let filtered_entity = filter_entity(entity, &mut filter, continue_on_error);
                output_entity(
                    &mut stream,
                    filtered_entity,
                    &mut num_entities,
                    &mut num_entities_filtered,
                    &mut num_errors,
                    raw,
                );
            }

            // the last item could be:
            // 1. incomplete, so just iterate
            // 2. shorter than the filled buffer, meaning we're EOF
            // 3. splitting the suffix (should iterate fine)
            // 4. exactly before the suffix (should iterate fine)
            let last = last.trim();
            if last.ends_with(&suffix) {
                debug!("Last entity");
                let filtered_entity = filter_entity(
                    &last[..last.len() - suffix.len()],
                    &mut filter,
                    continue_on_error,
                );
                output_entity(
                    &mut stream,
                    filtered_entity,
                    &mut num_entities,
                    &mut num_entities_filtered,
                    &mut num_errors,
                    raw,
                );
                break;
            }

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
