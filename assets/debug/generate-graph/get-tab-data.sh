#!/bin/sh

url="http://127.0.0.1:60001"

mkdir -p tab_data

for i in $(seq 0 199); do
    file_name="tab_data/data-$i.json"
    echo "$url > $file_name"
    curl -s "$url" -o $file_name

    if [ $? -ne 0 ]; then
        echo "Failed to get data from $url, quitting."
        exit 1
    fi

    sleep 1
done

