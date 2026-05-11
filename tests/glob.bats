load test_helper

@test "glob txt files" {
	run "$VIBE_BIN" -t glob '{"pattern":"src/**/*.txt"}'
	assert_success
	assert_output --partial "/src/alpha.txt"
	assert_output --partial "/src/beta.txt"
	assert_output --partial "/src/gamma.txt"
}

@test "glob non-matching pattern" {
	run "$VIBE_BIN" -t glob '{"pattern":"*.py"}'
	assert_success
	assert_output "[No files found]"
}

@test "glob root level files" {
	run "$VIBE_BIN" -t glob '{"pattern":"*.md"}'
	assert_success
	assert_output "/README.md"
}

@test "glob empty pattern fails" {
	run "$VIBE_BIN" -t glob '{"pattern":""}'
	assert_failure
	assert_output --partial "pattern is required"
}
