load test_helper

@test "log default shows commits" {
	run "$VIBE_BIN" -t log '{}'
	assert_success
	assert_output --partial "initial commit"
	assert_output --partial "Test User <test@test.com>"
}

@test "log with limit 1" {
	run "$VIBE_BIN" -t log '{"limit":1}'
	assert_success
	assert_output --partial "initial commit"
	refute_output --partial $'initial commit\ninitial commit'
}

@test "log with path filter" {
	run "$VIBE_BIN" -t log '{"path":"/src"}'
	assert_success
	assert_output --partial "initial commit"
}

@test "log with path not matching any commits" {
	run "$VIBE_BIN" -t log '{"path":"/nonexistent"}'
	assert_success
	assert_output "[No commits found]"
}

@test "log with invalid limit fails" {
	run "$VIBE_BIN" -t log '{"limit":101}'
	assert_failure
	assert_output --partial "limit must be between 1 and 100"
}
