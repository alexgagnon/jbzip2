# Benchmarks for decompressing and filtering JSON files

FILTER='select((.type == "item") and (.labels | has("en")) and (.claims.P31 // [] | map(.mainsnak.datavalue.value.id != "Q13442814") | all)) | (.id|ltrimstr("Q")) as $id | .labels["en"].value\, (.aliases["en"] // [] | map(.value))[] | [.\, $id] | @tsv'

hyperfine --show-output --export-markdown decompress_and_json_results.md -L filter "$FILTER" -L i 1 -L b 1000000000 -r 5 \
$'../../jbzip2-shell -i ../samples/data/{i}.json.bz2 -j \'{filter}\' -p \'[\n\' -s \'\n]\' -d \',\n\' -b {b}' \
$'../../jbzip2-linked -i ../samples/data/{i}.json.bz2 -j \'{filter}\' -p \'[\n\' -s \'\n]\' -d \',\n\' -b {b}' \
# $'bunzip2 --keep -c ../samples/data/{i}.json.bz2 | ../../jstream/jstream -d 1 | jq -r \'{filter}\'' \
# $'lbunzip2 --keep -c ../samples/data/{i}.json.lbz2 | jstream -d 1 | jq -r \'{filter}\'' \