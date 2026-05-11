load test_helper

@test "grep existing pattern" {
	run "$VIBE_BIN" -t grep '{"pattern":"new line"}'
	assert_success
	assert_output --partial "/src/alpha.txt"
	assert_output --partial "new line"
}

@test "grep in specific path" {
	run "$VIBE_BIN" -t grep '{"pattern":"hello","path":"/docs"}'
	assert_success
	assert_output --partial "/docs/notes.txt"
	assert_output --partial "hello world"
}

@test "grep non-existent pattern" {
	run "$VIBE_BIN" -t grep '{"pattern":"nonexistent_pattern_xyzzy"}'
	assert_success
	assert_output "[No matches found]"
}

@test "grep empty pattern fails" {
	run "$VIBE_BIN" -t grep '{"pattern":""}'
	assert_failure
	assert_output --partial "pattern is required"
}
