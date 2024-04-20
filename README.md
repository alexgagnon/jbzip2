# jbzip2

CLI enabling decompressing and transforming bzip2 compressed files in one shot. This helps with very large files that would take up too much disk space by decompressing it first and then transforming, and works faster than just piping the output between common commands. It uses the `jq` tool under the hood to filter entities and extract desired properties.

**NOTE: your prefix/suffix/delimiter needs to allow for splitting the raw text into individual items.**

Currently it works best for files that are jsonl or a single array (such as Wikidata dumps).

Notes:
- although `jq` is primarily used to process JSON input, it can output in any format, such as csv or even arbitrary text
- you can test jq filters here: https://jqplay.org/

## Usage

### jsonl

The default prefix/suffix/delimiters are set up to handle jsonl.

`jbzip2 --input example.jsonl.bz2 --jq-filter '.id'`

### Wikidata Dumps

Wikidata dumps are a single large array of items split by a newline, so we need to supply custom prefix/suffix/delimiter values

`jbzip2 --input latest-all.json.bz2 --jq-filter 'select(.type == "item") | .id | @tsv' --type wikidump`

### Other

You can provide your own prefix, suffix, and delimiters as required but **you may need to use your shell specific way of escaping characters, for example with sh/bash, you need to use e.g. --prefix $'[\n'.**

## Debugging

For additional debugging information, run the command with env_logger variable (i.e. `RUST_LOG={info,debug,trace}`)

## TODO

- [ ] look at parallelizing the child processes