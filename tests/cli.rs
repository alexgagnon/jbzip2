#[cfg(test)]
mod tests {
    use rstest::rstest;
    use std::fs::File;
    use std::io::{BufRead, BufReader, Error};
    use std::path::Path;

    #[test]
    fn test_process() -> Result<(), Error> {
        let path = Path::new("./tests/test-data.json.bz2").to_path_buf();
        let (mut reader, size) = jbzip2::get_file_as_bufreader(&path)?;
        let mut output = Vec::new();
        jbzip2::process(
            &mut reader,
            size,
            &mut output,
            &".id".to_string(),
            500000,
            Some("[\n".to_string()),
            Some("\n]".to_string()),
            ",\n".to_string(),
            true,
        )?;
        println!("{:?}", String::from_utf8_lossy(&output));
        assert_eq!(
            output,
            b"\"Q1\"\n\"Q2\"\n\"Q3\"\n\"Q4\"\n\"Q5\"\n\"Q6\"\n\"P1\"\n\"Q60\"\n"
        );
        Ok(())
    }

    #[rstest]
    fn test_wikidata_dump_files(
      #[values("george", "simple", "test-data", "1", "10", "100")]
      filename: &str
    ) -> Result<(), Error> {
        let input_path = Path::new(&format!("./tests/{}.json.bz2", filename)).to_path_buf();
        let (mut reader, size) = jbzip2::get_file_as_bufreader(&input_path)?;
        let expect_path = Path::new(&format!("./tests/{}.expected.txt", &filename)).to_path_buf();
        let mut expected = BufReader::new(File::open(expect_path).expect("Could not open file"));
        let mut output = Vec::new();
        jbzip2::process(
            &mut reader,
            size,
            &mut output,
            &".id".to_string(),
            50000000,
            Some("[\n".to_string()),
            Some("\n]".to_string()),
            ",\n".to_string(),
            true,
        )?;
        assert!(compare(
            &mut BufReader::new(output.as_slice()),
            &mut expected
        ));
        Ok(())
    }

    fn compare(a: &mut impl BufRead, b: &mut impl BufRead) -> bool {
        let mut buf1 = [0; 10000];
        let mut buf2 = [0; 10000];
        loop {
            if let Result::Ok(n1) = a.read(&mut buf1) {
                if n1 > 0 {
                    if let Result::Ok(n2) = b.read(&mut buf2) {
                        if n1 == n2 {
                            if buf1 == buf2 {
                                continue;
                            }
                        }
                        return false;
                    }
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        true
    }
}
