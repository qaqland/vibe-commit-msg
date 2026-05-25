load test_helper

@test "progress requires in_progress before completed" {
	run "$VIBE_BIN" -t progress '{"step":"Step 1 — Load context","status":"completed","note":"cache loaded"}'
	assert_failure
	assert_output --partial "must be marked in_progress before completed"
}

@test "progress requires note for completed" {
	run "$VIBE_BIN" -t progress '{"step":"Step 1 — Load context","status":"completed"}'
	assert_failure
	assert_output --partial "note is required"
}

@test "progress returns minimal summary" {
	run "$VIBE_BIN" -t progress '{"step":"Step 1 — Load context","status":"in_progress"}'
	assert_success
	assert_output --partial "Step 1 — Load context"
	assert_output --partial "in_progress"
}

@test "progress returns minimal summary with done count" {
	run "$VIBE_BIN" -t progress '{"step":"Step 1 — Load context","status":"in_progress"}'
	assert_success
	assert_output --partial "Step 1 — Load context"
	assert_output --partial "in_progress"
	assert_output --partial "0/7 done"
}
