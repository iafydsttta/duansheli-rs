#!/bin/bash

set -euxo pipefail

cd fixtures/

# Duration to run (15 minutes in seconds)
DURATION=$((15 * 60))

# Interval between file creation (3 minutes in seconds)
INTERVAL=$((3 * 60))

# Calculate end time
END_TIME=$(($(date +%s) + DURATION))

echo "Starting file creation loop for 15 minutes..."
echo "Creating files every 3 minutes"

# Loop until end time is reached
while [ $(date +%s) -lt $END_TIME ]; do
    # Create filename with ISO 8601 timestamp
    FILENAME=$(date +%Y-%m-%dT%H:%M:%S%z)
    
    # Create the file
    touch "$FILENAME"
    
    echo "Created: $FILENAME"
    
    # Sleep for 3 minutes (unless we're at the end)
    REMAINING=$((END_TIME - $(date +%s)))
    if [ $REMAINING -gt 0 ]; then
        SLEEP_TIME=$((REMAINING < INTERVAL ? REMAINING : INTERVAL))
        sleep $SLEEP_TIME
    fi
done

echo "Loop completed after 15 minutes"