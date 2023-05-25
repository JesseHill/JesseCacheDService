# Some tests for KV latency.

You can run:
curl -H 'Host:terribly-proven-shiner.edgecompute.app' 'https://stp92.fastly.host/lookup-test?iterations=2000' | jq '.[].lookupTimeTaken.nanos' | st --summary

To see latency numbers using the low-level API directly for lookups.

Run:
curl -H 'Host:terribly-proven-shiner.edgecompute.app' 'https://stp92.fastly.host/kv-lookup-test?iterations=2000' | jq '.[].lookupTimeTaken.nanos' | st --summary

To see a similar test using the KV store API instead.
