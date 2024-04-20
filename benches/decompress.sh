#!/bin/bash

FILTER='select((.type == "item") and (.labels | has("en")) and (.claims.P31 // [] | map(.mainsnak.datavalue.value.id != "Q13442814") | all)) | (.id|ltrimstr("Q")) as $id | .labels["en"].value, (.aliases["en"] // [] | map(.value))[] | [., $id] | @tsv'
FILE='../samples/data/latest-all.json.bz2'
MULTIPLIER=1
RUNS=2

for ((j=1; j<=RUNS; j++))
do
  for i in 15 30 45 60 75 90 105 120
  do
    TIME_IN_MINUTES=$(($i * MULTIPLIER))
    TIME=$(($TIME_IN_MINUTES * 60))
    echo "Running for $TIME_IN_MINUTES minutes... Iteration $j"
    
    echo "jbzip2 - 0.1Gb"
    timeout $TIME ../target/release/jbzip2 -i $FILE -j ".[] | $FILTER" -t wikidump -b 100000000 -f -o ./tmp.jbzip2.2.$TIME_IN_MINUTES.tsv
    wc -l ./tmp.jbzip2.2.$TIME_IN_MINUTES.tsv >> ./tmp.jbzip2.tsv

    echo "jbzip2 - 1Gb"
    timeout $TIME ../target/release/jbzip2 -i $FILE -j ".[] | $FILTER" -t wikidump -b 1000000000 -f -o ./tmp.jbzip2.2.$TIME_IN_MINUTES.tsv
    wc -l ./tmp.jbzip2.2.$TIME_IN_MINUTES.tsv >> ./tmp.jbzip2.tsv

    echo "jstream"
    timeout $TIME lbunzip2 --keep -c $FILE | ../../jstream/jstream -d 1 | jq -r "$FILTER" > ./tmp.jstream.$TIME_IN_MINUTES.tsv
    wc -l ./tmp.jstream.$TIME_IN_MINUTES.tsv >> ./tmp.jstream.tsv

    echo "jq"
    timeout $TIME lbunzip2 --keep -c $FILE | jq -rn --stream "fromstream(1|truncate_stream(inputs)) | $FILTER" > ./tmp.jq-stream.$TIME_IN_MINUTES.tsv
    wc -l ./tmp.jq-stream.$TIME_IN_MINUTES.tsv >> ./tmp.jq-stream.tsv
  done
done
