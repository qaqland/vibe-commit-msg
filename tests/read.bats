load test_helper

@test "read existing file" {
	run "$VIBE_BIN" -t read '{"path":"/simple.txt"}'
	assert_success
	assert_output --partial "1: alpha"
}

@test "read with offset" {
	run "$VIBE_BIN" -t read '{"path":"/simple.txt","offset":2}'
	assert_success
	assert_output --partial "2: beta"
}

@test "read with limit" {
	run "$VIBE_BIN" -t read '{"path":"/simple.txt","limit":1}'
	assert_success
	assert_output --partial "1: alpha"
}

@test "read with offset and limit" {
	run "$VIBE_BIN" -t read '{"path":"/simple.txt","offset":2,"limit":1}'
	assert_success
	assert_output --partial "2: beta"
}

@test "read directory fails" {
	run "$VIBE_BIN" -t read '{"path":"/src"}'
	assert_failure
	assert_output --partial "list"
}

@test "read path without leading slash fails" {
	run "$VIBE_BIN" -t read '{"path":"src/main.rs"}'
	assert_failure
	assert_output --partial "Path must start with '/'"
}

@test "read non-existent file fails" {
	run "$VIBE_BIN" -t read '{"path":"/nonexistent.rs"}'
	assert_failure
	assert_output --partial "File not found"
}

@test "read empty file" {
	run "$VIBE_BIN" -t read '{"path":"/empty.txt"}'
	assert_success
	assert_output "[Empty file]"
}
