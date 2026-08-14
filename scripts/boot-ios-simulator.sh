#!/usr/bin/env bash
set -euo pipefail

# Try iPhone 16 first, fall back to iPhone 15, then iPhone 14, then any available iOS simulator
DEVICE_ID=$(xcrun simctl list devices available -j | python3 -c "
import json, sys
d = json.load(sys.stdin)['devices']
for name in ['iPhone 16', 'iPhone 15', 'iPhone 14']:
    try:
        print(next(x['udid'] for rt in d.values() for x in rt if x['name'] == name))
        break
    except StopIteration:
        continue
else:
    # Fallback: any iOS simulator
    try:
        print(next(x['udid'] for rt in d.values() for x in rt if 'iOS' in str(rt)))
    except StopIteration:
        sys.exit(1)
")

if [ -z "$DEVICE_ID" ]; then
    echo "No iOS simulator found"
    exit 1
fi

echo "DEVICE_ID=$DEVICE_ID" >> "$GITHUB_ENV"
xcrun simctl boot "$DEVICE_ID"