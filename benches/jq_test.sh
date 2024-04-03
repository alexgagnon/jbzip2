# Benchmarks - Processing only

FILTER='select((.type == "item") and (.labels | has("en")) and (.claims.P31 // [] | map(.mainsnak.datavalue.value.id != "Q13442814") | all)) | (.id|ltrimstr("Q")) as $id | .labels["en"].value\, (.aliases["en"] // [] | map(.value))[] | [.\, $id] | @tsv'

for i in 1_000_000; do
  hyperfine --show-output -i -u second -L filter "$FILTER" -L i $i -r 5 \
  --export-markdown results_$i.md \
  --command-name jstream \
  --command-name 'jq-jsonl' \
  $'../../jstream/jstream -d 1 < ../samples/data/{i}.json | jq -r \'{filter}\'' \
  $'jq -r \'{filter}\' < ../samples/data/{i}.jsonl'
  # --command-name jq \
  # --command-name jm \
  # $'jq -r \'.[] | {filter}\' ../samples/data/{i}.json' \
  # $'../../jm/jm ../samples/data/{i}.json | jq -r \'{filter}\'' \
  # --command-name 'jq-stream' \
  # $'jq -rn --stream \'fromstream(1|truncate_stream(inputs)) | {filter}\' ../samples/data/{i}.json' \
done