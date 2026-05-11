load test_helper

@test "cache list when empty" {
	run "$VIBE_BIN" -t cache '{}'
	assert_success
	assert_output "[No cache files]"
}

@test "cache write then read" {
	run "$VIBE_BIN" -t cache '{"path":"test.md","content":"---\nkey: value\n---\nbody text"}'
	assert_success
	assert_output --partial "[Written test.md"

	run "$VIBE_BIN" -t cache '{"path":"test.md"}'
	assert_success
	assert_output --partial "body text"
	assert_output --partial "key: value"
}

@test "cache list shows files after write" {
	run "$VIBE_BIN" -t cache '{"path":"list_test.md","content":"---\nnum: 42\n---\nhello"}'
	assert_success

	run "$VIBE_BIN" -t cache '{}'
	assert_success
	assert_output --partial "list_test.md"
	assert_output --partial "num: 42"
}

@test "cache read non-existent file" {
	run "$VIBE_BIN" -t cache '{"path":"nonexistent.md"}'
	assert_success
	assert_output "[File not found]"
}

@test "cache write invalid filename fails" {
	run "$VIBE_BIN" -t cache '{"path":"../etc","content":"---\n---\nbody"}'
	assert_failure
	assert_output --partial "Invalid file name"
}

@test "cache write without front matter fails" {
	run "$VIBE_BIN" -t cache '{"path":"bad.md","content":"no front matter"}'
	assert_failure
	assert_output --partial "File must start with ---"
}

@test "cache content without path fails" {
	run "$VIBE_BIN" -t cache '{"content":"---\n---\nbody"}'
	assert_failure
	assert_output --partial "content requires path"
}

@test "cache write injects updated_at" {
	run "$VIBE_BIN" -t cache '{"path":"stamp.md","content":"---\ntag: a\n---\ncontent"}'
	assert_success

	run "$VIBE_BIN" -t cache '{"path":"stamp.md"}'
	assert_success
	assert_output --partial "updated_at:"
	assert_output --partial "tag: a"
	assert_output --partial "content"
}
