# Benchmarks - Processing only

FILTER='select((.type == "item") and (.labels | has("en")) and (.claims.P31 // [] | map(.mainsnak.datavalue.value.id != "Q13442814") | all)) | (.id|ltrimstr("Q")) as $id | .labels["en"].value\, (.aliases["en"] // [] | map(.value))[] | [.\, $id] | @tsv'

hyperfine --show-output --export-json json_only_results.json -L filter "$FILTER" -L i 1 -r 1 \
$'jq -r \'.[] | {filter}\' ../samples/data/{i}.json' 
# $'jq -rn --stream \'fromstream(1|truncate_stream(inputs)) | {filter}\' ../samples/data/{i}.json' \
# $'../../jstream/jstream -d 1 < ../samples/data/{i}.json | jq -r \'{filter}\'' \
# $'../../jm/jm ../samples/data/{i}.json | jq -r \'{filter}\''
