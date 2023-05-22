# jbzip2

CLI which enables decompressing and transforming bzip2 compressed JSON files in one shot. This helps with very large files that would either take up too much disk space, and works faster than just piping the output between commands. It uses the `jq` tool under the hood to filter entities and extract desired properties.

**NOTE: it processes each item one at a time, so your prefix/suffix/delimiter needs to allow for splitting the raw text into individual items.**

**NOTE: you may need to use your shell specific way of escaping characters, for example with sh/bash, you need to use e.g. --prefix $'[\n'.**

Currently it works best for files that are ndjson or a single array (such as Wikidata dumps).

Notes:
- although `jq` is primarily used to process JSON input, it can output in any format, such as csv or even arbitrary text
- you can test jq filters here: https://jqplay.org/

## TODO:

Convert to peekable stream, peek on first char of 'delimiter' and keep peeking until sure there's a match, then handle the entity. Once it's done iterating remove the bytes from the end that match 'suffix'.
## Usage

### ndjson

The default prefix/suffix/delimiters are set up to handle ndjson.

`jbzip2 --input example.ndjson.bz2 --jq-filter '.id'

### Wikidata Dumps

Wikidata dumps are a single large array of items split by a newline, so we need to supply custom prefix/suffix/delimiter values

`jbzip2 --input latest-all.json.bz2 --jq-filter 'select(.type == "item") | .id | @tsv' --prefix $'[\n' --suffix $'\n]' --delimiter $',\n'`


## Debugging

For additional debugging information, run the command with env_logger variable (i.e. `RUST_LOG={info,debug,trace}`)

## Benchmarks

### Just processing

- `jq -rn --stream 'fromstream(1|truncate_stream(inputs)) | select((.type == "item") and (.labels | has("en")) and (.claims.P31 // [] | map(.mainsnak.datavalue.value.id != "Q13442814") | all)) | (.id|ltrimstr("Q")) as $id | .labels["en"].value, (.aliases["en"] // [] | map(.value))[] | [$id, .] | @tsv' <file.json>`
- `jm <file.json> | jq 'select((.type == "item") and (.labels | has("en")) and (.claims.P31 // [] | map(.mainsnak.datavalue.value.id != "Q13442814") | all)) | (.id|ltrimstr("Q")) as $id | .labels["en"].value, (.aliases["en"] // [] | map(.value))[] | [$id, .] | @tsv'`
- `jstream -d 1 < `
- `jbzip2 -i <file.json> -j 'select((.type == "item") and (.labels | has("en")) and (.claims.P31 // [] | map(.mainsnak.datavalue.value.id != "Q13442814") | all)) | (.id|ltrimstr("Q")) as $id | .labels["en"].value, (.aliases["en"] // [] | map(.value))[] | [$id, .] | @tsv' -p $'[\n' -s $'\n]' -d $',\n' -b 50000000`

`hyperfine --parameter-list i 1,10,100,1000,10000,100000,1000000,2500000 -r 1 $'jq -rn --stream \'fromstream(1|truncate_stream(inputs)) | select((.type == "item") and (.labels | has("en")) and (.claims.P31 // [] | map(.mainsnak.datavalue.value.id != "Q13442814") | all)) | (.id|ltrimstr("Q")) as $id | .labels["en"].value, (.aliases["en"] // [] | map(.value))[] | [$id, .] | @tsv\' ~/dev/masters-thesis/data/dumps/{i}.json' $'jstream -d 1 < ~/dev/masters-thesis/data/dumps/{i}.json | jq \'select((.type == "item") and (.labels | has("en")) and (.claims.P31 // [] | map(.mainsnak.datavalue.value.id != "Q13442814") | all)) | (.id|ltrimstr("Q")) as $id | .labels["en"].value, (.aliases["en"] // [] | map(.value))[] | [$id, .] | @tsv\'' $'jm {i}.json | jq \'select((.type == "item") and (.labels | has("en")) and (.claims.P31 // [] | map(.mainsnak.datavalue.value.id != "Q13442814") | all)) | (.id|ltrimstr("Q")) as $id | .labels["en"].value, (.aliases["en"] // [] | map(.value))[] | [$id, .] | @tsv\''`

`hyperfine --export-json results.json --parameter-list i 1,10 -r 1 $'jq -rn --stream \'fromstream(1|truncate_stream(inputs)) | select((.type == "item") and (.labels | has("en")) and (.claims.P31 // [] | map(.mainsnak.datavalue.value.id != "Q13442814") | all)) | (.id|ltrimstr("Q")) as $id | .labels["en"].value, (.aliases["en"] // [] | map(.value))[] | [$id, .] | @tsv\' ~/dev/masters-thesis/data/dumps/{i}.json' $'jstream -d 1 < ~/dev/masters-thesis/data/dumps/{i}.json | jq \'select((.type == "item") and (.labels | has("en")) and (.claims.P31 // [] | map(.mainsnak.datavalue.value.id != "Q13442814") | all)) | (.id|ltrimstr("Q")) as $id | .labels["en"].value, (.aliases["en"] // [] | map(.value))[] | [$id, .] | @tsv\'' $'./jm ~/dev/masters-thesis/data/dumps/{i}.json | jq \'select((.type == "item") and (.labels | has("en")) and (.claims.P31 // [] | map(.mainsnak.datavalue.value.id != "Q13442814") | all)) | (.id|ltrimstr("Q")) as $id | .labels["en"].value, (.aliases["en"] // [] | map(.value))[] | [$id, .] | @tsv\''`

### Decompression and processing

- `bunzip2 --keep -c <file.bz2> | jstream -d 1 | jq \'select((.type == "item") and (.labels | has("en")) and (.claims.P31 // [] | map(.mainsnak.datavalue.value.id != "Q13442814") | all)) | (.id|ltrimstr("Q")) as $id | .labels["en"].value, (.aliases["en"] // [] | map(.value))[] | [$id, .] | @tsv\''`
- `jbzip2 -i <file.bz2> -j 'select((.type == "item") and (.labels | has("en")) and (.claims.P31 // [] | map(.mainsnak.datavalue.value.id != "Q13442814") | all)) | (.id|ltrimstr("Q")) as $id | .labels["en"].value, (.aliases["en"] // [] | map(.value))[] | [$id, .] | @tsv' -p $'[\n' -s $'\n]' -d $',\n' -b 5000000'`

`hyperfine --parameter-list i 1,10,100,1000,10000,100000,1000000,2500000 -r 1 $'bunzip2 --keep -c <file.bz2> | jstream -d 1 | jq \'select((.type == "item") and (.labels | has("en")) and (.claims.P31 // [] | map(.mainsnak.datavalue.value.id != "Q13442814") | all)) | (.id|ltrimstr("Q")) as $id | .labels["en"].value, (.aliases["en"] // [] | map(.value))[] | [$id, .] | @tsv\'' $'./dev/jbzip2/target/release/jbzip2 -i ./dev/masters-thesis/data/dumps/{i}.json.bz2 -j \'select((.type == "item") and (.labels | has("en")) and (.claims.P31 // [] | map(.mainsnak.datavalue.value.id != "Q13442814") | all)) | (.id|ltrimstr("Q")) as $id | .labels["en"].value, (.aliases["en"] // [] | map(.value))[] | [$id, .] | @tsv\' -p \'[\n\' -s \'\n]\' -d \',\n\' -b 5000000'`

