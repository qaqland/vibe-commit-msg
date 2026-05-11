load test_helper

@test "list directory contents" {
	run "$VIBE_BIN" -t list '{"path":"/src"}'
	assert_success
	assert_output --partial "/src/alpha.txt"
	assert_output --partial "/src/beta.txt"
	assert_output --partial "/src/gamma.txt"
}

@test "list root directory" {
	run "$VIBE_BIN" -t list '{"path":"/"}'
	assert_success
	assert_output --partial "/README.md"
	assert_output --partial "/docs/"
	assert_output --partial "/src/"
}

@test "list file path fails" {
	run "$VIBE_BIN" -t list '{"path":"/src/alpha.txt"}'
	assert_failure
	assert_output --partial "read tool instead"
}

@test "list non-existent path fails" {
	run "$VIBE_BIN" -t list '{"path":"/nonexistent"}'
	assert_failure
	assert_output --partial "File not found"
}

@test "list path without leading slash fails" {
	run "$VIBE_BIN" -t list '{"path":"src"}'
	assert_failure
	assert_output --partial "Path must start with '/'"
}
