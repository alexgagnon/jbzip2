# Benchmarks for decompressing and filtering JSON files

FILTER='select((.type == "item") and (.labels | has("en")) and (.claims.P31 // [] | map(.mainsnak.datavalue.value.id != "Q13442814") | all)) | (.id|ltrimstr("Q")) as $id | .labels["en"].value\, (.aliases["en"] // [] | map(.value))[] | [.\, $id] | @tsv'
FILE=''

for i in 1 10 100 1_000 10_000 100_000 1_000_000; do
  hyperfine --show-output -i -u second -L file "$FILE" -L filter "$FILTER" -L i $i -r 5 \
  --export-markdown decompress_results_2_$i.md \
  --command-name jbzip2 \
  --command-name 'jbzip2 b 1000000000' \
  --command-name 'bunzip | jstream' \
  --command-name 'lbzip2 | jstream' \
  $'../target/release/jbzip2 -i {file} -j \'{filter}\' -t wikidump -b 10000000' \
  $'../target/release/jbzip2 -i {file} -j \'{filter}\' -t wikidump -b 100000000' \
  $'bunzip2 --keep -c {file} | ../../jstream/jstream -d 1 | jq -r \'{filter}\'' \
  $'lbunzip2 --keep -c {file} | ../../jstream/jstream -d 1 | jq -r \'{filter}\''
done