# Benchmarks for decompressing and filtering JSON files

FILTER='select((.type == "item") and (.labels | has("en")) and (.claims.P31 // [] | map(.mainsnak.datavalue.value.id != "Q13442814") | all)) | (.id|ltrimstr("Q")) as $id | .labels["en"].value\, (.aliases["en"] // [] | map(.value))[] | [.\, $id] | @tsv'

for i in 1 10 100 1_000 10_000 100_000; do
  hyperfine --show-output -i -u second -L filter "$FILTER" -L i $i -r 5 \
  --export-markdown decompress_and_json_results_$i.md \
  --command-name jbzip2 \
  --command-name jstream \
  -L b 1000000000 \
  $'../target/release/jbzip2 -i ../samples/data/{i}.json.bz2 -j \'{filter}\' -t wikidump \
  $'bunzip2 --keep -c ../samples/data/{i}.json.bz2 | ../../jstream/jstream -d 1 | jq -r \'{filter}\'' \
  $'lbunzip2 --keep -c ../samples/data/{i}.json.bz2 | ../../jstream/jstream -d 1 | jq -r \'{filter}\'' \
done