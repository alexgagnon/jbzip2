| Command | Mean [ms] | Min [ms] | Max [ms] | Relative |
|:---|---:|---:|---:|---:|
| `../../jbzip2-shell -i ../samples/data/1.json.bz2 -j 'select((.type == "item") and (.labels \| has("en")) and (.claims.P31 // [] \| map(.mainsnak.datavalue.value.id != "Q13442814") \| all)) \| (.id\|ltrimstr("Q")) as $id \| .labels["en"].value, (.aliases["en"] // [] \| map(.value))[] \| [., $id] \| @tsv' -p '[
' -s '
]' -d ',
' -b 1000000000` | 63.8 ± 4.9 | 58.8 | 69.5 | 1.00 |
| `bunzip2 --keep -c ../samples/data/1.json.bz2 \| ../../jstream/jstream -d 1 \| jq -r 'select((.type == "item") and (.labels \| has("en")) and (.claims.P31 // [] \| map(.mainsnak.datavalue.value.id != "Q13442814") \| all)) \| (.id\|ltrimstr("Q")) as $id \| .labels["en"].value, (.aliases["en"] // [] \| map(.value))[] \| [., $id] \| @tsv'` | 74.1 ± 5.0 | 69.9 | 82.8 | 1.16 ± 0.12 |
| `../../jbzip2-shell -i ../samples/data/10.json.bz2 -j 'select((.type == "item") and (.labels \| has("en")) and (.claims.P31 // [] \| map(.mainsnak.datavalue.value.id != "Q13442814") \| all)) \| (.id\|ltrimstr("Q")) as $id \| .labels["en"].value, (.aliases["en"] // [] \| map(.value))[] \| [., $id] \| @tsv' -p '[
' -s '
]' -d ',
' -b 1000000000` | 232.8 ± 7.1 | 221.8 | 240.9 | 3.65 ± 0.30 |
| `bunzip2 --keep -c ../samples/data/10.json.bz2 \| ../../jstream/jstream -d 1 \| jq -r 'select((.type == "item") and (.labels \| has("en")) and (.claims.P31 // [] \| map(.mainsnak.datavalue.value.id != "Q13442814") \| all)) \| (.id\|ltrimstr("Q")) as $id \| .labels["en"].value, (.aliases["en"] // [] \| map(.value))[] \| [., $id] \| @tsv'` | 156.2 ± 5.1 | 149.3 | 162.5 | 2.45 ± 0.21 |
| `../../jbzip2-shell -i ../samples/data/100.json.bz2 -j 'select((.type == "item") and (.labels \| has("en")) and (.claims.P31 // [] \| map(.mainsnak.datavalue.value.id != "Q13442814") \| all)) \| (.id\|ltrimstr("Q")) as $id \| .labels["en"].value, (.aliases["en"] // [] \| map(.value))[] \| [., $id] \| @tsv' -p '[
' -s '
]' -d ',
' -b 1000000000` | 1533.6 ± 12.6 | 1521.0 | 1547.4 | 24.03 ± 1.87 |
| `bunzip2 --keep -c ../samples/data/100.json.bz2 \| ../../jstream/jstream -d 1 \| jq -r 'select((.type == "item") and (.labels \| has("en")) and (.claims.P31 // [] \| map(.mainsnak.datavalue.value.id != "Q13442814") \| all)) \| (.id\|ltrimstr("Q")) as $id \| .labels["en"].value, (.aliases["en"] // [] \| map(.value))[] \| [., $id] \| @tsv'` | 908.3 ± 27.3 | 877.0 | 942.7 | 14.23 ± 1.18 |
| `../../jbzip2-shell -i ../samples/data/1_000.json.bz2 -j 'select((.type == "item") and (.labels \| has("en")) and (.claims.P31 // [] \| map(.mainsnak.datavalue.value.id != "Q13442814") \| all)) \| (.id\|ltrimstr("Q")) as $id \| .labels["en"].value, (.aliases["en"] // [] \| map(.value))[] \| [., $id] \| @tsv' -p '[
' -s '
]' -d ',
' -b 1000000000` | 5992.2 ± 102.5 | 5903.4 | 6152.8 | 93.89 ± 7.44 |
| `bunzip2 --keep -c ../samples/data/1_000.json.bz2 \| ../../jstream/jstream -d 1 \| jq -r 'select((.type == "item") and (.labels \| has("en")) and (.claims.P31 // [] \| map(.mainsnak.datavalue.value.id != "Q13442814") \| all)) \| (.id\|ltrimstr("Q")) as $id \| .labels["en"].value, (.aliases["en"] // [] \| map(.value))[] \| [., $id] \| @tsv'` | 3616.1 ± 120.2 | 3525.3 | 3822.1 | 56.66 ± 4.77 |
| `../../jbzip2-shell -i ../samples/data/10_000.json.bz2 -j 'select((.type == "item") and (.labels \| has("en")) and (.claims.P31 // [] \| map(.mainsnak.datavalue.value.id != "Q13442814") \| all)) \| (.id\|ltrimstr("Q")) as $id \| .labels["en"].value, (.aliases["en"] // [] \| map(.value))[] \| [., $id] \| @tsv' -p '[
' -s '
]' -d ',
' -b 1000000000` | 38999.8 ± 742.3 | 38325.5 | 40257.3 | 611.11 ± 48.68 |
| `bunzip2 --keep -c ../samples/data/10_000.json.bz2 \| ../../jstream/jstream -d 1 \| jq -r 'select((.type == "item") and (.labels \| has("en")) and (.claims.P31 // [] \| map(.mainsnak.datavalue.value.id != "Q13442814") \| all)) \| (.id\|ltrimstr("Q")) as $id \| .labels["en"].value, (.aliases["en"] // [] \| map(.value))[] \| [., $id] \| @tsv'` | 24502.4 ± 169.1 | 24298.4 | 24722.3 | 383.94 ± 29.82 |
| `../../jbzip2-shell -i ../samples/data/100_000.json.bz2 -j 'select((.type == "item") and (.labels \| has("en")) and (.claims.P31 // [] \| map(.mainsnak.datavalue.value.id != "Q13442814") \| all)) \| (.id\|ltrimstr("Q")) as $id \| .labels["en"].value, (.aliases["en"] // [] \| map(.value))[] \| [., $id] \| @tsv' -p '[
' -s '
]' -d ',
' -b 1000000000` | 294830.1 ± 19291.0 | 280325.4 | 326224.7 | 4619.82 ± 468.08 |
| `bunzip2 --keep -c ../samples/data/100_000.json.bz2 \| ../../jstream/jstream -d 1 \| jq -r 'select((.type == "item") and (.labels \| has("en")) and (.claims.P31 // [] \| map(.mainsnak.datavalue.value.id != "Q13442814") \| all)) \| (.id\|ltrimstr("Q")) as $id \| .labels["en"].value, (.aliases["en"] // [] \| map(.value))[] \| [., $id] \| @tsv'` | 148211.9 ± 4231.9 | 144471.7 | 155143.3 | 2322.39 ± 191.51 |
