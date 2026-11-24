#!/usr/bin/env bash
# Runs a command until it succeeds, at most five times, waiting a little
# longer after each failure. The release workflow wraps every step that
# downloads with it: a dropped connection to a mirror should not fail a
# release that builds.
#
#   bash packaging/retry.sh rustup toolchain install

set -u

attempts=5
for attempt in $(seq 1 "$attempts"); do
  "$@" && exit 0
  status=$?
  if [ "$attempt" -eq "$attempts" ]; then
    echo "::error::$* failed $attempts times"
    exit "$status"
  fi
  echo "::warning::$* failed (attempt $attempt of $attempts, exit $status): trying again"
  sleep $((attempt * 15))
done
