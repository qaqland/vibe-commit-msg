load test_helper

@test "diff all staged changes" {
	run "$VIBE_BIN" -t diff '{}'
	assert_success
	assert_output --partial "diff --git"
	assert_output --partial "src/alpha.txt"
	assert_output --partial "src/gamma.txt"
	assert_output --partial "+++"
	assert_output --partial "---"
	assert_output --partial "+new line"
	assert_output --partial "+new file"
}

@test "diff specific path" {
	run "$VIBE_BIN" -t diff '{"path":"/src/alpha.txt"}'
	assert_success
	assert_output --partial "diff --git"
	assert_output --partial "src/alpha.txt"
	refute_output --partial "gamma"
}

@test "diff unchanged file" {
	run "$VIBE_BIN" -t diff '{"path":"/src/beta.txt"}'
	assert_success
	assert_output "[No changes found]"
}

@test "diff path without leading slash fails" {
	run "$VIBE_BIN" -t diff '{"path":"src/alpha.txt"}'
	assert_failure
	assert_output --partial "path must start with '/'"
}
