use std::fs::File;
use std::io::{BufReader, BufRead, Read, Write, BufWriter};
use std::path::PathBuf;
use std::time::Instant;
use bzip2::read::MultiBzDecoder;
use indicatif::{HumanDuration, ProgressBar, ProgressStyle, HumanBytes};
use jq_rs::JqProgram;
use log::{debug, info, trace};
use simdutf8::basic::from_utf8;

pub fn process(reader: &mut impl BufRead, size: u64, output: &mut impl Write, jq_filter: &String, buffer_size: usize, prefix: Option<String>, suffix: Option<String>, delimiter: String, continue_on_error: bool) -> Result<(), std::io::Error> {
  let mut stream = BufWriter::new(output);
  let mut filter = jq_rs::compile(jq_filter).expect("Could not compile jq filter");
  
  let mut total_bytes: u64 = 0;

  let mut md = MultiBzDecoder::new(reader);
  
  trace!("Initializing buffer to size {}", buffer_size);
  let mut buffer = vec![0; buffer_size];

  let bar = ProgressBar::new(size);

  bar.set_draw_rate(1);
  bar.set_style(ProgressStyle::default_bar()
  .template("{msg}\n{spinner:.green} [{elapsed_precise}] ({bytes_per_sec})")
  .progress_chars("#>-"));

  let start = Instant::now();

  // discard the prefix characters
  if !prefix.is_none() {
    debug!("Stripping prefix");
    md.read(&mut vec![0u8; prefix.unwrap().len()]).expect("Could not strip prefix");
  }

  debug!("Filtering entities...");
  let d = delimiter.as_bytes();
  let s = suffix.unwrap();
  debug!("{:?}", from_utf8(d));
  let mut i = 0;
  let mut num_entities = 0;
  let mut num_entities_output = 0;
  for byte in md.bytes() {
    let byte = byte.unwrap();
    buffer[i] = byte;
    i += 1;
    if i > d.len() {

      // end of entity
      if &buffer[i - d.len()..i] == d {
        num_entities += 1;
        let entity = from_utf8(&buffer[..i - d.len()]).expect("Could not convert to string");
        // debug!("Start: {:?}, End: {:?}", &buffer[..10], &buffer[i - 10..i]);
        let filtered_entity = filter_entity(entity, &mut filter, continue_on_error);
        // debug!("{}", filtered_entity);
        if !filtered_entity.eq("") {
            stream.write(filtered_entity.as_bytes()).expect("Could not write");
            num_entities_output += 1;
        }
        buffer = vec![0; buffer_size];
        i = 0;
      }

      // end of file
      else if &buffer[i - s.len()..i] == s.as_bytes() {
        num_entities += 1;
        let entity = from_utf8(&buffer[..i - s.len()]).expect("Could not convert to string");
        let filtered_entity = filter_entity(entity, &mut filter, continue_on_error);
        // debug!("{}", filtered_entity);
        if !filtered_entity.eq("") {
            stream.write(filtered_entity.as_bytes()).expect("Could not write");
            num_entities_output += 1;
        }
        break;
      }
    }
  }

  // let mut num_entities = 0;
  // let mut num_entities_output = 0;
  // let mut n = md.read(&mut buffer)?;
  // debug!("Read {} bytes", n);
  // // n = md.read(&mut buffer)?;
  // // println!("Read {} bytes", n);
  // // debug!("{}", from_utf8(&buffer[100000..]).expect("could not decode"));

  // let start = Instant::now();

  // let mut str_buffer = String::new();

  // let suffix = suffix.unwrap();
  // let suffix = suffix.as_str();

  // while n > 0 {
  //     total_bytes += n as u64;
  //     bar.inc(n as u64);

  //     // convert to utf8 string
  //     let decompressed = from_utf8(&buffer[..n]).expect("Could not convert to string");

  //     str_buffer.push_str(decompressed);

  //     let mut entities: Vec<_> = str_buffer.split(&delimiter).collect();
  //     let length = entities.len();
  //     debug!("{} items", length);

  //     // iterate over the entities that are "whole"
  //     // &mut so we can mutably borrow each item in the vector
  //     for entity in &mut entities[..(length - 1)] {
  //         let filtered_entity = filter_entity(entity, &mut filter, continue_on_error);
  //         // debug!("{}", filtered_entity);
  //         num_entities += 1;
  //         if !filtered_entity.eq("") {
  //             stream.write(filtered_entity.as_bytes()).expect("Could not write");
  //             num_entities_output += 1;
  //         }
  //         bar.set_message(format!("Processed {} entities, {} outputted", num_entities, num_entities_output));
  //     }

  //     // mutable ref to entities done here
  //     let last = entities.last_mut().expect("Could not get last item").trim_end();

  //     // debug!("Last: {}", last)

  //     // reset the string buffer with the incomplete last entity
  //     str_buffer = last.to_string();

  //     // clear the buffer
  //     buffer = vec![0u8; buffer_size];

  //     // read in the next bytes
  //     n = md.read(&mut buffer)?;
  //     debug!("Read {} bytes", n);
  // }

  // // push whatever is left in the buffer into the string
  // str_buffer.push_str(from_utf8(&buffer[..]).expect("Could not convert to string"));

  // debug!("Start: {}, End: {}", str_buffer[..10].to_string(), str_buffer[str_buffer.len() - 100..].to_string());

  // // handle the last bit of text
  // if !str_buffer.is_empty() {
  //   debug!("Last entity");

  //   if !suffix.is_empty() {
  //     debug!("Stripping suffix");

  //     // easiest way is to just pop the number of suffix chars from the end, which respects utf8
  //     // TODO: probably fine to just drop the last bytes instead
  //     for _ in 0..suffix.chars().count() {
  //       str_buffer.pop();
  //     }
  //   }

  //   debug!("Start: {}, End: {}", str_buffer[..10].to_string(), str_buffer[str_buffer.len() - 10..].to_string());
    
  //   let filtered_entity = filter_entity(&str_buffer, &mut filter, continue_on_error);
  //   num_entities += 1;
  //   if !filtered_entity.eq("") {
  //       stream.write_all(filtered_entity.as_bytes()).expect("Could not write");
  //       num_entities_output += 1;
  //   }
  // }
  stream.flush().expect("Could not flush");
  
  bar.set_message(format!("Processed {} entities, {} outputted", num_entities, num_entities_output));  
  bar.finish_with_message(format!("Finished! Processed {} entities and outputted {} in {}", HumanBytes(total_bytes), num_entities, HumanDuration(start.elapsed())));
  Ok(())
}

pub fn get_file_as_bufreader(path: &PathBuf) -> Result<(BufReader<File>, u64), std::io::Error> {
  let file = File::open(path)?;
  let size = file.metadata()?.len();
  debug!("Opening {:?}, size: {}", path, size);
  Ok((BufReader::new(file), size))
}

fn filter_entity(entity: &str, filter: &mut JqProgram, continue_on_error: bool) -> String {
  debug!(">> filter_entity");
  trace!("{}", entity);
  let result = filter.run(&entity);
  let filtered_entity = match result {
      Ok(e) => e,
      Err(error) => if !continue_on_error {
          println!("Could not parse: {}", error);
          panic!("Could not parse: {}", error);
      } else {
          info!("Could not parse: {}", error);
          String::from("null")
      }
  };
  trace!("{}", filtered_entity);
  debug!("<< filter_entity");
  filtered_entity
}