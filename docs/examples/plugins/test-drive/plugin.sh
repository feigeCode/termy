#!/bin/sh
read line
printf '%s\n' '{"type":"hello","payload":{"protocol_version":1,"plugin_id":"example.test-drive","name":"Test Drive Plugin","version":"0.1.0","capabilities":["command_provider"]}}'
printf '%s\n' '{"type":"log","payload":{"level":"info","message":"test-drive plugin started"}}'
printf '%s\n' '{"type":"toast","payload":{"level":"success","message":"test-drive plugin connected","duration_ms":1800}}'
while read line; do
  if [ "$line" = '{"type":"ping"}' ]; then
    printf '%s\n' '{"type":"log","payload":{"level":"debug","message":"received host ping"}}'
    printf '%s\n' '{"type":"pong"}'
  fi
  if [ "$line" = '{"type":"invoke_command","payload":{"command_id":"example.test-drive.ping"}}' ]; then
    printf '%s\n' '{"type":"log","payload":{"level":"info","message":"test-drive command invoked"}}'
    printf '%s\n' '{"type":"toast","payload":{"level":"info","message":"test-drive command fired","duration_ms":1400}}'
  fi
  if [ "$line" = '{"type":"shutdown"}' ]; then
    printf '%s\n' '{"type":"log","payload":{"level":"info","message":"test-drive plugin shutting down"}}'
    exit 0
  fi
done
