# Benchmarks - Processing only

FILTER='select((.type == "item") and (.labels | has("en")) and (.claims.P31 // [] | map(.mainsnak.datavalue.value.id != "Q13442814") | all)) | (.id|ltrimstr("Q")) as $id | .labels["en"].value\, (.aliases["en"] // [] | map(.value))[] | [.\, $id] | @tsv'

for i in 10_000 100_000 1_000_000; do
  hyperfine --show-output -i -u second -L filter "$FILTER" -L i $i -r 5 \
  --command-name jq \
  --export-markdown jsonl_only_results_8gb_$i.md \
  $'jq -r \'{filter}\' < ../samples/data/{i}.jsonl'
done
