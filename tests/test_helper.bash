bats_load_library bats-support
bats_load_library bats-assert

setup() {
	VIBE_BIN="${VIBE_BIN:?VIBE_BIN must be set}"

	cd "$BATS_TEST_TMPDIR"

	git init
	git config user.email "test@test.com"
	git config user.name "Test User"

	mkdir -p src docs

	cat >src/alpha.txt <<'FI'
old line
FI

	cat >src/beta.txt <<'FI'
unchanged
FI

	cat >docs/notes.txt <<'FI'
hello world
line two
FI

	echo "# test project" >README.md

	git add -A
	git commit -m "initial commit"

	cat >src/alpha.txt <<'FI'
old line
new line
FI

	cat >src/gamma.txt <<'FI'
new file
FI

	git add src/alpha.txt src/gamma.txt

	cat >simple.txt <<'FI'
alpha
beta
gamma
FI
	git add simple.txt

	touch empty.txt
	git add empty.txt
}
