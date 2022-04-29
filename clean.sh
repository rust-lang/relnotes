#!/bin/bash

if [[ "$OSTYPE" == "darwin"* ]]; then
    inplace=(-i '')
else
    inplace=-i
fi

rg '\[([^]]*)\]: ' -o release.md > defs
rg '\[([^]]*)\]$' -o release.md > used

for word in `cat defs`; do
    pattern=$(echo "$word" | sed 's@\[@\\[@g' | sed 's@]@\\]@g' | sed 's@/@\\/@g')
    in_used=$(echo "$word" | sed 's@:.*@@')
    if ! fgrep -q "$in_used" used; then
        sed $inplace -e "/^$pattern/d" release.md
    fi
done

rm defs used
