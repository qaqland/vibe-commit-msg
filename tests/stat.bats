load test_helper

@test "stat with staged changes" {
	run "$VIBE_BIN" -t stat '{}'
	assert_success
	assert_output --partial "+1"
	assert_output --partial "/src/alpha.txt"
	assert_output --partial "/src/gamma.txt"
}

@test "stat output format" {
	run "$VIBE_BIN" -t stat '{}'
	assert_success
	assert_output --partial $'+1\t-0\t/src/alpha.txt'
}
